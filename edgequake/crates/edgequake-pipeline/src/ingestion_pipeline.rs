//! Shared ingestion pipeline factory (SPEC-025 5.2 / 5.3, SPEC-026 Phase 2 chunk registry).

use std::sync::Arc;

use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};

use crate::adaptive_chunking::ChunkingPolicy;
use crate::chunker::{ChunkOptions, ChunkStrategy, ChunkerConfig};
use crate::extraction_mode::ExtractionMode;
use crate::extractor::{EntityExtractor, GleaningConfig, GleaningExtractor, LLMExtractor};
use crate::pipeline::{Pipeline, PipelineConfig};
use crate::prompts::{EntityExtractionSchema, ExtractionCaps};

/// Per-document ingestion tuning applied when building a workspace pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestionPipelineOptions {
    pub document_size_bytes: usize,
    pub enable_gleaning: bool,
    pub max_gleaning: usize,
    pub chunk_strategy: ChunkStrategy,
    pub chunk_options: Option<ChunkOptions>,
    /// Workspace chunking policy (SPEC-116). `None` ≡ Inherit fleet env.
    pub chunking_policy: Option<ChunkingPolicy>,
    /// When true, use `ChunkStrategy::Pdf` unless an explicit strategy overrides it.
    ///
    /// Set by the API processor when the source document is a PDF and its
    /// markdown content contains `<!-- edgequake-page:N -->` markers.
    /// This ensures **no chunk ever crosses a PDF page boundary**.
    pub is_pdf_source: bool,
    /// Workspace extract LLM provider id (`ollama`, `mistral`, …).
    ///
    /// Used to apply local-vs-cloud extraction timeouts/concurrency without
    /// changing cloud quality defaults.
    pub llm_provider: Option<String>,
    /// When true, allow gleaning on local providers (Ollama / LM Studio).
    ///
    /// Local gleaning is off by default — it doubles LLM load and worsens
    /// connection storms. Set via metadata `allow_local_gleaning` or env
    /// `EDGEQUAKE_LOCAL_ENABLE_GLEANING=1`.
    pub allow_local_gleaning: bool,
    /// Natural-language output language for extraction string values (SPEC-096).
    ///
    /// Resolved by callers via `resolve_extraction_language` (workspace → env → English).
    pub extraction_language: String,
    /// Desired extract-role reasoning effort (SPEC-109 / SPEC-113).
    ///
    /// When `None`, extractors apply provider-aware flooring (`none` for Ollama).
    pub reasoning_effort: Option<String>,
    /// SPEC-117: resolved extract caps (`None` ≡ fleet env / 40/100 at build time).
    pub extraction_caps: Option<ExtractionCaps>,
    /// SPEC-160: resolved extraction mode (document > workspace > env > `llm`).
    ///
    /// `build_ingestion_pipeline` builds the LLM extractor only. A caller that
    /// resolves `Decision` must wire a decision extractor itself (LAW-160-4).
    pub extraction_mode: ExtractionMode,
    /// SPEC-160: document-level overrides as sent at upload
    /// (`extraction_mode`, `decision_gate_preset`). `None` ≡ the document set nothing.
    /// The workspace pipeline factory resolves them against workspace and env.
    pub document_extraction: Option<serde_json::Value>,
}

impl IngestionPipelineOptions {
    pub fn from_document_size(document_size_bytes: usize) -> Self {
        Self {
            document_size_bytes,
            enable_gleaning: true,
            max_gleaning: 1,
            chunk_strategy: ChunkStrategy::default(),
            chunk_options: None,
            chunking_policy: None,
            is_pdf_source: false,
            llm_provider: None,
            allow_local_gleaning: false,
            extraction_language: crate::prompts::DEFAULT_EXTRACTION_LANGUAGE.to_string(),
            reasoning_effort: None,
            extraction_caps: None,
            extraction_mode: ExtractionMode::default(),
            document_extraction: None,
        }
    }

    /// Attach the document-level extraction overrides (SPEC-160).
    ///
    /// Reads only `extraction_mode` and `decision_gate_preset`; an object with
    /// neither key is stored as `None`.
    pub fn with_document_extraction_from(mut self, metadata: &serde_json::Value) -> Self {
        let keep = [
            crate::extraction_mode::META_EXTRACTION_MODE,
            crate::extractor::decision::gate::META_DECISION_GATE_PRESET,
        ];
        let picked: serde_json::Map<String, serde_json::Value> = keep
            .iter()
            .filter_map(|k| metadata.get(*k).map(|v| (k.to_string(), v.clone())))
            .collect();
        self.document_extraction =
            (!picked.is_empty()).then_some(serde_json::Value::Object(picked));
        self
    }

    /// Set the resolved extraction mode (SPEC-160).
    pub fn with_extraction_mode(mut self, mode: ExtractionMode) -> Self {
        self.extraction_mode = mode;
        self
    }

    /// Set workspace chunking policy (SPEC-116). Document `chunk_options` still win last.
    pub fn with_chunking_policy(mut self, policy: ChunkingPolicy) -> Self {
        self.chunking_policy = Some(policy);
        self
    }

    /// Set resolved extract caps (SPEC-117).
    pub fn with_extraction_caps(mut self, caps: ExtractionCaps) -> Self {
        self.extraction_caps = Some(caps);
        self
    }

    /// Set extraction output language (SPEC-096).
    pub fn with_extraction_language(mut self, language: impl Into<String>) -> Self {
        self.extraction_language = language.into();
        self
    }

    /// Bind the extract-role provider so pipeline knobs can be provider-aware.
    pub fn with_llm_provider(mut self, provider: impl Into<String>) -> Self {
        self.llm_provider = Some(provider.into());
        self
    }

    /// Set extract-role reasoning effort (e.g. `"none"` for Ollama think-off).
    pub fn with_reasoning_effort(mut self, effort: Option<String>) -> Self {
        self.reasoning_effort = effort
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        self
    }

    /// Mark this document as a PDF source so the Pdf chunking strategy is
    /// selected automatically (can still be overridden with `with_chunk_strategy`).
    pub fn for_pdf(mut self) -> Self {
        self.is_pdf_source = true;
        if self.chunk_strategy == ChunkStrategy::Recursive
            || self.chunk_strategy == ChunkStrategy::Fixed
        {
            self.chunk_strategy = ChunkStrategy::Pdf;
        }
        self
    }

    pub fn with_gleaning(mut self, enabled: bool, max_passes: usize) -> Self {
        self.enable_gleaning = enabled;
        self.max_gleaning = max_passes;
        self
    }

    /// Opt in to gleaning when the extract provider is local (Ollama / LM Studio).
    pub fn with_allow_local_gleaning(mut self, allow: bool) -> Self {
        self.allow_local_gleaning = allow;
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

/// Build chunker config from document size + optional workspace policy + API overrides.
///
/// Precedence (SPEC-116 LAW-116-2):
/// 1. Workspace [`ChunkingPolicy`] (or Inherit → fleet env)
/// 2. Small-doc floor when effective adaptive + non-Fixed strategy + ≤50KB → max(800)
/// 3. Document `ChunkOptions` last
pub fn build_chunker_config(
    document_size_bytes: usize,
    strategy: ChunkStrategy,
    chunk_options: Option<&ChunkOptions>,
) -> ChunkerConfig {
    build_chunker_config_with_policy(document_size_bytes, strategy, None, chunk_options)
}

/// Like [`build_chunker_config`] with explicit workspace [`ChunkingPolicy`].
pub fn build_chunker_config_with_policy(
    document_size_bytes: usize,
    strategy: ChunkStrategy,
    chunking_policy: Option<&ChunkingPolicy>,
    chunk_options: Option<&ChunkOptions>,
) -> ChunkerConfig {
    use crate::adaptive_chunking::{
        policy_uses_adaptive, resolve_base_chunk_size_overlap_with_policy,
    };

    let (mut chunk_size, chunk_overlap) =
        resolve_base_chunk_size_overlap_with_policy(document_size_bytes, chunking_policy);

    // Recursive/markdown/pdf floor when adaptive (legacy LightRAG small-doc path).
    if policy_uses_adaptive(chunking_policy)
        && strategy != ChunkStrategy::Fixed
        && document_size_bytes <= 50_000
    {
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

/// Chunking and extraction knobs shared by every extraction mode (DRY).
fn ingestion_pipeline_config(options: &IngestionPipelineOptions) -> PipelineConfig {
    let chunker_config = build_chunker_config_with_policy(
        options.document_size_bytes,
        options.chunk_strategy,
        options.chunking_policy.as_ref(),
        options.chunk_options.as_ref(),
    );
    let provider = options.llm_provider.as_deref().unwrap_or("");
    PipelineConfig {
        chunker: chunker_config,
        chunk_strategy: options.chunk_strategy,
        ..PipelineConfig::from_env_for_provider(provider)
    }
}

/// Build a pipeline whose extractor the caller supplies (SPEC-160 decision mode).
///
/// No chat LLM is involved: the only model call is the extractor's, so text
/// does not leave the decision backend (LAW-160-3). Gleaning does not apply:
/// a closed decision has no "missed entity" pass.
///
/// Set `options.llm_provider` to the extractor's provider name so local
/// concurrency and timeout profiles apply.
pub fn build_pipeline_with_extractor(
    extractor: Arc<dyn EntityExtractor>,
    embedding: Arc<dyn EmbeddingProvider>,
    options: IngestionPipelineOptions,
) -> Pipeline {
    let pipeline_config = ingestion_pipeline_config(&options);
    tracing::info!(
        doc_size_bytes = options.document_size_bytes,
        chunk_size = pipeline_config.chunker.chunk_size,
        chunk_strategy = options.chunk_strategy.as_str(),
        extractor = extractor.name(),
        extractor_model = extractor.model_name(),
        chunk_timeout_secs = pipeline_config.chunk_extraction_timeout_secs,
        max_concurrent_extractions = pipeline_config.max_concurrent_extractions,
        "Building ingestion pipeline with a custom extractor"
    );
    Pipeline::new(pipeline_config)
        .with_extractor(extractor)
        .with_embedding_provider(embedding)
}

/// SPEC-180: build the pipeline of a `normal` workspace: chunk, embed, nothing else.
///
/// There is no extractor and no chat LLM, so no text leaves for a cloud model at ingest
/// (LAW-180-3). The shape is fixed here, not read from `EDGEQUAKE_INGEST_PROFILE`: that
/// variable is a global eval switch and must never decide a workspace's behavior
/// (EC-180-31). Gleaning and extraction caps do not apply.
pub fn build_chunk_only_pipeline(
    embedding: Arc<dyn EmbeddingProvider>,
    options: IngestionPipelineOptions,
) -> Pipeline {
    let mut pipeline_config = ingestion_pipeline_config(&options);
    crate::pipeline::IngestProfile::ChunkOnly.apply_to(&mut pipeline_config);
    tracing::info!(
        doc_size_bytes = options.document_size_bytes,
        chunk_size = pipeline_config.chunker.chunk_size,
        chunk_strategy = options.chunk_strategy.as_str(),
        "Building chunk-only (normal RAG) ingestion pipeline"
    );
    Pipeline::new(pipeline_config).with_embedding_provider(embedding)
}

/// Build a document-scoped ingestion pipeline with adaptive chunking and optional gleaning.
pub fn build_ingestion_pipeline(
    llm: Arc<dyn LLMProvider>,
    embedding: Arc<dyn EmbeddingProvider>,
    entity_schema: EntityExtractionSchema,
    options: IngestionPipelineOptions,
) -> Pipeline {
    let provider = options.llm_provider.as_deref().unwrap_or("");
    let pipeline_config = ingestion_pipeline_config(&options);

    let (enable_gleaning, max_gleaning) = crate::pipeline::resolve_gleaning_for_provider(
        provider,
        options.enable_gleaning,
        options.max_gleaning,
        options.allow_local_gleaning,
    );
    if options.enable_gleaning
        && !enable_gleaning
        && crate::pipeline::is_local_extraction_provider(provider)
    {
        tracing::info!(
            llm_provider = provider,
            "Disabled gleaning for local LLM to reduce Ollama load; set {}=1 or allow_local_gleaning to opt in",
            crate::pipeline::LOCAL_ENABLE_GLEANING_ENV
        );
    }

    tracing::info!(
        doc_size_bytes = options.document_size_bytes,
        chunk_size = pipeline_config.chunker.chunk_size,
        chunk_overlap = pipeline_config.chunker.chunk_overlap,
        chunk_strategy = options.chunk_strategy.as_str(),
        enable_gleaning = enable_gleaning,
        max_gleaning = max_gleaning,
        extraction_language = %options.extraction_language,
        llm_provider = provider,
        is_local_extraction = crate::pipeline::is_local_extraction_provider(provider),
        reasoning_effort = options.reasoning_effort.as_deref().unwrap_or("(floor)"),
        chunk_timeout_secs = pipeline_config.chunk_extraction_timeout_secs,
        max_concurrent_extractions = pipeline_config.max_concurrent_extractions,
        ollama_context_length = %std::env::var("OLLAMA_CONTEXT_LENGTH").unwrap_or_else(|_| "(unset)".into()),
        "Building ingestion pipeline"
    );

    let language = options.extraction_language.clone();
    let effort = options.reasoning_effort.clone();
    let caps = options
        .extraction_caps
        .unwrap_or_else(ExtractionCaps::from_env);
    tracing::info!(
        max_entities = caps.max_entities,
        max_total_records = caps.max_total_records,
        "Resolved extraction caps for ingestion pipeline"
    );
    let base_extractor: Arc<dyn EntityExtractor> = Arc::new(
        LLMExtractor::new(llm.clone())
            .with_entity_schema(entity_schema.clone())
            .with_language(language.clone())
            .with_reasoning_effort(effort.clone())
            .with_extraction_caps(caps),
    );

    let extractor: Arc<dyn EntityExtractor> = if enable_gleaning && max_gleaning > 0 {
        Arc::new(
            GleaningExtractor::new(llm, base_extractor)
                .with_entity_schema(entity_schema)
                .with_language(language)
                .with_reasoning_effort(effort)
                .with_extraction_caps(caps)
                .with_config(GleaningConfig { max_gleaning }),
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

    // T-160-U67 — only the two SPEC-160 keys travel; other metadata is dropped.
    #[test]
    fn document_extraction_keeps_two_keys() {
        let meta = serde_json::json!({
            "extraction_mode": "decision",
            "decision_gate_preset": "recall",
            "title": "x"
        });
        let o =
            IngestionPipelineOptions::from_document_size(10).with_document_extraction_from(&meta);
        let kept = o.document_extraction.unwrap();
        assert_eq!(kept.as_object().unwrap().len(), 2);
        assert!(kept.get("title").is_none());
        let none = IngestionPipelineOptions::from_document_size(10)
            .with_document_extraction_from(&serde_json::json!({"title": "x"}));
        assert!(none.document_extraction.is_none());
    }

    #[test]
    fn large_document_gets_smaller_chunks() {
        // WHY: adaptive_chunk_size(150_000) = 600, but MockProvider.max_tokens() = 512
        // so with_embedding_provider caps chunk_size to max(512/2, 100) = 256.
        // This test documents the full pipeline behaviour including the embedding cap.
        let llm = Arc::new(MockProvider::new()) as Arc<dyn LLMProvider>;
        let embedding = Arc::new(MockProvider::new()) as Arc<dyn EmbeddingProvider>;
        let pipeline = build_ingestion_pipeline_simple(
            llm,
            embedding,
            EntityExtractionSchema::server_default(),
            IngestionPipelineOptions::from_document_size(150_000),
        );
        // MockProvider.max_tokens() = 512 → max_safe = 256; adaptive would be 600
        assert_eq!(
            pipeline.config().chunker.chunk_size,
            256,
            "Expected chunk_size to be capped at 256 by MockProvider.max_tokens()=512"
        );
    }

    // T-160-U11 — options default to the LLM mode and carry an explicit mode.
    #[test]
    fn spec160_options_default_to_llm_and_carry_mode() {
        let opts = IngestionPipelineOptions::from_document_size(1_000);
        assert_eq!(opts.extraction_mode, ExtractionMode::Llm);
        let opts = opts.with_extraction_mode(ExtractionMode::Decision);
        assert_eq!(opts.extraction_mode, ExtractionMode::Decision);
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
        let chunks = RecursiveCharacterChunking.chunk(text, &cfg).await.unwrap();
        assert!(
            chunks.len() >= 3,
            "expected >=3 chunks with token_size=15, got {}",
            chunks.len()
        );
    }

    #[test]
    fn spec096_pipeline_factory_wires_language() {
        let opts = IngestionPipelineOptions::from_document_size(1_000)
            .with_extraction_language("Korean")
            .with_gleaning(false, 0);
        assert_eq!(opts.extraction_language, "Korean");
        let llm = Arc::new(MockProvider::new()) as Arc<dyn LLMProvider>;
        let embedding = Arc::new(MockProvider::new()) as Arc<dyn EmbeddingProvider>;
        // Smoke: factory accepts language option without panic.
        let _pipeline = build_ingestion_pipeline(
            llm,
            embedding,
            EntityExtractionSchema::server_default(),
            opts,
        );
        let prompt = crate::prompts::json_extraction_prompt(
            "Seoul is the capital.",
            &EntityExtractionSchema::server_default(),
            "Korean",
        );
        assert!(prompt.contains("Korean"));
        assert!(prompt.contains("Output Language"));
    }
}
