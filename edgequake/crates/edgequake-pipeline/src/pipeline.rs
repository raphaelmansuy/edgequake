//! Document processing pipeline.

use std::sync::Arc;

use edgequake_llm::traits::EmbeddingProvider;
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};

use crate::chunker::{Chunker, ChunkerConfig, TextChunk};
use crate::error::Result;
use crate::extractor::{EntityExtractor, ExtractionResult};
use crate::lineage::{DocumentLineage, ExtractionMetadata, LineageBuilder, SourceSpan};

/// Pipeline configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Chunking configuration.
    pub chunker: ChunkerConfig,

    /// Batch size for LLM extraction.
    pub extraction_batch_size: usize,

    /// Batch size for embedding generation.
    pub embedding_batch_size: usize,

    /// Whether to enable entity extraction.
    pub enable_entity_extraction: bool,

    /// Whether to enable relationship extraction.
    pub enable_relationship_extraction: bool,

    /// Whether to generate chunk embeddings.
    pub enable_chunk_embeddings: bool,

    /// Whether to generate entity embeddings.
    pub enable_entity_embeddings: bool,

    /// Whether to generate relationship embeddings.
    pub enable_relationship_embeddings: bool,

    /// Maximum concurrent extraction tasks.
    pub max_concurrent_extractions: usize,

    /// Whether to track document lineage.
    pub enable_lineage_tracking: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            chunker: ChunkerConfig::default(),
            extraction_batch_size: 10,
            embedding_batch_size: 100,
            enable_entity_extraction: true,
            enable_relationship_extraction: true,
            enable_chunk_embeddings: true,
            enable_entity_embeddings: true,
            enable_relationship_embeddings: true,
            max_concurrent_extractions: 4,
            enable_lineage_tracking: false,
        }
    }
}

/// Result of processing a document through the pipeline.
#[derive(Debug, Clone)]
pub struct ProcessingResult {
    /// Document ID.
    pub document_id: String,

    /// Generated chunks.
    pub chunks: Vec<TextChunk>,

    /// Extraction results per chunk.
    pub extractions: Vec<ExtractionResult>,

    /// Processing statistics.
    pub stats: ProcessingStats,

    /// Document lineage tracking (optional).
    pub lineage: Option<DocumentLineage>,
}

/// Statistics from pipeline processing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessingStats {
    /// Number of chunks created.
    pub chunk_count: usize,

    /// Number of entities extracted.
    pub entity_count: usize,

    /// Number of relationships extracted.
    pub relationship_count: usize,

    /// Processing time in milliseconds.
    pub processing_time_ms: u64,

    /// Number of LLM calls made.
    pub llm_calls: usize,

    /// Total tokens used.
    pub total_tokens: usize,

    /// LLM model used for entity extraction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,

    /// Embedding model used for vector embeddings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,

    /// Embedding dimensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_dimensions: Option<usize>,

    /// Entity types extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_types: Option<Vec<String>>,

    /// Relationship types extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship_types: Option<Vec<String>>,

    /// Keywords extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    /// Chunking strategy used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking_strategy: Option<String>,

    /// Average chunk size in characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_chunk_size: Option<usize>,
}

/// Document processing pipeline.
pub struct Pipeline {
    config: PipelineConfig,
    chunker: Chunker,
    extractor: Option<Arc<dyn EntityExtractor>>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
}

impl Pipeline {
    /// Create a new pipeline with the given configuration.
    pub fn new(config: PipelineConfig) -> Self {
        let chunker = Chunker::new(config.chunker.clone());

        Self {
            config,
            chunker,
            extractor: None,
            embedding_provider: None,
        }
    }

    /// Create a pipeline with default configuration.
    pub fn default_pipeline() -> Self {
        Self::new(PipelineConfig::default())
    }

    /// Set the entity extractor.
    pub fn with_extractor(mut self, extractor: Arc<dyn EntityExtractor>) -> Self {
        self.extractor = Some(extractor);
        self
    }

    /// Set the embedding provider.
    pub fn with_embedding_provider(mut self, provider: Arc<dyn EmbeddingProvider>) -> Self {
        self.embedding_provider = Some(provider);
        self
    }

    /// Extract entities from chunks in parallel using a semaphore.
    async fn extract_parallel(
        &self,
        chunks: &[TextChunk],
        extractor: &Arc<dyn EntityExtractor>,
    ) -> Result<Vec<ExtractionResult>> {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(
            self.config.max_concurrent_extractions,
        ));

        // Create futures for all chunks
        let futures: Vec<_> = chunks
            .iter()
            .map(|chunk| {
                let semaphore = semaphore.clone();
                let extractor = extractor.clone();
                let chunk = chunk.clone();

                async move {
                    // Acquire permit (released on drop)
                    let _permit = semaphore
                        .acquire()
                        .await
                        .map_err(|e| crate::error::PipelineError::ExtractionError(e.to_string()))?;

                    extractor.extract(&chunk).await
                }
            })
            .collect();

        // Execute concurrently with buffer to respect semaphore
        let results: Vec<Result<ExtractionResult>> = stream::iter(futures)
            .buffer_unordered(self.config.max_concurrent_extractions)
            .collect()
            .await;

        // Collect results, propagating first error
        results.into_iter().collect()
    }

    /// Process a document through the pipeline.
    pub async fn process(&self, document_id: &str, content: &str) -> Result<ProcessingResult> {
        let start = std::time::Instant::now();
        let mut stats = ProcessingStats::default();

        // Step 1: Chunk the document
        let mut chunks = self.chunker.chunk(content, document_id)?;
        stats.chunk_count = chunks.len();

        // Track chunking strategy and average chunk size
        stats.chunking_strategy =
            Some(format!("sliding_window_{}", self.config.chunker.chunk_size));
        if !chunks.is_empty() {
            let total_chars: usize = chunks.iter().map(|c| c.content.len()).sum();
            stats.avg_chunk_size = Some(total_chars / chunks.len());
        }

        // Step 2: Extract entities and relationships
        let mut extractions = Vec::new();
        let mut entity_types_set = std::collections::HashSet::new();
        let mut relationship_types_set = std::collections::HashSet::new();
        let mut keywords_set = std::collections::HashSet::new();
        let mut total_input_tokens = 0usize;
        let mut total_output_tokens = 0usize;

        if self.config.enable_entity_extraction || self.config.enable_relationship_extraction {
            if let Some(extractor) = &self.extractor {
                // Capture LLM model name
                stats.llm_model = Some(extractor.model_name().to_string());

                // Use parallel extraction for better performance
                extractions = self.extract_parallel(&chunks, extractor).await?;

                // Aggregate statistics from all extractions
                for extraction in &extractions {
                    stats.entity_count += extraction.entities.len();
                    stats.relationship_count += extraction.relationships.len();
                    stats.llm_calls += 1;
                    total_input_tokens += extraction.input_tokens;
                    total_output_tokens += extraction.output_tokens;

                    // Collect unique entity types
                    for entity in &extraction.entities {
                        entity_types_set.insert(entity.entity_type.clone());
                    }

                    // Collect unique relationship types and keywords
                    for rel in &extraction.relationships {
                        relationship_types_set.insert(rel.relation_type.clone());
                        for keyword in &rel.keywords {
                            keywords_set.insert(keyword.clone());
                        }
                    }
                }

                stats.total_tokens = total_input_tokens + total_output_tokens;
            }
        }

        // Store collected types and keywords
        if !entity_types_set.is_empty() {
            stats.entity_types = Some(entity_types_set.into_iter().collect());
        }
        if !relationship_types_set.is_empty() {
            stats.relationship_types = Some(relationship_types_set.into_iter().collect());
        }
        if !keywords_set.is_empty() {
            let mut keywords: Vec<String> = keywords_set.into_iter().collect();
            keywords.sort();
            // Limit to top 50 keywords
            keywords.truncate(50);
            stats.keywords = Some(keywords);
        }

        // Step 3: Generate embeddings
        if let Some(provider) = &self.embedding_provider {
            // Capture embedding model info
            stats.embedding_model = Some(provider.model().to_string());
            stats.embedding_dimensions = Some(provider.dimension());

            // Chunk embeddings
            if self.config.enable_chunk_embeddings {
                let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
                if !texts.is_empty() {
                    let embeddings = provider
                        .embed(&texts)
                        .await
                        .map_err(|e| crate::error::PipelineError::EmbeddingError(e.to_string()))?;

                    for (chunk, embedding) in chunks.iter_mut().zip(embeddings) {
                        chunk.embedding = Some(embedding);
                    }
                }
            }

            // Entity embeddings
            if self.config.enable_entity_embeddings {
                for extraction in &mut extractions {
                    let entity_texts: Vec<String> = extraction
                        .entities
                        .iter()
                        .map(|e| format!("{}: {}", e.name, e.description))
                        .collect();

                    if !entity_texts.is_empty() {
                        let embeddings = provider.embed(&entity_texts).await.map_err(|e| {
                            crate::error::PipelineError::EmbeddingError(e.to_string())
                        })?;

                        for (entity, embedding) in extraction.entities.iter_mut().zip(embeddings) {
                            entity.embedding = Some(embedding);
                        }
                    }
                }
            }

            // Relationship embeddings (as per LightRAG spec)
            if self.config.enable_relationship_embeddings {
                for extraction in &mut extractions {
                    let relationship_texts: Vec<String> = extraction
                        .relationships
                        .iter()
                        .map(|r| {
                            // Format: "keywords\tsource->target\ndescription"
                            // Matches LightRAG's relationship embedding format
                            format!(
                                "{}\t{}->{}\n{}",
                                r.keywords.join(", "),
                                r.source,
                                r.target,
                                r.description
                            )
                        })
                        .collect();

                    if !relationship_texts.is_empty() {
                        let embeddings =
                            provider.embed(&relationship_texts).await.map_err(|e| {
                                crate::error::PipelineError::EmbeddingError(e.to_string())
                            })?;

                        for (relationship, embedding) in
                            extraction.relationships.iter_mut().zip(embeddings)
                        {
                            relationship.embedding = Some(embedding);
                        }
                    }
                }
            }
        }

        stats.processing_time_ms = start.elapsed().as_millis() as u64;

        // Step 4: Build lineage if enabled
        let lineage = if self.config.enable_lineage_tracking {
            let job_id = uuid::Uuid::new_v4().to_string();
            let mut builder = LineageBuilder::new(document_id, document_id, &job_id);

            // Record chunks with their line numbers
            for chunk in &chunks {
                let metadata =
                    ExtractionMetadata::new(stats.llm_model.as_deref().unwrap_or("unknown"));
                builder.record_chunk(
                    &chunk.id,
                    chunk.index,
                    chunk.start_line,
                    chunk.end_line,
                    chunk.start_offset,
                    chunk.end_offset,
                    metadata,
                );
            }

            // Record entities and relationships from extractions
            for extraction in &extractions {
                for entity in &extraction.entities {
                    let entity_id = format!("{}_{}", extraction.source_chunk_id, entity.name);
                    let span = SourceSpan::new(0, 0, 0, 0); // Detailed span would require chunk info
                    builder.record_entity(
                        &entity_id,
                        &entity.name,
                        &extraction.source_chunk_id,
                        span,
                        &entity.description,
                    );
                }

                for rel in &extraction.relationships {
                    let rel_id = format!(
                        "{}_{}_{}",
                        extraction.source_chunk_id, rel.source, rel.target
                    );
                    let span = SourceSpan::new(0, 0, 0, 0);
                    builder.record_relationship(
                        &rel_id,
                        &rel.source,
                        &rel.target,
                        &rel.relation_type,
                        &extraction.source_chunk_id,
                        span,
                        &rel.description,
                    );
                }
            }

            Some(builder.build())
        } else {
            None
        };

        Ok(ProcessingResult {
            document_id: document_id.to_string(),
            chunks,
            extractions,
            stats,
            lineage,
        })
    }

    /// Process multiple documents.
    pub async fn process_batch(
        &self,
        documents: &[(String, String)],
    ) -> Result<Vec<ProcessingResult>> {
        let mut results = Vec::with_capacity(documents.len());

        for (doc_id, content) in documents {
            let result = self.process(doc_id, content).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Get the pipeline configuration.
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    /// Get the chunker.
    pub fn chunker(&self) -> &Chunker {
        &self.chunker
    }

    /// Get the extractor.
    pub fn extractor(&self) -> Option<Arc<dyn EntityExtractor>> {
        self.extractor.clone()
    }

    /// Get the embedding provider.
    pub fn embedding_provider(&self) -> Option<Arc<dyn EmbeddingProvider>> {
        self.embedding_provider.clone()
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::default_pipeline()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractor::SimpleExtractor;

    #[tokio::test]
    async fn test_pipeline_basic_processing() {
        let pipeline = Pipeline::default_pipeline();

        let result = pipeline
            .process("doc-1", "This is a test document with some content.")
            .await
            .unwrap();

        assert_eq!(result.document_id, "doc-1");
        assert!(!result.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_pipeline_with_extractor() {
        let extractor = Arc::new(SimpleExtractor::default());
        let pipeline = Pipeline::default_pipeline().with_extractor(extractor);

        let result = pipeline
            .process("doc-1", "John Doe works at Acme Corp in New York.")
            .await
            .unwrap();

        // Should have extraction results
        assert!(result.stats.llm_calls > 0);
    }

    #[tokio::test]
    async fn test_pipeline_batch_processing() {
        let pipeline = Pipeline::default_pipeline();

        let documents = vec![
            ("doc-1".to_string(), "First document content.".to_string()),
            ("doc-2".to_string(), "Second document content.".to_string()),
        ];

        let results = pipeline.process_batch(&documents).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].document_id, "doc-1");
        assert_eq!(results[1].document_id, "doc-2");
    }

    #[test]
    fn test_pipeline_config_defaults() {
        let config = PipelineConfig::default();

        assert_eq!(config.extraction_batch_size, 10);
        assert!(config.enable_entity_extraction);
        assert!(config.enable_chunk_embeddings);
        assert!(!config.enable_lineage_tracking);
    }

    #[tokio::test]
    async fn test_pipeline_with_lineage_tracking() {
        let extractor = Arc::new(SimpleExtractor::default());
        let mut config = PipelineConfig::default();
        config.enable_lineage_tracking = true;

        let pipeline = Pipeline::new(config).with_extractor(extractor);

        let result = pipeline
            .process("doc-1", "John Doe works at Acme Corp in New York.")
            .await
            .unwrap();

        // Should have lineage
        assert!(result.lineage.is_some());

        let lineage = result.lineage.unwrap();
        assert_eq!(lineage.document_id, "doc-1");
        assert!(!lineage.chunks.is_empty());
        assert_eq!(lineage.total_chunks, result.chunks.len());
    }

    #[tokio::test]
    async fn test_pipeline_without_lineage_tracking() {
        let pipeline = Pipeline::default_pipeline();

        let result = pipeline
            .process("doc-1", "Simple document content.")
            .await
            .unwrap();

        // Should not have lineage (disabled by default)
        assert!(result.lineage.is_none());
    }
}
