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
//! - **BR0002**: Chunk size 800 tokens (default), overlap 100 tokens
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

pub mod adaptive_chunking;
pub mod cache;
pub mod chunk_storage;
pub mod chunker;
pub mod error;
pub mod extractor;
pub mod ingestion_pipeline;
pub mod ingestion_types;
pub mod lineage;
pub mod markdown_ir;
pub mod merger;
pub mod multimodal;
pub mod persistence;
pub mod pipeline;
pub mod progress;
pub mod prompts;
pub mod sanitizer;
pub mod stage_bridge;
pub mod summarizer;
pub mod table_preprocessor;
pub mod test_fixtures;
pub mod validation;

pub use adaptive_chunking::{adaptive_chunk_overlap, calculate_adaptive_chunk_size};
pub use cache::{
    generate_cache_key, generate_cache_key_multi, CacheEntry, CacheStats, CacheType,
    CachedExtractor, LLMCache, MemoryLLMCache,
};
pub use chunk_storage::build_chunk_kv_records;
pub use chunker::{
    calculate_line_numbers, default_recursive_separators, make_page_marker, parse_page_marker,
    resolve_chunker, split_into_page_segments, CharacterBasedChunking, ChunkOptions, ChunkResult,
    ChunkStrategy, Chunker, ChunkerConfig, ChunkingStrategy, MarkdownChunking, PageAwareChunking,
    ParagraphBoundaryChunking, RecursiveCharacterChunking, SectionMetadata,
    SentenceBoundaryChunking, TextChunk, TokenBasedChunking, PAGE_MARKER_PREFIX,
    PAGE_MARKER_SUFFIX,
};
pub use error::{
    ChunkExtractionOutcome, ChunkFailure, PipelineError, ResilientExtractionResult, Result,
};
pub use extractor::{
    assign_token_usage, effective_temperature_for_model, extraction_completion_options,
    recommended_chunk_size_for_bytes, ConfigurableEntitySchema, EntityExtractor, ExtractedEntity,
    ExtractedRelationship, ExtractionResult, GleaningConfig, GleaningExtractor, LLMExtractor,
    SOTAExtractor, SimpleExtractor,
};
pub use ingestion_pipeline::{
    build_chunker_config, build_ingestion_pipeline, build_ingestion_pipeline_simple,
    IngestionPipelineOptions,
};
pub use markdown_ir::{extract_markdown_blocks, format_breadcrumb, PREFACE_HEADING};
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
pub use merger::{
    description_similarity, KnowledgeGraphMerger, LineageSink, MergeArtifacts, MergePhase,
    MergeProgress, MergeProgressCallback, MergeStats, MergerConfig, NoopEntitySink,
    NoopLineageSink, RelationalEntitySink,
};
pub use multimodal::{
    inject_modality_relations, parse_mm_display_name, MmChunkSidecarMeta, MmHeadingBlock,
    MmSidecarBlock, MmSidecarRef,
};
pub use persistence::{
    build_chunk_vector_batch, persist_processing_result, ChunkVectorBuildOptions,
    DefaultIngestionPersister, IngestionPersistConfig, IngestionPersistContext,
    IngestionPersistOutput, IngestionPersistSettings, IngestionPersister,
};
pub use pipeline::{
    ChunkProgressCallback,
    ChunkProgressUpdate,
    CostBreakdownStats,
    EmbedProgressCallback,
    EmbedProgressUpdate,
    Pipeline,
    PipelineConfig,
    ProcessingResult,
    ProcessingStats,
    // Issue-194: configurable timeout / concurrency constants
    DEFAULT_CHUNK_MAX_RETRIES,
    DEFAULT_CHUNK_TIMEOUT_SECS,
    DEFAULT_INITIAL_RETRY_DELAY_MS,
    DEFAULT_MAX_CONCURRENT_EXTRACTIONS,
    MAX_CHUNK_MAX_RETRIES,
    MIN_CHUNK_TIMEOUT_SECS,
};
pub use progress::{
    default_model_pricing, CostBreakdown, CostTracker, IngestionError, IngestionProgress,
    IngestionStatus, MessageLevel, ModelPricing, OperationCost, PipelineStage, ProgressMessage,
    ProgressTracker, StageProgress, StageStatus,
};
pub use prompts::{
    default_entity_types, detect_format_markers, format_section_context, normalize_entity_name,
    text_with_section_context, truncate_section_context, EntityExtractionPrompts,
    ExtractionResultParser, HybridExtractionParser, JsonExtractionParser, SummarizationPrompts,
    TupleParser, DEFAULT_COMPLETION_DELIMITER, DEFAULT_TUPLE_DELIMITER, SUPPORTED_LANGUAGES,
};
pub use sanitizer::{EmojiMode, SanitizeConfig, SanitizeReport, Sanitizer};
pub use stage_bridge::{
    pipeline_stage_to_unified, tasks_phase_slug_to_unified, unified_stage_slug,
    unified_to_pipeline_stage, unified_to_tasks_phase_slug,
};
pub use summarizer::{DescriptionSummarizer, LLMSummarizer, SimpleSummarizer, SummarizerConfig};
pub use table_preprocessor::{
    preprocess_tabular_content, PreprocessResult, TablePreprocessorConfig,
};
pub use test_fixtures::SPEC021_SARAH_CHEN_EXTRACTION_JSON;
pub use validation::{
    validate_document_content, validate_document_filename, DocumentValidator, ValidationCode,
    ValidationConfig, ValidationIssue, ValidationResult,
};
