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
//!   │  2. process_with_resilience()  ← EXPENSIVE (LLM calls)   │
//!   │     ├─ checkpoint saved after success ←─── SAVE POINT    │
//!   │  3. store chunks in KV         ← IDEMPOTENT (upserts)    │
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
use edgequake_storage::contracts::CheckpointArtifactStore;
use edgequake_storage::traits::KVStorage;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Suffix for checkpoint KV keys (`{document_id}-pipeline-checkpoint`).
pub const CHECKPOINT_KEY_SUFFIX: &str = "-pipeline-checkpoint";

/// SPEC-047 P7e: durable extraction snapshot after successful persist.
/// Survives checkpoint clear so soft-reprocess / merge-only can skip LLM extract.
pub const EXTRACTION_SNAPSHOT_SUFFIX: &str = "-extraction-snapshot";

/// Maximum age of a crash-resume checkpoint in seconds (24h).
const CHECKPOINT_MAX_AGE_SECS: u64 = 86_400;

/// SPEC-047 P7e: durable snapshots live longer (7d) — soft-reprocess of completed docs.
const SNAPSHOT_MAX_AGE_SECS: u64 = 7 * 86_400;

/// Soft size guard before Postgres jsonb (~256 MiB hard limit / JENTRY_OFFLENMASK).
/// Stay well under so array-element totals and encoding overhead do not trip 54000.
const CHECKPOINT_MAX_SERIALIZED_BYTES: usize = 200_000_000;

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

    /// Full-text SHA-256 hex digest of source text (SPEC-083 X-28).
    pub content_hash: String,

    /// When true, embeddings were omitted to stay under jsonb size limits
    /// (SPEC-047 P5). Caller must re-run `Pipeline::ensure_embeddings` on resume.
    #[serde(default)]
    pub embeddings_omitted: bool,
}

impl PipelineCheckpoint {
    /// Compute a full SHA-256 content hash for integrity checking (X-28).
    ///
    /// Hashes the entire document so suffix-only edits invalidate checkpoints.
    fn compute_content_hash(text: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        hex::encode(hasher.finalize())
    }
}

/// Build the KV storage key for a document's pipeline checkpoint.
pub fn checkpoint_key(document_id: &str) -> String {
    format!("{document_id}{CHECKPOINT_KEY_SUFFIX}")
}

/// Build the KV key for a durable extraction snapshot (SPEC-047 P7e).
fn extraction_snapshot_key(document_id: &str) -> String {
    format!("{document_id}{EXTRACTION_SNAPSHOT_SUFFIX}")
}

/// Where reused extractions came from (SPEC-047 P7e).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionReuseKind {
    /// Mid-flight crash checkpoint (cleared on success).
    CrashCheckpoint,
    /// Durable post-success snapshot (survives finalize).
    DurableSnapshot,
}

/// Pure plan for extract-vs-reuse (SOLID: no I/O).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionReusePlan {
    /// Load from checkpoint or snapshot (caller picks which exists).
    Reuse(ExtractionReuseKind),
    /// Run LLM extraction.
    Fresh,
    /// `merge_only` requested but nothing reusable — fail closed.
    MergeOnlyMissing,
}

/// Decide extract reuse without I/O (SPEC-047 P7e SSOT).
///
/// Priority: force_fresh → Fresh (unless merge_only → Missing);
/// crash checkpoint → Durable snapshot → Fresh / Missing.
pub fn plan_extraction_reuse(
    has_checkpoint: bool,
    has_snapshot: bool,
    force_fresh: bool,
    merge_only: bool,
) -> ExtractionReusePlan {
    if force_fresh {
        return if merge_only {
            ExtractionReusePlan::MergeOnlyMissing
        } else {
            ExtractionReusePlan::Fresh
        };
    }
    if has_checkpoint {
        return ExtractionReusePlan::Reuse(ExtractionReuseKind::CrashCheckpoint);
    }
    if has_snapshot {
        return ExtractionReusePlan::Reuse(ExtractionReuseKind::DurableSnapshot);
    }
    if merge_only {
        ExtractionReusePlan::MergeOnlyMissing
    } else {
        ExtractionReusePlan::Fresh
    }
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
    store: Option<&dyn CheckpointArtifactStore>,
    document_id: &str,
    result: &ProcessingResult,
    workspace_id: &str,
    extraction_provider: &str,
    embedding_provider: &str,
    source_text: &str,
) -> Result<(), String> {
    // SPEC-047 P5: always strip embeddings — regenerable; LLM extract is not.
    // Mega-docs (7k+ ents) otherwise exceed Postgres jsonb ~256 MiB and leave
    // no durable resume after crash during merge.
    let mut slim_result = result.clone();
    let stripped = slim_result.strip_embeddings();

    let checkpoint = PipelineCheckpoint {
        result: slim_result,
        workspace_id: workspace_id.to_string(),
        extraction_provider: extraction_provider.to_string(),
        embedding_provider: embedding_provider.to_string(),
        created_at_epoch: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        content_hash: PipelineCheckpoint::compute_content_hash(source_text),
        embeddings_omitted: true,
    };

    let key = checkpoint_key(document_id);
    let value = serde_json::to_value(&checkpoint)
        .map_err(|e| format!("Failed to serialize pipeline checkpoint for {document_id}: {e}"))?;

    let approx_bytes = serde_json::to_vec(&value).map(|b| b.len()).unwrap_or(0);
    if approx_bytes > CHECKPOINT_MAX_SERIALIZED_BYTES {
        return Err(format!(
            "Pipeline checkpoint for {document_id} still too large after stripping embeddings \
             ({approx_bytes} bytes > {CHECKPOINT_MAX_SERIALIZED_BYTES}). \
             Resume will re-run extraction if this save is skipped."
        ));
    }

    // SPEC-091 WP1 (WP-AC-05): skip KV when relational authority AND typed write lands.
    // Non-UUID / no-pool paths keep KV so unit tests and degraded boots still resume.
    let relational = crate::services::relational_sidecar_store::checkpoints_prefer_relational();
    let wrote_typed = crate::services::relational_sidecar_store::typed_checkpoint_put(
        store,
        document_id,
        crate::services::relational_sidecar_store::CHECKPOINT_KIND_CRASH,
        &value,
    )
    .await;
    let skip_kv = relational && wrote_typed;
    if !skip_kv {
        kv.upsert(&[(key.clone(), value.clone())])
            .await
            .map_err(|e| format!("Failed to save pipeline checkpoint {key}: {e}"))?;
    }

    info!(
        document_id = %document_id,
        chunks = result.chunks.len(),
        entities = result.stats.entity_count,
        relationships = result.stats.relationship_count,
        embeddings_stripped = stripped,
        checkpoint_bytes = approx_bytes,
        kv_write = !skip_kv,
        typed_write = wrote_typed,
        "Saved pipeline checkpoint (extraction result persisted for resume; embeddings omitted)"
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
    store: Option<&dyn CheckpointArtifactStore>,
    document_id: &str,
    workspace_id: &str,
    extraction_provider: &str,
    embedding_provider: &str,
    source_text: &str,
) -> Option<ProcessingResult> {
    load_validated_checkpoint_blob(
        kv,
        store,
        &checkpoint_key(document_id),
        crate::services::relational_sidecar_store::CHECKPOINT_KIND_CRASH,
        document_id,
        workspace_id,
        extraction_provider,
        embedding_provider,
        source_text,
        CHECKPOINT_MAX_AGE_SECS,
        "pipeline checkpoint",
    )
    .await
}

/// Clear a pipeline checkpoint after successful processing.
///
/// Called when all storage stages complete successfully, freeing KV space.
pub async fn clear_pipeline_checkpoint(
    kv: &Arc<dyn KVStorage>,
    store: Option<&dyn CheckpointArtifactStore>,
    document_id: &str,
) {
    let key = checkpoint_key(document_id);
    crate::services::relational_sidecar_store::typed_checkpoint_delete(
        store,
        document_id,
        crate::services::relational_sidecar_store::CHECKPOINT_KIND_CRASH,
    )
    .await;
    match kv.delete(std::slice::from_ref(&key)).await {
        Ok(_) => debug!(document_id = %document_id, "Cleared pipeline checkpoint"),
        Err(e) => warn!(
            document_id = %document_id,
            error = %e,
            "Failed to clear pipeline checkpoint (non-fatal)"
        ),
    }
}

/// SPEC-047 P7e: persist a durable extraction snapshot after successful merge.
///
/// Survives [`clear_pipeline_checkpoint`] so soft-reprocess / `merge_only` can
/// skip LLM extract. Same slim shape as crash checkpoints (embeddings omitted).
#[allow(clippy::too_many_arguments)]
pub async fn save_extraction_snapshot(
    kv: &Arc<dyn KVStorage>,
    store: Option<&dyn CheckpointArtifactStore>,
    document_id: &str,
    result: &ProcessingResult,
    workspace_id: &str,
    extraction_provider: &str,
    embedding_provider: &str,
    source_text: &str,
) -> Result<(), String> {
    let mut slim_result = result.clone();
    let stripped = slim_result.strip_embeddings();

    let snapshot = PipelineCheckpoint {
        result: slim_result,
        workspace_id: workspace_id.to_string(),
        extraction_provider: extraction_provider.to_string(),
        embedding_provider: embedding_provider.to_string(),
        created_at_epoch: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        content_hash: PipelineCheckpoint::compute_content_hash(source_text),
        embeddings_omitted: true,
    };

    let key = extraction_snapshot_key(document_id);
    let value = serde_json::to_value(&snapshot)
        .map_err(|e| format!("Failed to serialize extraction snapshot for {document_id}: {e}"))?;

    let approx_bytes = serde_json::to_vec(&value).map(|b| b.len()).unwrap_or(0);
    if approx_bytes > CHECKPOINT_MAX_SERIALIZED_BYTES {
        return Err(format!(
            "Extraction snapshot for {document_id} too large ({approx_bytes} bytes)"
        ));
    }

    // SPEC-091 WP1: relational + successful typed write → KV write-stop.
    let relational = crate::services::relational_sidecar_store::checkpoints_prefer_relational();
    let wrote_typed = crate::services::relational_sidecar_store::typed_checkpoint_put(
        store,
        document_id,
        crate::services::relational_sidecar_store::CHECKPOINT_KIND_SNAPSHOT,
        &value,
    )
    .await;
    let skip_kv = relational && wrote_typed;
    if !skip_kv {
        kv.upsert(&[(key.clone(), value.clone())])
            .await
            .map_err(|e| format!("Failed to save extraction snapshot {key}: {e}"))?;
    }

    info!(
        document_id = %document_id,
        chunks = result.chunks.len(),
        entities = result.stats.entity_count,
        embeddings_stripped = stripped,
        snapshot_bytes = approx_bytes,
        kv_write = !skip_kv,
        typed_write = wrote_typed,
        "P7e: saved durable extraction snapshot (soft-reprocess / merge-only reuse)"
    );
    Ok(())
}

/// Load durable extraction snapshot (SPEC-047 P7e). Same validation as checkpoint.
pub async fn load_extraction_snapshot(
    kv: &Arc<dyn KVStorage>,
    store: Option<&dyn CheckpointArtifactStore>,
    document_id: &str,
    workspace_id: &str,
    extraction_provider: &str,
    embedding_provider: &str,
    source_text: &str,
) -> Option<ProcessingResult> {
    load_validated_checkpoint_blob(
        kv,
        store,
        &extraction_snapshot_key(document_id),
        crate::services::relational_sidecar_store::CHECKPOINT_KIND_SNAPSHOT,
        document_id,
        workspace_id,
        extraction_provider,
        embedding_provider,
        source_text,
        SNAPSHOT_MAX_AGE_SECS,
        "extraction snapshot",
    )
    .await
}

/// Clear durable extraction snapshot (Full reprocess / content wipe).
pub async fn clear_extraction_snapshot(
    kv: &Arc<dyn KVStorage>,
    store: Option<&dyn CheckpointArtifactStore>,
    document_id: &str,
) {
    let key = extraction_snapshot_key(document_id);
    crate::services::relational_sidecar_store::typed_checkpoint_delete(
        store,
        document_id,
        crate::services::relational_sidecar_store::CHECKPOINT_KIND_SNAPSHOT,
    )
    .await;
    match kv.delete(std::slice::from_ref(&key)).await {
        Ok(_) => debug!(document_id = %document_id, "Cleared extraction snapshot"),
        Err(e) => warn!(
            document_id = %document_id,
            error = %e,
            "Failed to clear extraction snapshot (non-fatal)"
        ),
    }
}

/// Shared load+validate for crash checkpoint and durable snapshot (DRY).
#[allow(clippy::too_many_arguments)]
async fn load_validated_checkpoint_blob(
    kv: &Arc<dyn KVStorage>,
    store: Option<&dyn CheckpointArtifactStore>,
    key: &str,
    sidecar_kind: &str,
    document_id: &str,
    workspace_id: &str,
    extraction_provider: &str,
    embedding_provider: &str,
    source_text: &str,
    max_age_secs: u64,
    label: &str,
) -> Option<ProcessingResult> {
    // SPEC-091 Wave B4: flag-gated typed read first; KV fallback on any gap.
    let value = if crate::services::relational_sidecar_store::checkpoints_prefer_relational() {
        match crate::services::relational_sidecar_store::typed_checkpoint_get(
            store,
            document_id,
            sidecar_kind,
        )
        .await
        {
            Some(v) => Some(v),
            None => match kv.get_by_id(key).await {
                Ok(v) => v,
                Err(e) => {
                    warn!(
                        document_id = %document_id,
                        error = %e,
                        %label,
                        "Failed to read reusable extraction blob — falling through"
                    );
                    return None;
                }
            },
        }
    } else {
        match kv.get_by_id(key).await {
            Ok(Some(v)) => Some(v),
            Ok(None) => {
                debug!(document_id = %document_id, %label, "No reusable extraction blob found");
                return None;
            }
            Err(e) => {
                warn!(
                    document_id = %document_id,
                    error = %e,
                    %label,
                    "Failed to read reusable extraction blob — falling through"
                );
                return None;
            }
        }
    };
    let Some(value) = value else {
        debug!(document_id = %document_id, %label, "No reusable extraction blob found");
        return None;
    };

    let checkpoint: PipelineCheckpoint = match serde_json::from_value(value) {
        Ok(cp) => cp,
        Err(e) => {
            warn!(
                document_id = %document_id,
                error = %e,
                %label,
                "Corrupt reusable extraction blob — clearing"
            );
            let _ = kv.delete(&[key.to_string()]).await;
            crate::services::relational_sidecar_store::typed_checkpoint_delete(
                store,
                document_id,
                sidecar_kind,
            )
            .await;
            return None;
        }
    };

    if checkpoint.workspace_id != workspace_id {
        info!(
            document_id = %document_id,
            %label,
            "Workspace mismatch on reusable extraction blob — ignoring"
        );
        let _ = kv.delete(&[key.to_string()]).await;
        crate::services::relational_sidecar_store::typed_checkpoint_delete(
            store,
            document_id,
            sidecar_kind,
        )
        .await;
        return None;
    }

    if checkpoint.extraction_provider != extraction_provider
        || checkpoint.embedding_provider != embedding_provider
    {
        info!(
            document_id = %document_id,
            %label,
            "Provider mismatch on reusable extraction blob — ignoring"
        );
        let _ = kv.delete(&[key.to_string()]).await;
        crate::services::relational_sidecar_store::typed_checkpoint_delete(
            store,
            document_id,
            sidecar_kind,
        )
        .await;
        return None;
    }

    let current_hash = PipelineCheckpoint::compute_content_hash(source_text);
    if checkpoint.content_hash != current_hash {
        info!(
            document_id = %document_id,
            %label,
            "Content hash mismatch on reusable extraction blob — ignoring"
        );
        let _ = kv.delete(&[key.to_string()]).await;
        crate::services::relational_sidecar_store::typed_checkpoint_delete(
            store,
            document_id,
            sidecar_kind,
        )
        .await;
        return None;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let age = now.saturating_sub(checkpoint.created_at_epoch);
    if age > max_age_secs {
        info!(
            document_id = %document_id,
            age_hours = age / 3600,
            max_age_hours = max_age_secs / 3600,
            %label,
            "Reusable extraction blob too old — ignoring"
        );
        let _ = kv.delete(&[key.to_string()]).await;
        crate::services::relational_sidecar_store::typed_checkpoint_delete(
            store,
            document_id,
            sidecar_kind,
        )
        .await;
        return None;
    }

    info!(
        document_id = %document_id,
        chunks = checkpoint.result.chunks.len(),
        entities = checkpoint.result.stats.entity_count,
        age_secs = age,
        %label,
        "Resuming from reusable extraction blob — skipping LLM extraction"
    );

    Some(checkpoint.result)
}

/// Clean up stale/orphaned pipeline checkpoints on server startup.
///
/// Scans KV storage for checkpoint keys older than `CHECKPOINT_MAX_AGE_SECS`
/// and removes them. This prevents unbounded storage growth from crashed
/// processing runs that never completed.
pub async fn cleanup_stale_checkpoints(
    kv: &Arc<dyn KVStorage>,
    store: Option<&dyn CheckpointArtifactStore>,
) {
    // SPEC-091 Wave B4: typed sweep mirrors the KV sweep (no-op without store).
    crate::services::relational_sidecar_store::cleanup_stale_typed_checkpoints(
        store,
        CHECKPOINT_MAX_AGE_SECS,
    )
    .await;
    let checkpoint_keys = match kv.keys_with_suffix(CHECKPOINT_KEY_SUFFIX).await {
        Ok(keys) => keys,
        Err(e) => {
            warn!(error = %e, "Failed to list checkpoint keys for cleanup");
            return;
        }
    };

    if checkpoint_keys.is_empty() {
        return;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let values = match kv.get_by_ids_ordered(&checkpoint_keys).await {
        Ok(values) => values,
        Err(e) => {
            warn!(error = %e, "Failed to batch-read checkpoint keys for cleanup");
            return;
        }
    };

    let mut cleaned = 0u32;
    for (key, maybe_value) in checkpoint_keys.iter().zip(values.iter()) {
        let Some(value) = maybe_value else {
            let _ = kv.delete(std::slice::from_ref(key)).await;
            cleaned += 1;
            continue;
        };
        if let Ok(cp) = serde_json::from_value::<PipelineCheckpoint>(value.clone()) {
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

    if cleaned > 0 {
        info!(
            total_checkpoints = checkpoint_keys.len(),
            cleaned = cleaned,
            "Cleaned up stale pipeline checkpoints on startup"
        );
    }
}

/// Suffix for per-chunk mid-extract checkpoint (`{document_id}-chunk-extract-partial`).
pub const PARTIAL_CHUNK_CHECKPOINT_SUFFIX: &str = "-chunk-extract-partial";

/// Incremental per-chunk extraction state (survives mid-document crash).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartialChunkCheckpoint {
    pub workspace_id: String,
    pub extraction_provider: String,
    pub content_hash: String,
    pub created_at_epoch: u64,
    /// chunk_id → extraction result
    pub completed: std::collections::HashMap<String, edgequake_pipeline::ExtractionResult>,
}

fn partial_chunk_key(document_id: &str) -> String {
    format!("{document_id}{PARTIAL_CHUNK_CHECKPOINT_SUFFIX}")
}

/// Load mid-extract per-chunk results when content/provider still match.
pub async fn load_partial_chunk_checkpoint(
    kv: &Arc<dyn KVStorage>,
    document_id: &str,
    workspace_id: &str,
    extraction_provider: &str,
    content: &str,
) -> Option<std::collections::HashMap<String, edgequake_pipeline::ExtractionResult>> {
    let key = partial_chunk_key(document_id);
    let value = kv.get_by_id(&key).await.ok().flatten()?;
    let parsed: PartialChunkCheckpoint = serde_json::from_value(value).ok()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if now.saturating_sub(parsed.created_at_epoch) > CHECKPOINT_MAX_AGE_SECS {
        warn!(document_id, "Partial chunk checkpoint expired — ignoring");
        return None;
    }
    let hash = PipelineCheckpoint::compute_content_hash(content);
    if parsed.workspace_id != workspace_id
        || parsed.extraction_provider != extraction_provider
        || parsed.content_hash != hash
    {
        debug!(document_id, "Partial chunk checkpoint stale — ignoring");
        return None;
    }
    if parsed.completed.is_empty() {
        return None;
    }
    info!(
        document_id,
        resumed_chunks = parsed.completed.len(),
        "Loaded partial chunk extraction checkpoint"
    );
    Some(parsed.completed)
}

/// Upsert one completed chunk into the partial checkpoint (best-effort).
pub async fn save_partial_chunk_extraction(
    kv: &Arc<dyn KVStorage>,
    document_id: &str,
    workspace_id: &str,
    extraction_provider: &str,
    content: &str,
    chunk_id: &str,
    result: edgequake_pipeline::ExtractionResult,
) {
    let key = partial_chunk_key(document_id);
    let hash = PipelineCheckpoint::compute_content_hash(content);
    let mut state = match kv.get_by_id(&key).await.ok().flatten() {
        Some(raw) => serde_json::from_value::<PartialChunkCheckpoint>(raw).unwrap_or_default(),
        None => PartialChunkCheckpoint::default(),
    };
    if state.content_hash != hash || state.workspace_id != workspace_id {
        state = PartialChunkCheckpoint {
            workspace_id: workspace_id.to_string(),
            extraction_provider: extraction_provider.to_string(),
            content_hash: hash,
            created_at_epoch: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            completed: Default::default(),
        };
    }
    state.extraction_provider = extraction_provider.to_string();
    state.completed.insert(chunk_id.to_string(), result);
    match serde_json::to_value(&state) {
        Ok(value) => {
            let approx = serde_json::to_vec(&value).map(|b| b.len()).unwrap_or(0);
            if approx > CHECKPOINT_MAX_SERIALIZED_BYTES {
                warn!(
                    document_id,
                    size = approx,
                    "Partial chunk checkpoint too large — skipping write"
                );
                return;
            }
            let _ = kv.upsert(&[(key, value)]).await;
        }
        Err(e) => {
            warn!(document_id, error = %e, "Failed to serialize partial chunk checkpoint");
        }
    }
}

/// Clear mid-extract partial checkpoint (after full success or force-fresh).
pub async fn clear_partial_chunk_checkpoint(kv: &Arc<dyn KVStorage>, document_id: &str) {
    let key = partial_chunk_key(document_id);
    let _ = kv.delete(std::slice::from_ref(&key)).await;
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
        assert_eq!(hash1.len(), 64); // full SHA-256 = 32 bytes = 64 hex chars
    }

    #[test]
    fn test_content_hash_differs_for_different_content() {
        let hash1 = PipelineCheckpoint::compute_content_hash("document A content");
        let hash2 = PipelineCheckpoint::compute_content_hash("document B content");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn e2e_checkpoint_rejects_suffix_change() {
        // X-28: full SHA-256 — suffix-only edits must invalidate the checkpoint.
        let base = "x".repeat(65_536);
        let text1 = format!("{}AAA", base);
        let text2 = format!("{}BBB", base);
        let hash1 = PipelineCheckpoint::compute_content_hash(&text1);
        let hash2 = PipelineCheckpoint::compute_content_hash(&text2);
        assert_ne!(hash1, hash2);
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
            embeddings_omitted: false,
        };

        let json = serde_json::to_value(&checkpoint).unwrap();
        let restored: PipelineCheckpoint = serde_json::from_value(json).unwrap();

        assert_eq!(restored.workspace_id, "ws-1");
        assert_eq!(restored.extraction_provider, "openai");
        assert_eq!(restored.result.document_id, "test-doc");
    }

    #[test]
    fn slim_checkpoint_omits_embeddings_flag_defaults_false_for_legacy() {
        use edgequake_pipeline::{ProcessingResult, ProcessingStats};

        // Legacy checkpoints lack `embeddings_omitted` — serde default must be false.
        let legacy = PipelineCheckpoint {
            result: ProcessingResult {
                document_id: "d".to_string(),
                chunks: vec![],
                extractions: vec![],
                stats: ProcessingStats::default(),
                lineage: None,
            },
            workspace_id: "ws".to_string(),
            extraction_provider: "mock".to_string(),
            embedding_provider: "mock".to_string(),
            created_at_epoch: 1,
            content_hash: "abcd".to_string(),
            embeddings_omitted: false,
        };
        let mut json = serde_json::to_value(&legacy).unwrap();
        json.as_object_mut().unwrap().remove("embeddings_omitted");
        let cp: PipelineCheckpoint = serde_json::from_value(json).unwrap();
        assert!(!cp.embeddings_omitted);
    }

    #[test]
    fn re_embedding_stage_string_is_honest() {
        // SPEC-057 P2: resume re-embed must surface a dedicated stage key.
        assert!(
            include_str!("text_insert/extraction.rs").contains("re_embedding"),
            "extraction resume path must set document status re_embedding"
        );
        assert!(
            include_str!("text_insert/extraction.rs").contains("embeddings_omitted"),
            "extraction resume path must mention embeddings_omitted"
        );
    }

    #[tokio::test]
    async fn save_strips_embeddings_from_checkpoint() {
        use edgequake_pipeline::{ProcessingResult, ProcessingStats, TextChunk};
        use edgequake_storage::MemoryKVStorage;

        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));
        let mut chunk = TextChunk::new("c0", "hello world", 0, 0, 11);
        chunk.embedding = Some(vec![0.1, 0.2, 0.3]);

        let result = ProcessingResult {
            document_id: "doc-slim".to_string(),
            chunks: vec![chunk],
            extractions: vec![],
            stats: ProcessingStats {
                chunk_count: 1,
                ..Default::default()
            },
            lineage: None,
        };
        assert!(!result.needs_reembed());

        save_pipeline_checkpoint(
            &kv,
            None,
            "doc-slim",
            &result,
            "ws",
            "mock",
            "mock",
            "hello world",
        )
        .await
        .unwrap();

        let loaded =
            load_pipeline_checkpoint(&kv, None, "doc-slim", "ws", "mock", "mock", "hello world")
                .await
                .expect("checkpoint should load");
        assert!(
            loaded.needs_reembed(),
            "slim checkpoint must require re-embed"
        );
        assert!(loaded.chunks[0].embedding.is_none());
        // In-memory original must still retain embeddings for persist path.
        assert!(result.chunks[0].embedding.is_some());
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
            None,
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
            None,
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

        save_pipeline_checkpoint(
            &kv, None, "doc-1", &result, "ws-A", "openai", "ollama", "text",
        )
        .await
        .unwrap();

        // Load with different workspace — should return None
        let loaded =
            load_pipeline_checkpoint(&kv, None, "doc-1", "ws-B", "openai", "ollama", "text").await;
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

        save_pipeline_checkpoint(
            &kv, None, "doc-2", &result, "ws", "openai", "ollama", "text",
        )
        .await
        .unwrap();

        // Load with different provider — should return None
        let loaded =
            load_pipeline_checkpoint(&kv, None, "doc-2", "ws", "anthropic", "ollama", "text").await;
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
            None,
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
        let loaded = load_pipeline_checkpoint(
            &kv,
            None,
            "doc-3",
            "ws",
            "openai",
            "ollama",
            "modified text",
        )
        .await;
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

        save_pipeline_checkpoint(
            &kv, None, "doc-4", &result, "ws", "openai", "ollama", "text",
        )
        .await
        .unwrap();

        // Verify it exists
        let loaded =
            load_pipeline_checkpoint(&kv, None, "doc-4", "ws", "openai", "ollama", "text").await;
        assert!(loaded.is_some());

        // Clear it
        clear_pipeline_checkpoint(&kv, None, "doc-4").await;

        // Verify it's gone
        let loaded =
            load_pipeline_checkpoint(&kv, None, "doc-4", "ws", "openai", "ollama", "text").await;
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
            load_pipeline_checkpoint(&kv, None, "doc-corrupt", "ws", "openai", "ollama", "text")
                .await;
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_no_checkpoint_returns_none() {
        use edgequake_storage::MemoryKVStorage;

        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));

        let loaded = load_pipeline_checkpoint(
            &kv,
            None,
            "nonexistent-doc",
            "ws",
            "openai",
            "ollama",
            "text",
        )
        .await;
        assert!(loaded.is_none());
    }

    #[test]
    fn plan_extraction_reuse_priority() {
        use super::{plan_extraction_reuse, ExtractionReuseKind, ExtractionReusePlan};

        assert_eq!(
            plan_extraction_reuse(true, true, false, false),
            ExtractionReusePlan::Reuse(ExtractionReuseKind::CrashCheckpoint)
        );
        assert_eq!(
            plan_extraction_reuse(false, true, false, false),
            ExtractionReusePlan::Reuse(ExtractionReuseKind::DurableSnapshot)
        );
        assert_eq!(
            plan_extraction_reuse(false, false, false, false),
            ExtractionReusePlan::Fresh
        );
        assert_eq!(
            plan_extraction_reuse(false, false, false, true),
            ExtractionReusePlan::MergeOnlyMissing
        );
        assert_eq!(
            plan_extraction_reuse(false, false, true, true),
            ExtractionReusePlan::MergeOnlyMissing
        );
        assert_eq!(
            plan_extraction_reuse(true, false, true, false),
            ExtractionReusePlan::Fresh
        );
    }

    #[tokio::test]
    async fn p7e_snapshot_survives_checkpoint_clear() {
        use edgequake_pipeline::{ProcessingResult, ProcessingStats};
        use edgequake_storage::MemoryKVStorage;

        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("p7e"));
        let result = ProcessingResult {
            document_id: "doc-p7e".to_string(),
            chunks: vec![],
            extractions: vec![],
            stats: ProcessingStats {
                entity_count: 2,
                relationship_count: 1,
                ..Default::default()
            },
            lineage: None,
        };
        let text = "durable snapshot source text";

        save_pipeline_checkpoint(
            &kv, None, "doc-p7e", &result, "ws", "openai", "ollama", text,
        )
        .await
        .unwrap();
        save_extraction_snapshot(
            &kv, None, "doc-p7e", &result, "ws", "openai", "ollama", text,
        )
        .await
        .unwrap();
        clear_pipeline_checkpoint(&kv, None, "doc-p7e").await;

        assert!(
            load_pipeline_checkpoint(&kv, None, "doc-p7e", "ws", "openai", "ollama", text)
                .await
                .is_none(),
            "crash checkpoint must be cleared"
        );
        let snap =
            load_extraction_snapshot(&kv, None, "doc-p7e", "ws", "openai", "ollama", text).await;
        assert!(snap.is_some(), "P7e durable snapshot must survive");
        assert_eq!(snap.unwrap().stats.entity_count, 2);

        clear_extraction_snapshot(&kv, None, "doc-p7e").await;
        assert!(
            load_extraction_snapshot(&kv, None, "doc-p7e", "ws", "openai", "ollama", text)
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn partial_chunk_checkpoint_roundtrip() {
        use edgequake_pipeline::ExtractionResult;
        use edgequake_storage::MemoryKVStorage;

        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("partial"));
        let result = ExtractionResult::new("c1");
        save_partial_chunk_extraction(&kv, "doc1", "ws", "ollama", "hello", "c1", result).await;
        let loaded = load_partial_chunk_checkpoint(&kv, "doc1", "ws", "ollama", "hello").await;
        assert!(loaded.as_ref().unwrap().contains_key("c1"));
        assert!(
            load_partial_chunk_checkpoint(&kv, "doc1", "ws", "ollama", "other")
                .await
                .is_none()
        );
        clear_partial_chunk_checkpoint(&kv, "doc1").await;
        assert!(
            load_partial_chunk_checkpoint(&kv, "doc1", "ws", "ollama", "hello")
                .await
                .is_none()
        );
    }
}
