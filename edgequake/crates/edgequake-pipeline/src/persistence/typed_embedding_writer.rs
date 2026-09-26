//! SPEC-091 W3 — typed chunk embedding dual-write (single writer site).
//!
//! After relational `chunks` are persisted (same stage as the legacy
//! `eq_*_vectors` upsert), this hook mirrors the embedded chunk vectors into
//! `chunk_embeddings` via the `EmbeddingIndex` port. Chunk UUIDs are resolved
//! from the relational spine through the `ChunkRepository` port — the typed row
//! therefore always references a real `chunks.id` (LD-02) and this crate stays
//! free of a direct `sqlx` dependency.

use std::collections::HashMap;

use edgequake_storage::traits::domain::{
    ChunkRepository, DocumentId, EmbeddingIndex, EmbeddingRow, ModelId, WorkspaceId,
};
use edgequake_storage::StorageError;
use uuid::Uuid;

use crate::pipeline::ProcessingResult;

use super::document_id_resolve::resolve_relational_document_id;
use super::IngestionPersistContext;

/// Dual-write embedded chunk vectors into `chunk_embeddings`.
///
/// No-op when the document has no embedded chunks, the document id cannot be
/// resolved to a relational UUID (SPEC-118 maps `injection::` composites), or
/// the workspace id is absent/invalid (typed schema requires
/// `workspace_id NOT NULL`). This path never fails the ingest by itself: the
/// caller maps errors per backend policy (warn-only during rollout).
pub async fn persist_typed_chunk_embeddings(
    index: &dyn EmbeddingIndex,
    repo: &dyn ChunkRepository,
    ctx: &IngestionPersistContext,
    result: &ProcessingResult,
) -> Result<u64, StorageError> {
    // SPEC-118: share resolve SSOT with relational_chunk_writer (DRY).
    let doc_uuid = match resolve_relational_document_id(&ctx.document_id) {
        Ok(id) => id.into_uuid(),
        Err(_) => return Ok(0),
    };
    let ws_uuid = match ctx
        .workspace_id
        .as_deref()
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(u) => u,
        None => return Ok(0),
    };

    let embedded: Vec<(usize, &[f32])> = result
        .chunks
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.embedding.as_deref().map(|e| (i, e)))
        .collect();
    if embedded.is_empty() {
        return Ok(0);
    }

    // Resolve relational chunk ids for this document in one round trip
    // (LAW-D7); unmatched chunks are skipped (defensive — W1 writer already
    // inserted them, so a miss indicates a partial-failure retry).
    let spine = repo.load_for_document(DocumentId::new(doc_uuid)).await?;
    let id_by_index: HashMap<i32, Uuid> =
        spine.into_iter().map(|c| (c.chunk_index, c.id.0)).collect();

    let mut batch: Vec<EmbeddingRow> = Vec::with_capacity(embedded.len());
    for (chunk_pos, embedding) in embedded {
        let chunk_index = i32::try_from(chunk_pos).unwrap_or(i32::MAX);
        let Some(chunk_uuid) = id_by_index.get(&chunk_index) else {
            continue;
        };
        batch.push(EmbeddingRow {
            chunk_id: (*chunk_uuid).into(),
            workspace_id: WorkspaceId::new(ws_uuid),
            embedding: embedding.to_vec(),
            dimensions: embedding.len() as i32,
        });
    }
    if batch.is_empty() {
        return Ok(0);
    }

    let report = index.upsert_batch(ModelId(Uuid::nil()), &batch).await?;
    Ok(report.upserted)
}
