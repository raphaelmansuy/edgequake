//! Single-writer coalescing partial-chunk extraction checkpoint (SPEC-156).
//!
//! WHY: fire-and-forget `tokio::spawn(save_partial_chunk_extraction)` per chunk
//! races on one KV blob (lost updates) and re-hashes + rewrites the whole
//! document state O(N) times → O(N²) bytes. This writer serialises inserts
//! through one mpsc consumer, hashes content once, coalesces KV flushes
//! (~250 ms), and guarantees a final flush when the handle is dropped /
//! `flush_now` is called. Pattern mirrors `services/run_progress_writer.rs`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use edgequake_pipeline::ExtractionResult;
use edgequake_storage::traits::KVStorage;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{timeout, Instant};

use super::pipeline_checkpoint::{
    flush_partial_chunk_state, PartialChunkCheckpoint, PipelineCheckpoint,
};

const COALESCE_MS: u64 = 250;

enum WriterMsg {
    Chunk {
        chunk_id: String,
        result: ExtractionResult,
    },
    Flush(oneshot::Sender<()>),
}

/// Handle shared by the extract callback (cheap to clone).
#[derive(Clone)]
pub struct PartialChunkCheckpointWriter {
    tx: mpsc::UnboundedSender<WriterMsg>,
    chunks_submitted: Arc<AtomicU64>,
}

impl PartialChunkCheckpointWriter {
    /// Spawn the single-writer task. Content is hashed once here.
    pub fn spawn(
        document_id: String,
        workspace_id: String,
        extraction_provider: String,
        content: &str,
        kv: Arc<dyn KVStorage>,
    ) -> Self {
        let content_hash = PipelineCheckpoint::compute_content_hash(content);
        let (tx, rx) = mpsc::unbounded_channel();
        let chunks_submitted = Arc::new(AtomicU64::new(0));
        crate::pg_session::spawn_with_rls(async move {
            run_writer_loop(
                document_id,
                workspace_id,
                extraction_provider,
                content_hash,
                kv,
                rx,
            )
            .await;
        });
        Self {
            tx,
            chunks_submitted,
        }
    }

    pub fn submit(&self, chunk_id: String, result: ExtractionResult) {
        self.chunks_submitted.fetch_add(1, Ordering::Relaxed);
        let _ = self.tx.send(WriterMsg::Chunk { chunk_id, result });
    }

    /// Block until the pending state is written (best-effort; 2 s timeout).
    pub async fn flush_now(&self) {
        let (tx, rx) = oneshot::channel();
        if self.tx.send(WriterMsg::Flush(tx)).is_err() {
            return;
        }
        let _ = timeout(Duration::from_secs(2), rx).await;
    }

    pub fn chunks_submitted(&self) -> u64 {
        self.chunks_submitted.load(Ordering::Relaxed)
    }
}

async fn run_writer_loop(
    document_id: String,
    workspace_id: String,
    extraction_provider: String,
    content_hash: String,
    kv: Arc<dyn KVStorage>,
    mut rx: mpsc::UnboundedReceiver<WriterMsg>,
) {
    let now_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut state = PartialChunkCheckpoint {
        workspace_id: workspace_id.clone(),
        extraction_provider: extraction_provider.clone(),
        content_hash: content_hash.clone(),
        created_at_epoch: now_epoch,
        completed: Default::default(),
    };

    // Seed from an existing matching partial so a restarted writer does not
    // wipe earlier chunks from a previous process (best-effort).
    seed_from_kv_if_matching(&kv, &document_id, &content_hash, &workspace_id, &mut state).await;

    let mut dirty = false;
    let mut last_flush = Instant::now()
        .checked_sub(Duration::from_millis(COALESCE_MS))
        .unwrap_or_else(Instant::now);

    loop {
        let msg = if dirty {
            let remaining = Duration::from_millis(COALESCE_MS).saturating_sub(last_flush.elapsed());
            match timeout(remaining, rx.recv()).await {
                Ok(Some(m)) => Some(m),
                Ok(None) => None,
                Err(_) => {
                    flush_partial_chunk_state(&kv, &document_id, &state).await;
                    dirty = false;
                    last_flush = Instant::now();
                    continue;
                }
            }
        } else {
            rx.recv().await
        };

        let Some(msg) = msg else {
            if dirty {
                flush_partial_chunk_state(&kv, &document_id, &state).await;
            }
            break;
        };

        match msg {
            WriterMsg::Chunk { chunk_id, result } => {
                state.extraction_provider = extraction_provider.clone();
                state.completed.insert(chunk_id, result);
                dirty = true;
            }
            WriterMsg::Flush(ack) => {
                if dirty {
                    flush_partial_chunk_state(&kv, &document_id, &state).await;
                    dirty = false;
                    last_flush = Instant::now();
                }
                let _ = ack.send(());
            }
        }

        if dirty && last_flush.elapsed() >= Duration::from_millis(COALESCE_MS) {
            flush_partial_chunk_state(&kv, &document_id, &state).await;
            dirty = false;
            last_flush = Instant::now();
        }
    }
}

async fn seed_from_kv_if_matching(
    kv: &Arc<dyn KVStorage>,
    document_id: &str,
    content_hash: &str,
    workspace_id: &str,
    state: &mut PartialChunkCheckpoint,
) {
    let key = format!(
        "{document_id}{}",
        super::pipeline_checkpoint::PARTIAL_CHUNK_CHECKPOINT_SUFFIX
    );
    let Ok(Some(raw)) = kv.get_by_id(&key).await else {
        return;
    };
    let Ok(parsed) = serde_json::from_value::<PartialChunkCheckpoint>(raw) else {
        return;
    };
    if parsed.content_hash == content_hash && parsed.workspace_id == workspace_id {
        state.created_at_epoch = parsed.created_at_epoch;
        state.completed = parsed.completed;
        state.extraction_provider = parsed.extraction_provider;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::MemoryKVStorage;

    #[tokio::test]
    async fn writer_coalesces_concurrent_chunks_without_loss() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("partial-writer"));
        let content = "hello world document body";
        let writer = PartialChunkCheckpointWriter::spawn(
            "doc-pw-1".into(),
            "ws".into(),
            "ollama".into(),
            content,
            Arc::clone(&kv),
        );

        for i in 0..20 {
            writer.submit(format!("c{i}"), ExtractionResult::new(format!("c{i}")));
        }
        writer.flush_now().await;

        let loaded = super::super::pipeline_checkpoint::load_partial_chunk_checkpoint(
            &kv, "doc-pw-1", "ws", "ollama", content,
        )
        .await
        .expect("partial checkpoint must load");
        assert_eq!(loaded.len(), 20);
        for i in 0..20 {
            assert!(loaded.contains_key(&format!("c{i}")));
        }
        assert_eq!(writer.chunks_submitted(), 20);
    }

    #[tokio::test]
    async fn writer_seeds_prior_matching_partial() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("partial-seed"));
        let content = "seed body";
        super::super::pipeline_checkpoint::save_partial_chunk_extraction(
            &kv,
            "doc-seed",
            "ws",
            "ollama",
            content,
            "prior",
            ExtractionResult::new("prior"),
        )
        .await;

        let writer = PartialChunkCheckpointWriter::spawn(
            "doc-seed".into(),
            "ws".into(),
            "ollama".into(),
            content,
            Arc::clone(&kv),
        );
        writer.submit("new".into(), ExtractionResult::new("new"));
        writer.flush_now().await;

        let loaded = super::super::pipeline_checkpoint::load_partial_chunk_checkpoint(
            &kv, "doc-seed", "ws", "ollama", content,
        )
        .await
        .expect("must load");
        assert!(loaded.contains_key("prior"));
        assert!(loaded.contains_key("new"));
    }
}
