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
pub mod anthropic_images;
pub mod chunk_storage;
pub mod chunker;
pub mod contextual_chunk;
pub mod entity_display;
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
pub mod structure_induce;
pub mod summarizer;
pub mod table_preprocessor;
pub mod test_fixtures;
pub mod text_embedder;
pub mod token_estimator;
pub mod validation;

pub use adaptive_chunking::{
    adaptive_chunk_overlap, adaptive_chunking_enabled, calculate_adaptive_chunk_size,
    chunking_policy_from_metadata, env_fixed_chunk_overlap, env_fixed_chunk_size,
    policy_uses_adaptive, resolve_base_chunk_size_overlap,
    resolve_base_chunk_size_overlap_with_policy, validate_fixed_pair, ChunkingMode, ChunkingPolicy,
    DEFAULT_FIXED_CHUNK_OVERLAP, DEFAULT_FIXED_CHUNK_TOKEN_SIZE,
};
pub use anthropic_images::{
    anthropic_image_source_json, materialize_image_for_anthropic, materialize_images_for_anthropic,
    AnthropicImageError,
};
pub use chunk_storage::build_chunk_kv_records;
pub use chunker::{
    calculate_line_numbers, default_recursive_separators, ingest_chunking_observation,
    ingest_chunking_observation_full, is_html_comment_only, is_mm_chunk_header, make_page_marker,
    markdown_chunk, markdown_pack_enabled, parse_page_marker, pdf_cross_page_pack_enabled,
    pdf_pack_enabled, resolve_chunker, split_into_page_segments, split_preserving_atomic_regions,
    AtomicKind, CharacterBasedChunking, ChunkOptions, ChunkResult, ChunkStrategy, ChunkTokenStats,
    Chunker, ChunkerConfig, ChunkingStrategy, ContentRegion, MarkdownChunking, PageAwareChunking,
    PageMarkerWriter, ParagraphBoundaryChunking, RecursiveCharacterChunking, SectionMetadata,
    SentenceBoundaryChunking, TextChunk, TokenBasedChunking, MARKDOWN_PACK_ENV, PAGE_MARKER_PREFIX,
    PAGE_MARKER_SUFFIX, PDF_CROSS_PAGE_PACK_ENV, PDF_PACK_ENV,
};
pub use error::{
    ChunkExtractionOutcome, ChunkFailure, PipelineError, ResilientExtractionResult, Result,
};
pub use extractor::{
    assign_token_usage, effective_temperature_for_model, extraction_completion_options,
    extraction_completion_options_with_effort, recommended_chunk_size_for_bytes,
    resolve_effective_temperature, resolve_extraction_reasoning_effort,
    structured_extract_effort_floor, ConfigurableEntitySchema, EntityExtractor, ExtractedEntity,
    ExtractedRelationship, ExtractionResult, GleaningConfig, GleaningExtractor, LLMExtractor,
    SOTAExtractor, SimpleExtractor,
};
pub use ingestion_pipeline::{
    build_chunker_config, build_chunker_config_with_policy, build_ingestion_pipeline,
    build_ingestion_pipeline_simple, IngestionPipelineOptions,
};
pub use markdown_ir::{
    extract_markdown_blocks, format_breadcrumb, is_atx_heading_only_text, parse_atx_heading,
    PREFACE_HEADING,
};
pub use structure_induce::{
    induce_faq_markdown, maybe_induce_structure, structure_induce_mode_from_env,
    StructureInduceMode, STRUCTURE_INDUCE_ENV,
};
pub use token_estimator::{
    count_tokens, heuristic_token_count, DefaultTokenEstimator, TokenEstimator,
};
// Re-export unified ingestion types for frontend compatibility
pub use entity_display::{resolve_entity_display_label, soft_label_opaque};
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
    apply_local_merge_async_clamp, apply_source_ids_limit, approx_token_count,
    collect_unique_fragments, decide_description_merge, description_similarity,
    document_id_from_chunk_id, document_ids_from_chunk_ids, force_llm_summary_on_merge_from_env,
    insert_chunk_lineage_properties, insert_document_lineage_properties,
    join_description_fragments, max_source_ids_per_entity_from_env,
    max_source_ids_per_relation_from_env, merge_and_insert_document_lineage, merge_document_ids,
    merge_max_async_from_env, merge_source_ids, parse_max_source_ids, parse_merge_max_async,
    resolve_incoming_document_ids, should_skip_description_update_keep,
    source_chunk_ids_from_properties, source_document_ids_from_properties,
    source_ids_limit_method_from_env, split_description_fragments, summary_max_tokens_from_env,
    truncate_keep_doc_diverse, DescriptionMergeBackend, DescriptionMergeDecision,
    DescriptionMergePolicy, EntityLineageLink, EntitySinkRow, KnowledgeGraphMerger, LineageSink,
    MergeArtifacts, MergePhase, MergeProgress, MergeProgressCallback, MergeStats, MergerConfig,
    NoopEntitySink, NoopLineageSink, RelationLineageLink, RelationalEntitySink,
    RelationshipSinkReport, RelationshipSinkRow, SourceIdsLimitMethod,
    DEFAULT_FORCE_LLM_SUMMARY_ON_MERGE, DEFAULT_MAX_SOURCE_IDS, DEFAULT_MERGE_MAX_ASYNC,
    DEFAULT_SUMMARY_MAX_TOKENS, GRAPH_FIELD_SEP, LOCAL_MERGE_MAX_ASYNC,
};
pub use multimodal::{
    bare_entity_id, inject_modality_relations, map_image_type_to_retrieval_modality,
    parse_drawing_item_locus, parse_mm_display_name, resolve_mm_display_from_node_props,
    resolve_mm_entity_display, resolve_retrieval_modality_from_content,
    stamp_retrieval_modality_on_chunks, DrawingItemKind, DrawingItemLocus, MmChunkSidecarMeta,
    MmDisplayInput, MmDisplayLabel, MmHeadingBlock, MmSidecarBlock, MmSidecarRef, MODALITY_CHART,
    MODALITY_EQUATION, MODALITY_FIGURE, MODALITY_TABLE,
};
pub use persistence::{
    build_chunk_vector_batch, build_relational_chunks, is_injection_composite_document_id,
    persist_processing_result, persist_relational_chunks, resolve_relational_document_id,
    ChunkVectorBuildOptions, DefaultIngestionPersister, IngestionAuthority, IngestionPersistConfig,
    IngestionPersistContext, IngestionPersistOutput, IngestionPersistSettings, IngestionPersister,
};
pub use pipeline::{
    allow_local_gleaning,
    allow_local_high_concurrency,
    apply_local_concurrency_safety_clamp,
    clamp_max_concurrent_extractions,
    clamp_max_gleaning,
    classify_extract_error,
    // Issue-194: configurable timeout / concurrency constants
    default_chunk_timeout_for_provider,
    default_max_concurrent_for_provider,
    extract_retry_budget,
    is_local_extraction_provider,
    is_local_provider_overload_error,
    // SPEC-091 QW2: admission resolver SSOT (LAW-Q1)
    queue_target_wait_secs_from_env,
    resolve_admission_plan,
    resolve_extract_provider_name_for_fairness,
    resolve_extract_provider_name_for_fairness_from,
    resolve_gleaning_for_provider,
    resolve_worker_pool_limits,
    resolve_worker_pool_limits_from,
    retry_delay_ms_for_chunk_error,
    AdmissionPlan,
    ChunkErrorInfo,
    ChunkExtractedCallback,
    ChunkProgressCallback,
    ChunkProgressPhase,
    ChunkProgressUpdate,
    CostBreakdownStats,
    EmbedProgressCallback,
    EmbedProgressUpdate,
    ExtractErrorClass,
    IngestProfile,
    Pipeline,
    PipelineConfig,
    ProcessingResult,
    ProcessingStats,
    ProviderKind,
    ProviderProfile,
    WorkerPoolLimits,
    ALLOW_LOCAL_HIGH_CONCURRENCY_ENV,
    DEFAULT_CHUNK_MAX_RETRIES,
    DEFAULT_CHUNK_TIMEOUT_SECS,
    DEFAULT_INITIAL_RETRY_DELAY_MS,
    DEFAULT_MAX_CONCURRENT_EXTRACTIONS,
    DEFAULT_QUEUE_TARGET_WAIT_SECS,
    LOCAL_CHUNK_TIMEOUT_SECS,
    LOCAL_DEFAULT_LIFECYCLE_TASKS_PER_TENANT,
    LOCAL_ENABLE_GLEANING_ENV,
    LOCAL_MAX_CONCURRENT_EXTRACTIONS,
    LOCAL_MAX_INGEST_TASKS_PER_TENANT_CAP,
    LOCAL_MAX_LIFECYCLE_TASKS_PER_TENANT_CAP,
    LOCAL_OVERLOAD_RETRY_DELAY_MS,
    LOCAL_WORKER_THREADS_CAP,
    MAX_CHUNK_MAX_RETRIES,
    MAX_CONCURRENT_EXTRACTIONS_CAP,
    MAX_GLEANING_CAP,
    MAX_RETRY_DELAY_MS,
    MIN_CHUNK_TIMEOUT_SECS,
    QUEUE_TARGET_WAIT_SECS_ENV,
};
pub use progress::{
    default_model_pricing, CostBreakdown, CostTracker, IngestionError, IngestionProgress,
    IngestionStatus, MessageLevel, ModelPricing, OperationCost, PipelineStage, ProgressMessage,
    ProgressTracker, StageProgress, StageStatus, PHASE_WEIGHTS,
};
pub use prompts::{
    apply_extraction_caps, apply_extraction_caps_with_strategy, canonicalize_extraction_language,
    default_entity_types, detect_format_markers, document_language_override,
    effective_extraction_language, extraction_language_from_metadata, format_section_context,
    is_extraction_language_clear, json_extraction_prompt, json_extraction_prompt_with_caps,
    json_gleaning_prompt, json_gleaning_prompt_with_caps, json_language_instruction,
    normalize_entity_name, resolve_extraction_language, resolve_extraction_language_from_env,
    text_with_section_context, truncate_section_context, with_document_language,
    with_optional_document_language, CapsSelectionStrategy, EntityExtractionPrompts,
    ExtractionCaps, ExtractionResultParser, HybridExtractionParser, JsonExtractionParser,
    SummarizationPrompts, TupleParser, DEFAULT_COMPLETION_DELIMITER, DEFAULT_EXTRACTION_LANGUAGE,
    DEFAULT_MAX_EXTRACTION_ENTITIES, DEFAULT_MAX_EXTRACTION_RECORDS, DEFAULT_TUPLE_DELIMITER,
    EXTRACTION_LANGUAGE_ENV, EXTRACT_CAPS_SELECTION_ENV, META_EXTRACT_MAX_ENTITIES,
    META_EXTRACT_MAX_RECORDS, SUPPORTED_LANGUAGES,
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
pub use text_embedder::LlmTextEmbedder;
pub use validation::{
    validate_document_content, validate_document_filename, DocumentValidator, ValidationCode,
    ValidationConfig, ValidationIssue, ValidationResult,
};
