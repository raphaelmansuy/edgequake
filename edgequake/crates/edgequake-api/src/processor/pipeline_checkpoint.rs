//! KG + Embedding pipeline checkpoint system.
//!
//! ## WHY
//!
//! The KG + Embedding pipeline (text_insert) is the most expensive processing
//! stage: LLM entity extraction can take minutes for large documents. If the
//! server crashes mid-extraction, all that work is lost and must be repeated.
//!
//! This module saves the expensive `ProcessingResult` (chunks, extractions,
//! embeddings, lineage) to KV storage after the LLM extraction stage completes.
//! On restart the checkpoint is loaded, skipping extraction entirely.
//!
//! ## Design
//!
//! ```text
//!   ┌──────────────────────────────────────────────────────────┐
//!   │  text_insert pipeline                                    │
//!   │                                                          │
//!   │  1. metadata setup                                       │
//!   │  2. process_with_resilience()  ← EXPENSIVE (LLM calls)  │
//!   │     ├─ checkpoint saved after success ←─── SAVE POINT    │
//!   │  3. store chunks in KV         ← IDEMPOTENT (upserts)   │
//!   │  4. store embeddings in vector ← IDEMPOTENT              │
//!   │  5. store entities in graph    ← IDEMPOTENT              │
//!   │  6. store edges in graph       ← IDEMPOTENT              │
//!   │  7. clear checkpoint           ← CLEANUP                 │
//!   └──────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Edge Cases
//!
//! - **Corrupt checkpoint**: Deserialization fails → treat as no checkpoint,
//!   reprocess from scratch.
//! - **Settings changed**: Different workspace provider or model → stale
//!   checkpoint returns wrong embeddings. Mitigated by including workspace_id
//!   and provider info in the checkpoint key.
//! - **Concurrent access**: The task system guarantees single-worker
//!   processing per document, so no lock contention.
//! - **Storage pressure**: Checkpoints can be large (MB scale for 500-chunk
//!   docs). Cleaned up on success; orphan cleanup runs on startup.
//!
//! ## Implements
//!
//! - FEAT-CHECKPOINT-KG: KG+Embedding pipeline checkpointing
//! - UC-RESUME-KG: System resumes KG pipeline after server restart

use std::sync::Arc;

use edgequake_pipeline::ProcessingResult;
use edgequake_storage::traits::KVStorage;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Checkpoint key prefix for pipeline processing results.
const CHECKPOINT_PREFIX: &str = "pipeline-checkpoint";

/// Maximum age of a checkpoint in seconds before it's considered stale.
/// Default: 24 hours. Checkpoints older than this are cleaned up on startup.
const CHECKPOINT_MAX_AGE_SECS: u64 = 86_400;

/// Wrapper around `ProcessingResult` with metadata for checkpoint validation.
///
/// WHY: We need to verify that the checkpoint matches the current processing
/// context (same workspace, same providers) before reusing it. A stale
/// checkpoint from a different workspace or provider would produce incorrect
/// results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineCheckpoint {
    /// The full processing result from `process_with_resilience`.
    pub result: ProcessingResult,

    /// Workspace ID the checkpoint was created for.
    pub workspace_id: String,

    /// LLM provider used for extraction (for staleness detection).
    pub extraction_provider: String,

    /// Embedding provider used (for staleness detection).
    pub embedding_provider: String,

    /// Unix timestamp when the checkpoint was created.
    pub created_at_epoch: u64,

    /// Content hash (first 64 bytes of source text SHA-256) for integrity.
    pub content_hash: String,
}

impl PipelineCheckpoint {
    /// Compute a short content hash for integrity checking.
    fn compute_content_hash(text: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        // Hash prefix to avoid hashing multi-MB documents entirely.
        let target_len = text.len().min(65_536);
        let safe_len = text.floor_char_boundary(target_len);
        let prefix = &text[..safe_len];
        hasher.update(prefix.as_bytes());
        hex::encode(&hasher.finalize()[..8]) // 16-char hex = 64-bit fingerprint
    }
}

/// Build the KV storage key for a document's pipeline checkpoint.
fn checkpoint_key(document_id: &str) -> String {
    format!("{}-{}", document_id, CHECKPOINT_PREFIX)
}

/// Save a pipeline checkpoint to KV storage after extraction completes.
///
/// # Arguments
/// * `kv` — KV storage instance
/// * `document_id` — Document being processed
/// * `result` — The expensive `ProcessingResult` to checkpoint
/// * `workspace_id` — Current workspace
/// * `extraction_provider` — LLM provider used for extraction
/// * `embedding_provider` — Embedding provider used
/// * `source_text` — Original document text (for content hash)
#[allow(clippy::too_many_arguments)]
pub async fn save_pipeline_checkpoint(
    kv: &Arc<dyn KVStorage>,
    document_id: &str,
    result: &ProcessingResult,
    workspace_id: &str,
    extraction_provider: &str,
    embedding_provider: &str,
    source_text: &str,
) -> Result<(), String> {
    let checkpoint = PipelineCheckpoint {
        result: result.clone(),
        workspace_id: workspace_id.to_string(),
        extraction_provider: extraction_provider.to_string(),
        embedding_provider: embedding_provider.to_string(),
        created_at_epoch: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        content_hash: PipelineCheckpoint::compute_content_hash(source_text),
    };

    let key = checkpoint_key(document_id);
    let value = serde_json::to_value(&checkpoint)
        .map_err(|e| format!("Failed to serialize pipeline checkpoint for {document_id}: {e}"))?;

    kv.upsert(&[(key.clone(), value)])
        .await
        .map_err(|e| format!("Failed to save pipeline checkpoint {key}: {e}"))?;

    info!(
        document_id = %document_id,
        chunks = result.chunks.len(),
        entities = result.stats.entity_count,
        relationships = result.stats.relationship_count,
        "Saved pipeline checkpoint (extraction result persisted for resume)"
    );

    Ok(())
}

/// Attempt to load a pipeline checkpoint from KV storage.
///
/// Returns `Some(ProcessingResult)` only if:
/// 1. A checkpoint exists for this document
/// 2. The workspace ID matches
/// 3. The extraction + embedding providers match (not stale)
/// 4. The content hash matches (source text hasn't changed)
/// 5. The checkpoint is not older than `CHECKPOINT_MAX_AGE_SECS`
///
/// Any validation failure logs a warning and returns `None`.
pub async fn load_pipeline_checkpoint(
    kv: &Arc<dyn KVStorage>,
    document_id: &str,
    workspace_id: &str,
    extraction_provider: &str,
    embedding_provider: &str,
    source_text: &str,
) -> Option<ProcessingResult> {
    let key = checkpoint_key(document_id);

    let value = match kv.get_by_id(&key).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            debug!(document_id = %document_id, "No pipeline checkpoint found");
            return None;
        }
        Err(e) => {
            warn!(
                document_id = %document_id,
                error = %e,
                "Failed to read pipeline checkpoint — reprocessing from scratch"
            );
            return None;
        }
    };

    let checkpoint: PipelineCheckpoint = match serde_json::from_value(value) {
        Ok(cp) => cp,
        Err(e) => {
            warn!(
                document_id = %document_id,
                error = %e,
                "Corrupt pipeline checkpoint (deserialization failed) — clearing and reprocessing"
            );
            // Best-effort cleanup of corrupt checkpoint
            let _ = kv.delete(&[key]).await;
            return None;
        }
    };

    // Validate workspace match
    if checkpoint.workspace_id != workspace_id {
        info!(
            document_id = %document_id,
            checkpoint_workspace = %checkpoint.workspace_id,
            current_workspace = %workspace_id,
            "Pipeline checkpoint workspace mismatch — reprocessing"
        );
        let _ = kv.delete(&[key]).await;
        return None;
    }

    // Validate provider match (prevents stale embeddings from wrong model)
    if checkpoint.extraction_provider != extraction_provider
        || checkpoint.embedding_provider != embedding_provider
    {
        info!(
            document_id = %document_id,
            checkpoint_extraction = %checkpoint.extraction_provider,
            current_extraction = %extraction_provider,
            checkpoint_embedding = %checkpoint.embedding_provider,
            current_embedding = %embedding_provider,
            "Pipeline checkpoint provider mismatch — reprocessing with current providers"
        );
        let _ = kv.delete(&[key]).await;
        return None;
    }

    // Validate content hash (prevents using checkpoint for changed content)
    let current_hash = PipelineCheckpoint::compute_content_hash(source_text);
    if checkpoint.content_hash != current_hash {
        info!(
            document_id = %document_id,
            "Pipeline checkpoint content hash mismatch — source text changed, reprocessing"
        );
        let _ = kv.delete(&[key]).await;
        return None;
    }

    // Validate age
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let age = now.saturating_sub(checkpoint.created_at_epoch);
    if age > CHECKPOINT_MAX_AGE_SECS {
        info!(
            document_id = %document_id,
            age_hours = age / 3600,
            max_age_hours = CHECKPOINT_MAX_AGE_SECS / 3600,
            "Pipeline checkpoint too old — reprocessing"
        );
        let _ = kv.delete(&[key]).await;
        return None;
    }

    info!(
        document_id = %document_id,
        chunks = checkpoint.result.chunks.len(),
        entities = checkpoint.result.stats.entity_count,
        age_secs = age,
        "Resuming from pipeline checkpoint — skipping LLM extraction"
    );

    Some(checkpoint.result)
}

/// Clear a pipeline checkpoint after successful processing.
///
/// Called when all storage stages complete successfully, freeing KV space.
pub async fn clear_pipeline_checkpoint(kv: &Arc<dyn KVStorage>, document_id: &str) {
    let key = checkpoint_key(document_id);
    match kv.delete(std::slice::from_ref(&key)).await {
        Ok(_) => debug!(document_id = %document_id, "Cleared pipeline checkpoint"),
        Err(e) => warn!(
            document_id = %document_id,
            error = %e,
            "Failed to clear pipeline checkpoint (non-fatal)"
        ),
    }
}

/// Clean up stale/orphaned pipeline checkpoints on server startup.
///
/// Scans KV storage for checkpoint keys older than `CHECKPOINT_MAX_AGE_SECS`
/// and removes them. This prevents unbounded storage growth from crashed
/// processing runs that never completed.
pub async fn cleanup_stale_checkpoints(kv: &Arc<dyn KVStorage>) {
    let all_keys = match kv.keys().await {
        Ok(keys) => keys,
        Err(e) => {
            warn!(error = %e, "Failed to list KV keys for checkpoint cleanup");
            return;
        }
    };

    let checkpoint_keys: Vec<String> = all_keys
        .into_iter()
        .filter(|k| k.ends_with(&format!("-{}", CHECKPOINT_PREFIX)))
        .collect();

    if checkpoint_keys.is_empty() {
        return;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut cleaned = 0u32;
    for key in &checkpoint_keys {
        if let Ok(Some(value)) = kv.get_by_id(key).await {
            if let Ok(cp) = serde_json::from_value::<PipelineCheckpoint>(value) {
                let age = now.saturating_sub(cp.created_at_epoch);
                if age > CHECKPOINT_MAX_AGE_SECS {
                    let _ = kv.delete(std::slice::from_ref(key)).await;
                    cleaned += 1;
                }
            } else {
                // Corrupt checkpoint — remove it
                let _ = kv.delete(std::slice::from_ref(key)).await;
                cleaned += 1;
            }
        }
    }

    if cleaned > 0 {
        info!(
            total_checkpoints = checkpoint_keys.len(),
            cleaned = cleaned,
            "Cleaned up stale pipeline checkpoints on startup"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_key_format() {
        assert_eq!(checkpoint_key("doc-123"), "doc-123-pipeline-checkpoint");
    }

    #[test]
    fn test_content_hash_deterministic() {
        let hash1 = PipelineCheckpoint::compute_content_hash("hello world");
        let hash2 = PipelineCheckpoint::compute_content_hash("hello world");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 16); // 8 bytes = 16 hex chars
    }

    #[test]
    fn test_content_hash_differs_for_different_content() {
        let hash1 = PipelineCheckpoint::compute_content_hash("document A content");
        let hash2 = PipelineCheckpoint::compute_content_hash("document B content");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_content_hash_long_text_uses_prefix() {
        // Hashes only first 64KB — two texts differing after 64KB produce same hash
        let base = "x".repeat(65_536);
        let text1 = format!("{}AAA", base);
        let text2 = format!("{}BBB", base);
        // Both should hash the same 64KB prefix
        let hash1 = PipelineCheckpoint::compute_content_hash(&text1);
        let hash2 = PipelineCheckpoint::compute_content_hash(&text2);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_pipeline_checkpoint_serialization_roundtrip() {
        use edgequake_pipeline::{ProcessingResult, ProcessingStats};

        let result = ProcessingResult {
            document_id: "test-doc".to_string(),
            chunks: vec![],
            extractions: vec![],
            stats: ProcessingStats::default(),
            lineage: None,
        };

        let checkpoint = PipelineCheckpoint {
            result,
            workspace_id: "ws-1".to_string(),
            extraction_provider: "openai".to_string(),
            embedding_provider: "ollama".to_string(),
            created_at_epoch: 1_700_000_000,
            content_hash: "abcdef0123456789".to_string(),
        };

        let json = serde_json::to_value(&checkpoint).unwrap();
        let restored: PipelineCheckpoint = serde_json::from_value(json).unwrap();

        assert_eq!(restored.workspace_id, "ws-1");
        assert_eq!(restored.extraction_provider, "openai");
        assert_eq!(restored.result.document_id, "test-doc");
    }

    #[tokio::test]
    async fn test_save_and_load_checkpoint() {
        use edgequake_pipeline::{ProcessingResult, ProcessingStats};
        use edgequake_storage::MemoryKVStorage;

        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));

        let result = ProcessingResult {
            document_id: "doc-42".to_string(),
            chunks: vec![],
            extractions: vec![],
            stats: ProcessingStats {
                entity_count: 5,
                relationship_count: 3,
                ..Default::default()
            },
            lineage: None,
        };

        // Save checkpoint
        save_pipeline_checkpoint(
            &kv,
            "doc-42",
            &result,
            "workspace-A",
            "openai",
            "ollama",
            "Some document text for testing",
        )
        .await
        .unwrap();

        // Load checkpoint — should succeed
        let loaded = load_pipeline_checkpoint(
            &kv,
            "doc-42",
            "workspace-A",
            "openai",
            "ollama",
            "Some document text for testing",
        )
        .await;

        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.document_id, "doc-42");
        assert_eq!(loaded.stats.entity_count, 5);
    }

    #[tokio::test]
    async fn test_load_checkpoint_workspace_mismatch() {
        use edgequake_pipeline::{ProcessingResult, ProcessingStats};
        use edgequake_storage::MemoryKVStorage;

        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));

        let result = ProcessingResult {
            document_id: "doc-1".to_string(),
            chunks: vec![],
            extractions: vec![],
            stats: ProcessingStats::default(),
            lineage: None,
        };

        save_pipeline_checkpoint(&kv, "doc-1", &result, "ws-A", "openai", "ollama", "text")
            .await
            .unwrap();

        // Load with different workspace — should return None
        let loaded =
            load_pipeline_checkpoint(&kv, "doc-1", "ws-B", "openai", "ollama", "text").await;
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_load_checkpoint_provider_mismatch() {
        use edgequake_pipeline::{ProcessingResult, ProcessingStats};
        use edgequake_storage::MemoryKVStorage;

        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));

        let result = ProcessingResult {
            document_id: "doc-2".to_string(),
            chunks: vec![],
            extractions: vec![],
            stats: ProcessingStats::default(),
            lineage: None,
        };

        save_pipeline_checkpoint(&kv, "doc-2", &result, "ws", "openai", "ollama", "text")
            .await
            .unwrap();

        // Load with different provider — should return None
        let loaded =
            load_pipeline_checkpoint(&kv, "doc-2", "ws", "anthropic", "ollama", "text").await;
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_load_checkpoint_content_changed() {
        use edgequake_pipeline::{ProcessingResult, ProcessingStats};
        use edgequake_storage::MemoryKVStorage;

        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));

        let result = ProcessingResult {
            document_id: "doc-3".to_string(),
            chunks: vec![],
            extractions: vec![],
            stats: ProcessingStats::default(),
            lineage: None,
        };

        save_pipeline_checkpoint(
            &kv,
            "doc-3",
            &result,
            "ws",
            "openai",
            "ollama",
            "original text",
        )
        .await
        .unwrap();

        // Load with different content — should return None
        let loaded =
            load_pipeline_checkpoint(&kv, "doc-3", "ws", "openai", "ollama", "modified text").await;
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_clear_checkpoint() {
        use edgequake_pipeline::{ProcessingResult, ProcessingStats};
        use edgequake_storage::MemoryKVStorage;

        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));

        let result = ProcessingResult {
            document_id: "doc-4".to_string(),
            chunks: vec![],
            extractions: vec![],
            stats: ProcessingStats::default(),
            lineage: None,
        };

        save_pipeline_checkpoint(&kv, "doc-4", &result, "ws", "openai", "ollama", "text")
            .await
            .unwrap();

        // Verify it exists
        let loaded = load_pipeline_checkpoint(&kv, "doc-4", "ws", "openai", "ollama", "text").await;
        assert!(loaded.is_some());

        // Clear it
        clear_pipeline_checkpoint(&kv, "doc-4").await;

        // Verify it's gone
        let loaded = load_pipeline_checkpoint(&kv, "doc-4", "ws", "openai", "ollama", "text").await;
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_corrupt_checkpoint_returns_none() {
        use edgequake_storage::MemoryKVStorage;

        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));

        // Manually insert corrupt checkpoint
        let key = checkpoint_key("doc-corrupt");
        kv.upsert(&[(key, serde_json::json!({"invalid": true}))])
            .await
            .unwrap();

        // Load should return None and clean up corrupt entry
        let loaded =
            load_pipeline_checkpoint(&kv, "doc-corrupt", "ws", "openai", "ollama", "text").await;
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_no_checkpoint_returns_none() {
        use edgequake_storage::MemoryKVStorage;

        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));

        let loaded =
            load_pipeline_checkpoint(&kv, "nonexistent-doc", "ws", "openai", "ollama", "text")
                .await;
        assert!(loaded.is_none());
    }
}
