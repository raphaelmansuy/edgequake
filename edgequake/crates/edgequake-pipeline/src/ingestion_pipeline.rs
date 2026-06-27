//! Shared ingestion pipeline factory (SPEC-025 5.2 / 5.3, SPEC-026 Phase 2 chunk registry).

use std::sync::Arc;

use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};

use crate::adaptive_chunking::{adaptive_chunk_overlap, calculate_adaptive_chunk_size};
use crate::chunker::{ChunkOptions, ChunkStrategy, ChunkerConfig};
use crate::extractor::{EntityExtractor, GleaningConfig, GleaningExtractor, LLMExtractor};
use crate::pipeline::{Pipeline, PipelineConfig};
use crate::prompts::EntityExtractionSchema;

/// Per-document ingestion tuning applied when building a workspace pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestionPipelineOptions {
    pub document_size_bytes: usize,
    pub enable_gleaning: bool,
    pub max_gleaning: usize,
    pub chunk_strategy: ChunkStrategy,
    pub chunk_options: Option<ChunkOptions>,
}

impl IngestionPipelineOptions {
    pub fn from_document_size(document_size_bytes: usize) -> Self {
        Self {
            document_size_bytes,
            enable_gleaning: true,
            max_gleaning: 1,
            chunk_strategy: ChunkStrategy::default(),
            chunk_options: None,
        }
    }

    pub fn with_gleaning(mut self, enabled: bool, max_passes: usize) -> Self {
        self.enable_gleaning = enabled;
        self.max_gleaning = max_passes;
        self
    }

    pub fn with_chunk_strategy(mut self, strategy: ChunkStrategy) -> Self {
        self.chunk_strategy = strategy;
        self
    }

    pub fn with_chunk_options(mut self, options: ChunkOptions) -> Self {
        self.chunk_options = Some(options);
        self
    }
}

/// Build chunker config from document size + optional API overrides.
pub fn build_chunker_config(
    document_size_bytes: usize,
    strategy: ChunkStrategy,
    chunk_options: Option<&ChunkOptions>,
) -> ChunkerConfig {
    let mut chunk_size = calculate_adaptive_chunk_size(document_size_bytes);
    let mut chunk_overlap = adaptive_chunk_overlap(chunk_size);

    // Recursive/markdown use LightRAG nominal 1200 when doc is small (adaptive still caps large docs).
    if strategy != ChunkStrategy::Fixed && document_size_bytes <= 50_000 {
        chunk_size = chunk_size.max(800);
    }

    let mut config = ChunkerConfig {
        chunk_size,
        chunk_overlap,
        ..Default::default()
    };

    if let Some(opts) = chunk_options {
        opts.apply_to_config(&mut config);
    }

    config
}

/// Build a document-scoped ingestion pipeline with adaptive chunking and optional gleaning.
pub fn build_ingestion_pipeline(
    llm: Arc<dyn LLMProvider>,
    embedding: Arc<dyn EmbeddingProvider>,
    entity_schema: EntityExtractionSchema,
    options: IngestionPipelineOptions,
) -> Pipeline {
    let chunker_config = build_chunker_config(
        options.document_size_bytes,
        options.chunk_strategy,
        options.chunk_options.as_ref(),
    );

    tracing::info!(
        doc_size_bytes = options.document_size_bytes,
        chunk_size = chunker_config.chunk_size,
        chunk_overlap = chunker_config.chunk_overlap,
        chunk_strategy = options.chunk_strategy.as_str(),
        enable_gleaning = options.enable_gleaning,
        max_gleaning = options.max_gleaning,
        "Building ingestion pipeline"
    );

    let pipeline_config = PipelineConfig {
        chunker: chunker_config,
        chunk_strategy: options.chunk_strategy,
        ..Default::default()
    };

    let base_extractor: Arc<dyn EntityExtractor> =
        Arc::new(LLMExtractor::new(llm.clone()).with_entity_schema(entity_schema));

    let extractor: Arc<dyn EntityExtractor> = if options.enable_gleaning && options.max_gleaning > 0
    {
        Arc::new(
            GleaningExtractor::new(llm, base_extractor).with_config(GleaningConfig {
                max_gleaning: options.max_gleaning,
                always_glean: false,
            }),
        )
    } else {
        base_extractor
    };

    Pipeline::new(pipeline_config)
        .with_extractor(extractor)
        .with_embedding_provider(embedding)
}

/// Backward-compatible alias.
pub fn build_ingestion_pipeline_simple(
    llm: Arc<dyn LLMProvider>,
    embedding: Arc<dyn EmbeddingProvider>,
    entity_schema: EntityExtractionSchema,
    options: IngestionPipelineOptions,
) -> Pipeline {
    build_ingestion_pipeline(llm, embedding, entity_schema, options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::MockProvider;

    #[test]
    fn large_document_gets_smaller_chunks() {
        let llm = Arc::new(MockProvider::new()) as Arc<dyn LLMProvider>;
        let embedding = Arc::new(MockProvider::new()) as Arc<dyn EmbeddingProvider>;
        let pipeline = build_ingestion_pipeline_simple(
            llm,
            embedding,
            EntityExtractionSchema::server_default(),
            IngestionPipelineOptions::from_document_size(150_000),
        );
        assert_eq!(pipeline.config().chunker.chunk_size, 600);
    }

    #[test]
    fn from_document_size_defaults_to_recursive() {
        let opts = IngestionPipelineOptions::from_document_size(10_000);
        assert_eq!(opts.chunk_strategy, ChunkStrategy::Recursive);
    }

    #[test]
    fn recursive_strategy_wired_in_config() {
        let opts = IngestionPipelineOptions::from_document_size(10_000)
            .with_chunk_strategy(ChunkStrategy::Recursive);
        let cfg = build_chunker_config(10_000, opts.chunk_strategy, None);
        assert!(cfg.chunk_size >= 800);
    }

    #[test]
    fn chunk_options_override_applies_for_recursive() {
        let opts = ChunkOptions {
            chunk_token_size: Some(15),
            chunk_overlap_token_size: Some(0),
            separators: Vec::new(),
        };
        let cfg = build_chunker_config(500, ChunkStrategy::Recursive, Some(&opts));
        assert_eq!(cfg.chunk_size, 15);
    }

    #[tokio::test]
    async fn recursive_with_small_chunk_options_splits_paragraphs() {
        use crate::chunker::{ChunkingStrategy, RecursiveCharacterChunking};
        let text = "Paragraph one has enough text for testing.\n\nParagraph two continues the document with more content.\n\nParagraph three finishes the sample.";
        let opts = ChunkOptions {
            chunk_token_size: Some(15),
            chunk_overlap_token_size: Some(0),
            separators: Vec::new(),
        };
        let cfg = build_chunker_config(text.len(), ChunkStrategy::Recursive, Some(&opts));
        let chunks = RecursiveCharacterChunking
            .chunk(text, &cfg)
            .await
            .unwrap();
        assert!(
            chunks.len() >= 3,
            "expected >=3 chunks with token_size=15, got {}",
            chunks.len()
        );
    }
}
