//! Vector table copy phase for WorkspaceCopy (EN-3677 Phase 1).
//!
//! ## Verbatim copy — same model, same dimensions
//!
//! v1 is same-model-only, so vector tables can be copied byte-for-byte via
//! `INSERT INTO dest SELECT ...`. No re-embed round trip, no LLM call, no
//! provider quota consumption. This is what makes v1 fast (~seconds) versus
//! v2 (~minutes for a KB with 10k chunks that has to hit the embedding
//! provider again).
//!
//! ## `halfvec` and `model_id` preservation
//!
//! Embeddings columns use pgvector's `halfvec(N)` (or `vector(N)` for legacy
//! rows). Both types round-trip via INSERT ... SELECT with no coercion —
//! no cast is needed. The `model_id` column MUST be preserved verbatim so
//! future queries on the destination workspace resolve to the same
//! embedding model as the source (rejecting the copy at that point would
//! silently strand vectors on the wrong model).
//!
//! ## Per-workspace vector table naming
//!
//! SPEC-054 / GH #297 introduced per-workspace physical vector tables named
//! `eq_..._ws_{workspace_id}_vectors`. Copy therefore needs to:
//!   1. Ensure the destination workspace's vector table exists (created
//!      lazily by the `vector_registry`).
//!   2. `INSERT INTO dest SELECT ... FROM source` with `workspace_id`,
//!      `chunk_id`, `entity_id`, `relationship_id` rewritten via the id-maps.

use uuid::Uuid;

use crate::services::workspace_copy::kv_copy::KvIdMaps;

/// Result of the vector copy pass — the `vectors_copied` field feeds
/// directly into the wire response's `vectors_copied` field.
#[derive(Debug, Clone, Copy, Default)]
pub struct VectorCopyResult {
    pub vectors_copied: usize,
    /// Vector tables that were physically touched (chunk_embeddings,
    /// entity_embeddings, relationship_embeddings, report_embeddings).
    /// Useful in logs for confirming the copy went end-to-end.
    pub tables_touched: usize,
}

/// Which embedding table families the copy walks. Kept as an enum so the
/// per-family counters can be reported separately in future observability
/// work without a signature change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingTable {
    ChunkEmbeddings,
    EntityEmbeddings,
    RelationshipEmbeddings,
    ReportEmbeddings,
}

impl EmbeddingTable {
    pub fn as_str(self) -> &'static str {
        match self {
            EmbeddingTable::ChunkEmbeddings => "chunk_embeddings",
            EmbeddingTable::EntityEmbeddings => "entity_embeddings",
            EmbeddingTable::RelationshipEmbeddings => "relationship_embeddings",
            EmbeddingTable::ReportEmbeddings => "report_embeddings",
        }
    }
}

pub const ALL_EMBEDDING_TABLES: &[EmbeddingTable] = &[
    EmbeddingTable::ChunkEmbeddings,
    EmbeddingTable::EntityEmbeddings,
    EmbeddingTable::RelationshipEmbeddings,
    EmbeddingTable::ReportEmbeddings,
];

/// SQL template for copying one embedding table with parent-id remap.
///
/// The `parent_id_column` is the column in the embeddings table that FKs
/// into the parent (`chunk_id`, `entity_id`, ...). The join against the
/// id-map temp table remaps every parent id to its dest counterpart.
///
/// This is kept as a template rather than executed here because the
/// storage crate owns the actual connection handle — Phase 1 only lays
/// out the pattern; Phase 2's follow-up commit will hook it up.
pub fn copy_embeddings_sql_template(
    table: EmbeddingTable,
    parent_id_column: &str,
    id_map_table: &str,
) -> String {
    let t = table.as_str();
    format!(
        "INSERT INTO {t} (id, workspace_id, {parent_id_column}, model_id, embedding, created_at) \
         SELECT gen_random_uuid(), $2::uuid, m.dest_id, e.model_id, e.embedding, e.created_at \
         FROM {t} e JOIN {id_map_table} m ON m.src_id = e.{parent_id_column} \
         WHERE e.workspace_id = $1::uuid"
    )
}

/// Execute the vector copy pass. **Scaffold**: real execution needs the
/// `vector_registry` handle from AppState to (a) ensure the destination
/// workspace's per-workspace physical table exists and (b) run the bulk
/// INSERTs above. Phase 1 emits a warning and returns zero counts.
pub async fn copy_vectors(
    _source_workspace_id: Uuid,
    _dest_workspace_id: Uuid,
    _id_maps: &KvIdMaps,
) -> Result<VectorCopyResult, String> {
    tracing::warn!(
        source_workspace_id = %_source_workspace_id,
        dest_workspace_id = %_dest_workspace_id,
        embedding_table_count = ALL_EMBEDDING_TABLES.len(),
        "workspace_copy::vector_copy: scaffold — vector copy is a no-op in Phase 1"
    );
    Ok(VectorCopyResult::default())
}
