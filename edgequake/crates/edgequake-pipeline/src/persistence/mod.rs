//! Cross-store persistence after pipeline compute (SPEC-021 P-G2).

mod document_id_resolve;
mod ingestion_persister;
mod relational_chunk_writer;
pub mod typed_embedding_writer;

pub use document_id_resolve::{is_injection_composite_document_id, resolve_relational_document_id};
pub use ingestion_persister::{
    build_chunk_vector_batch, persist_processing_result, ChunkVectorBuildOptions,
    DefaultIngestionPersister, IngestionAuthority, IngestionPersistConfig, IngestionPersistContext,
    IngestionPersistOutput, IngestionPersistSettings, IngestionPersister,
};
pub use relational_chunk_writer::{build_relational_chunks, persist_relational_chunks};
pub use typed_embedding_writer::persist_typed_chunk_embeddings;
