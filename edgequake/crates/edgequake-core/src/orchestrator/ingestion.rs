//! Document ingestion operations for EdgeQuake.
//!
//! Contains `insert()`, `insert_batch()`, and adaptive chunk size calculation.

use std::sync::Arc;

use edgequake_pipeline::{
    GleaningConfig, GleaningExtractor, KnowledgeGraphMerger, LLMExtractor, LLMSummarizer,
    MergerConfig, Pipeline, PipelineConfig, SummarizerConfig,
};

use crate::error::{Error, Result};
use crate::types::InsertResult;

use super::EdgeQuake;

/// Calculate adaptive chunk size based on document length.
///
/// WHY: Large documents need smaller chunks to avoid LLM timeouts and ensure reliable processing.
///
/// Based on LightRAG research:
/// - Default: 1200 tokens for normal documents
/// - Quality mode: 1500 tokens (maximum)
/// - Large documents: 600-800 tokens for better reliability
///
/// # Arguments
///
/// * `document_size_bytes` - Size of the document in bytes
///
/// # Returns
///
/// Recommended chunk size in tokens
///
/// # Examples
///
/// ```ignore
/// // Internal function - not part of public API
/// let chunk_size = calculate_adaptive_chunk_size(30_000);  // 30KB → 1200 tokens
/// let chunk_size = calculate_adaptive_chunk_size(80_000);  // 80KB → 800 tokens
/// let chunk_size = calculate_adaptive_chunk_size(200_000); // 200KB → 600 tokens
/// ```
fn calculate_adaptive_chunk_size(document_size_bytes: usize) -> usize {
    // Based on LightRAG best practices and empirical testing:
    // - Small documents (<50KB): Use standard 1200 tokens
    // - Medium documents (50-100KB): Use reduced 800 tokens
    // - Large documents (>100KB): Use minimal 600 tokens
    //
    // WHY these thresholds:
    // - 50KB ≈ 12,500 tokens → ~10 chunks at 1200 tokens (manageable)
    // - 100KB ≈ 25,000 tokens → ~31 chunks at 800 tokens (reasonable)
    // - 150KB ≈ 37,500 tokens → ~62 chunks at 600 tokens (many but necessary)
    //
    // Smaller chunks for large documents reduce:
    // 1. LLM timeout risk (less context per request)
    // 2. Entity extraction complexity (focused scope)
    // 3. Memory pressure (smaller batches)
    if document_size_bytes > 100_000 {
        600 // >100KB: minimal chunks for reliability
    } else if document_size_bytes > 50_000 {
        800 // 50-100KB: reduced chunks
    } else {
        1200 // <50KB: standard LightRAG default
    }
}

impl EdgeQuake {
    /// Insert a document for processing.
    ///
    /// # Implements
    ///
    /// - **FEAT0001**: Document Ingestion
    /// - **FEAT0002**: Text Chunking with Overlap
    /// - **FEAT0003**: LLM-Based Entity Extraction
    /// - **FEAT0005**: Knowledge Graph Construction
    /// - **FEAT0006**: Vector Embedding Generation
    ///
    /// # Enforces
    ///
    /// - **BR0001**: Document ID must be unique (error on duplicate)
    /// - **BR0002**: Chunk overlap < chunk size (validated in pipeline)
    /// - **BR0003**: Entity names normalized to UPPERCASE_UNDERSCORED
    ///
    /// # WHY: 3-Stage Pipeline Architecture
    ///
    /// The insert flow follows a 3-stage architecture (similar to LightRAG):
    ///
    /// 1. **Pipeline Processing** - Chunking → Entity Extraction → Embedding
    ///    - WHY chunks: LLM context windows are limited; chunks enable parallel processing
    ///    - WHY overlapping chunks: Entities spanning chunk boundaries are captured
    ///
    /// 2. **Knowledge Graph Merge** - Deduplicate and merge into graph storage
    ///    - WHY merge instead of insert: Same entity may appear in multiple documents
    ///    - WHY LLM summarization: Merge conflicting descriptions intelligently
    ///    - WHY source tracking: Enable cascade delete when documents are removed
    ///
    /// 3. **Vector Storage** - Store embeddings for semantic search
    ///    - WHY type metadata: Distinguish entity vectors from chunk vectors
    ///    - WHY tenant isolation: Multi-tenancy requires vector filtering
    ///
    /// # Arguments
    ///
    /// * `content` - Raw text content to process
    /// * `document_id` - Optional document ID; auto-generated UUID if not provided
    ///
    /// # Returns
    ///
    /// [`InsertResult`] with processing statistics (chunks, entities, relationships)
    ///
    /// # Errors
    ///
    /// - `Error::not_initialized` if EdgeQuake not initialized
    /// - `Error::internal` if pipeline or storage operations fail
    pub async fn insert(&self, content: &str, document_id: Option<&str>) -> Result<InsertResult> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        // Edge case: Empty document
        // WHY: Skip processing empty content to save resources
        let content_trimmed = content.trim();
        if content_trimmed.is_empty() {
            let doc_id = document_id
                .map(String::from)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            tracing::warn!(
                doc_id = %doc_id,
                "Skipping empty document - no content to process"
            );

            return Ok(InsertResult {
                document_id: doc_id,
                success: true,
                chunks_created: 0,
                entities_extracted: 0,
                relationships_extracted: 0,
                processing_time_ms: 0,
                error: None,
            });
        }

        // Edge case: Extremely large document (>10MB)
        // WHY: Documents over 10MB are likely to cause OOM or extreme timeouts
        const MAX_DOCUMENT_SIZE_BYTES: usize = 10 * 1024 * 1024; // 10MB
        if content.len() > MAX_DOCUMENT_SIZE_BYTES {
            let doc_id = document_id
                .map(String::from)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            let size_mb = content.len() as f64 / (1024.0 * 1024.0);
            tracing::error!(
                doc_id = %doc_id,
                size_bytes = content.len(),
                size_mb = %format!("{:.2}", size_mb),
                max_size_mb = MAX_DOCUMENT_SIZE_BYTES / (1024 * 1024),
                "Document exceeds maximum size limit"
            );

            return Err(Error::validation(format!(
                "Document too large: {:.2}MB. Maximum allowed: {}MB. \
                Please split the document into smaller files.",
                size_mb,
                MAX_DOCUMENT_SIZE_BYTES / (1024 * 1024)
            )));
        }

        let doc_id = document_id
            .map(String::from)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let start = std::time::Instant::now();

        // Calculate adaptive chunk size based on document length
        // WHY: Large documents need smaller chunks to avoid LLM timeouts
        // Based on LightRAG research: 1200 tokens optimal for <50KB, scale down for larger docs
        let doc_size_bytes = content.len();
        let adaptive_chunk_size = calculate_adaptive_chunk_size(doc_size_bytes);
        let adaptive_overlap = (adaptive_chunk_size as f32 * 0.083) as usize; // ~8% overlap (LightRAG best practice)
        let doc_size_kb = doc_size_bytes / 1024;

        tracing::info!(
            doc_id = %doc_id,
            doc_size_bytes = doc_size_bytes,
            doc_size_kb = doc_size_kb,
            adaptive_chunk_size = adaptive_chunk_size,
            adaptive_overlap = adaptive_overlap,
            default_chunk_size = self.config.chunk_token_size,
            "Using adaptive chunking for document ingestion"
        );

        // Create pipeline with adaptive configuration
        // WHY: Per-document pipeline allows dynamic chunk sizing
        // WHY not reuse stored pipeline: Stored pipeline uses static config
        let pipeline_config = PipelineConfig {
            chunker: edgequake_pipeline::ChunkerConfig {
                chunk_size: adaptive_chunk_size,
                chunk_overlap: adaptive_overlap,
                ..Default::default()
            },
            ..Default::default()
        };

        let llm = self
            .llm_provider
            .as_ref()
            .ok_or_else(|| Error::config("LLM provider not set"))?;

        let embedding = self
            .embedding_provider
            .as_ref()
            .ok_or_else(|| Error::config("Embedding provider not set"))?;

        // Create base extractor
        let base_extractor: Arc<dyn edgequake_pipeline::EntityExtractor> = Arc::new(
            LLMExtractor::new(llm.clone()).with_entity_types(self.config.entity_types.clone()),
        );

        // Wrap with GleaningExtractor if enabled
        let extractor: Arc<dyn edgequake_pipeline::EntityExtractor> = if self.config.enable_gleaning
            && self.config.max_gleaning > 0
        {
            Arc::new(
                GleaningExtractor::new(llm.clone(), base_extractor).with_config(GleaningConfig {
                    max_gleaning: self.config.max_gleaning,
                    always_glean: false,
                }),
            )
        } else {
            base_extractor
        };

        let pipeline = Pipeline::new(pipeline_config)
            .with_extractor(extractor)
            .with_embedding_provider(embedding.clone());

        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Graph storage not initialized"))?;

        let vector_storage = self
            .vector_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Vector storage not initialized"))?;

        // Stage 1: Process document through pipeline (Chunking → Extraction → Embedding)
        // WHY: Transforms raw text into structured knowledge graph elements
        let processing_result = pipeline
            .process(&doc_id, content)
            .await
            .map_err(|e| Error::internal(format!("Pipeline error: {}", e)))?;

        // Stage 2: Merge results into knowledge graph
        // WHY: Entities may exist from previous documents; merge avoids duplicates
        // WHY LLM summarization: When merging descriptions, LLM produces coherent summary
        let llm = self
            .llm_provider
            .as_ref()
            .ok_or_else(|| Error::not_initialized("LLM provider not initialized"))?;

        let merger_config = MergerConfig {
            use_llm_summarization: self.config.use_llm_summarization,
            ..Default::default()
        };

        let mut merger =
            KnowledgeGraphMerger::new(merger_config, graph_storage.clone(), vector_storage.clone())
                .with_tenant_context(
                    self.config.tenant_id.clone(),
                    self.config.workspace_id.clone(),
                );

        // Add LLM summarizer if enabled
        if self.config.use_llm_summarization {
            let summarizer = Arc::new(LLMSummarizer::new(llm.clone(), SummarizerConfig::default()));
            merger = merger.with_summarizer(summarizer);
        }

        let merge_stats = merger
            .merge(processing_result.extractions.clone())
            .await
            .map_err(|e| Error::internal(format!("Merge error: {}", e)))?;

        // Stage 3: Store chunk embeddings with type metadata
        // WHY type: "chunk" metadata: Enables filtering entity vs chunk vectors at query time
        // WHY tenant/workspace: Multi-tenancy isolation at vector level
        for chunk in &processing_result.chunks {
            if let Some(embedding) = &chunk.embedding {
                let mut metadata = serde_json::json!({
                    "type": "chunk",  // Mark as chunk for retrieval filtering
                    "document_id": doc_id,
                    "index": chunk.index,
                    "content": chunk.content
                });

                // Add tenant and workspace IDs if present
                if let Some(tenant_id) = &self.config.tenant_id {
                    metadata["tenant_id"] = serde_json::json!(tenant_id);
                }
                if let Some(workspace_id) = &self.config.workspace_id {
                    metadata["workspace_id"] = serde_json::json!(workspace_id);
                }

                vector_storage
                    .upsert(&[(chunk.id.clone(), embedding.clone(), metadata)])
                    .await
                    .map_err(|e| Error::internal(format!("Vector storage error: {}", e)))?;
            }
        }

        let processing_time_ms = start.elapsed().as_millis() as u64;

        Ok(InsertResult {
            document_id: doc_id,
            success: true,
            chunks_created: processing_result.stats.chunk_count,
            entities_extracted: merge_stats.entities_created + merge_stats.entities_updated,
            relationships_extracted: merge_stats.relationships_created
                + merge_stats.relationships_updated,
            processing_time_ms,
            error: None,
        })
    }

    /// Insert multiple documents.
    pub async fn insert_batch(
        &self,
        documents: Vec<(&str, Option<&str>)>,
    ) -> Result<Vec<InsertResult>> {
        let mut results = Vec::with_capacity(documents.len());

        for (content, doc_id) in documents {
            let result = self.insert(content, doc_id).await?;
            results.push(result);
        }

        Ok(results)
    }
}
