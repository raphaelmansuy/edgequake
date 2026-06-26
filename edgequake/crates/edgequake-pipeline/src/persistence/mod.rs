//! Cross-store persistence after pipeline compute (SPEC-021 P-G2).

mod ingestion_persister;

pub use ingestion_persister::{
    build_chunk_vector_batch, persist_processing_result, ChunkVectorBuildOptions,
    DefaultIngestionPersister, IngestionPersistConfig, IngestionPersistContext,
    IngestionPersistOutput, IngestionPersistSettings, IngestionPersister,
};
