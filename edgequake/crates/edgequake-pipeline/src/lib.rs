//! EdgeQuake Pipeline - Document Processing Pipeline
//!
//! # Implements
//!
//! - **FEAT0001**: Document Ingestion Pipeline
//! - **FEAT0002**: Entity Extraction
//! - **FEAT0003**: Relationship Discovery
//! - **FEAT0004**: Semantic Chunking
//! - **FEAT0005**: Embedding Generation
//! - **FEAT0006**: Entity Deduplication
//! - **FEAT0011**: Document-Chunk-Entity Lineage
//!
//! # Enforces
//!
//! - **BR0001**: Documents must be unique (content hash)
//! - **BR0002**: Chunk size 1200 tokens, overlap 100 tokens
//! - **BR0003**: Entity types from configurable list
//! - **BR0004**: Relationship keywords max 5 per edge
//! - **BR0005**: Entity description max 512 tokens
//! - **BR0006**: Same-entity relationships forbidden
//! - **BR0008**: Entity names normalized (UPPERCASE_UNDERSCORE)
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
//! | Stage | FEAT | Description |
//! |-------|------|-------------|
//! | Chunking | FEAT0004 | Split documents into overlapping chunks |
//! | Entity Extraction | FEAT0002 | Use LLM to extract entities |
//! | Relationship Extraction | FEAT0003 | Use LLM to extract relationships |
//! | Merging | FEAT0006 | Deduplicate and merge into graph |
//! | Embedding | FEAT0005 | Generate and store embeddings |
//!
//! # Architecture
//!
//! The pipeline is designed for async, parallelizable processing with
//! configurable batch sizes and rate limiting for LLM calls.
//!
//! # SOTA Features
//!
//! - **Tuple-based extraction**: More robust than JSON parsing
//! - **Entity name normalization**: Consistent naming across extractions (BR0008)
//! - **Line number tracking**: Full lineage support for chunks (FEAT0011)
//! - **Parallel processing**: Configurable concurrency for extractions
//!
//! # See Also
//!
//! - [`crate::pipeline`] for the main Pipeline struct
//! - [`crate::extractor`] for entity/relationship extraction
//! - [`crate::chunker`] for document chunking

pub mod cache;
pub mod chunker;
pub mod error;
pub mod extractor;
pub mod ingestion_types;
pub mod lineage;
pub mod merger;
pub mod pipeline;
pub mod progress;
pub mod prompts;
pub mod sanitizer;
pub mod summarizer;
pub mod table_preprocessor;
pub mod validation;

pub use cache::{
    generate_cache_key, generate_cache_key_multi, CacheEntry, CacheStats, CacheType,
    CachedExtractor, LLMCache, MemoryLLMCache,
};
pub use chunker::{
    calculate_line_numbers, CharacterBasedChunking, ChunkResult, Chunker, ChunkerConfig,
    ChunkingStrategy, ParagraphBoundaryChunking, SentenceBoundaryChunking, TextChunk,
    TokenBasedChunking,
};
pub use error::{
    ChunkExtractionOutcome, ChunkFailure, PipelineError, ResilientExtractionResult, Result,
};
pub use extractor::{
    EntityExtractor, ExtractedEntity, ExtractedRelationship, ExtractionResult, GleaningConfig,
    GleaningExtractor, LLMExtractor, SOTAExtractor, SimpleExtractor,
};
// Re-export unified ingestion types for frontend compatibility
pub use ingestion_types::{
    error_codes, IngestionError as UnifiedIngestionError,
    IngestionProgress as UnifiedIngestionProgress, SourceType,
    StageProgress as UnifiedStageProgress, StageStatus as UnifiedStageStatus, UnifiedStage,
};
pub use lineage::{
    ChunkLineage, DescriptionVersion, DocumentLineage, EntityLineage, EntitySource,
    ExtractionMetadata, LineageBuilder, RelationshipLineage, SourceSpan,
};
pub use merger::{KnowledgeGraphMerger, MergeStats, MergerConfig};
pub use pipeline::{
    ChunkProgressCallback, ChunkProgressUpdate, CostBreakdownStats, Pipeline, PipelineConfig,
    ProcessingResult, ProcessingStats,
};
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
pub use sanitizer::{EmojiMode, SanitizeConfig, SanitizeReport, Sanitizer};
pub use summarizer::{DescriptionSummarizer, LLMSummarizer, SimpleSummarizer, SummarizerConfig};
pub use table_preprocessor::{
    preprocess_tabular_content, PreprocessResult, TablePreprocessorConfig,
};
pub use validation::{
    validate_document_content, validate_document_filename, DocumentValidator, ValidationCode,
    ValidationConfig, ValidationIssue, ValidationResult,
};
