//! EdgeQuake Pipeline - Document Processing Pipeline
//!
//! This crate handles the ingestion and processing of documents:
//!
//! - Document chunking with overlap and line number tracking
//! - Entity and relationship extraction via LLM (SOTA tuple format)
//! - Knowledge graph construction
//! - Embedding generation and storage
//! - LLM response caching
//!
//! # Pipeline Stages
//!
//! 1. **Chunking**: Split documents into overlapping chunks with line numbers
//! 2. **Entity Extraction**: Use LLM to extract entities from chunks
//! 3. **Relationship Extraction**: Use LLM to extract relationships
//! 4. **Merging**: Merge entities and relationships into knowledge graph
//! 5. **Embedding**: Generate and store embeddings for chunks and entities
//!
//! # Architecture
//!
//! The pipeline is designed for async, parallelizable processing with
//! configurable batch sizes and rate limiting for LLM calls.
//!
//! # SOTA Features
//!
//! - **Tuple-based extraction**: More robust than JSON parsing
//! - **Entity name normalization**: Consistent naming across extractions
//! - **Line number tracking**: Full lineage support for chunks
//! - **Parallel processing**: Configurable concurrency for extractions

pub mod cache;
pub mod chunker;
pub mod error;
pub mod extractor;
pub mod lineage;
pub mod merger;
pub mod pipeline;
pub mod progress;
pub mod prompts;
pub mod summarizer;

pub use cache::{
    generate_cache_key, generate_cache_key_multi, CacheEntry, CacheStats, CacheType,
    CachedExtractor, LLMCache, MemoryLLMCache,
};
pub use chunker::{
    calculate_line_numbers, CharacterBasedChunking, ChunkResult, Chunker, ChunkerConfig,
    ChunkingStrategy, TextChunk, TokenBasedChunking,
};
pub use error::{PipelineError, Result};
pub use extractor::{
    EntityExtractor, ExtractedEntity, ExtractedRelationship, ExtractionResult, GleaningConfig,
    GleaningExtractor, LLMExtractor, SOTAExtractor, SimpleExtractor,
};
pub use lineage::{
    ChunkLineage, DescriptionVersion, DocumentLineage, EntityLineage, EntitySource,
    ExtractionMetadata, LineageBuilder, RelationshipLineage, SourceSpan,
};
pub use merger::{KnowledgeGraphMerger, MergeStats, MergerConfig};
pub use pipeline::{Pipeline, PipelineConfig, ProcessingResult, ProcessingStats};
pub use progress::{
    default_model_pricing, CostBreakdown, CostTracker, IngestionError, IngestionProgress,
    IngestionStatus, MessageLevel, ModelPricing, OperationCost, PipelineStage, ProgressMessage,
    ProgressTracker, StageProgress, StageStatus,
};
pub use prompts::{
    default_entity_types, normalize_entity_name, EntityExtractionPrompts, HybridExtractionParser,
    JsonExtractionParser, SummarizationPrompts, TupleParser, DEFAULT_COMPLETION_DELIMITER,
    DEFAULT_TUPLE_DELIMITER, SUPPORTED_LANGUAGES,
};
pub use summarizer::{DescriptionSummarizer, LLMSummarizer, SimpleSummarizer, SummarizerConfig};
