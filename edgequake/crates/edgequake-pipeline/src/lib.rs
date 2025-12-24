//! EdgeQuake Pipeline - Document Processing Pipeline
//!
//! This crate handles the ingestion and processing of documents:
//!
//! - Document chunking with overlap
//! - Entity and relationship extraction via LLM
//! - Knowledge graph construction
//! - Embedding generation and storage
//!
//! # Pipeline Stages
//!
//! 1. **Chunking**: Split documents into overlapping chunks
//! 2. **Entity Extraction**: Use LLM to extract entities from chunks
//! 3. **Relationship Extraction**: Use LLM to extract relationships
//! 4. **Merging**: Merge entities and relationships into knowledge graph
//! 5. **Embedding**: Generate and store embeddings for chunks and entities
//!
//! # Architecture
//!
//! The pipeline is designed for async, parallelizable processing with
//! configurable batch sizes and rate limiting for LLM calls.

pub mod chunker;
pub mod error;
pub mod extractor;
pub mod merger;
pub mod pipeline;
pub mod summarizer;

pub use chunker::{
    CharacterBasedChunking, ChunkResult, Chunker, ChunkerConfig, ChunkingStrategy, TextChunk,
    TokenBasedChunking,
};
pub use error::{PipelineError, Result};
pub use extractor::{
    EntityExtractor, ExtractedEntity, ExtractedRelationship, ExtractionResult, GleaningConfig,
    GleaningExtractor, LLMExtractor, SimpleExtractor,
};
pub use merger::{KnowledgeGraphMerger, MergeStats, MergerConfig};
pub use pipeline::{Pipeline, PipelineConfig};
pub use summarizer::{DescriptionSummarizer, LLMSummarizer, SimpleSummarizer, SummarizerConfig};
