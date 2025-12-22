//! Document processing pipeline.

use std::sync::Arc;

use edgequake_llm::traits::EmbeddingProvider;
use serde::{Deserialize, Serialize};

use crate::chunker::{Chunker, ChunkerConfig, TextChunk};
use crate::error::Result;
use crate::extractor::{EntityExtractor, ExtractionResult};

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

    /// Process a document through the pipeline.
    pub async fn process(&self, document_id: &str, content: &str) -> Result<ProcessingResult> {
        let start = std::time::Instant::now();
        let mut stats = ProcessingStats::default();

        // Step 1: Chunk the document
        let mut chunks = self.chunker.chunk(content, document_id)?;
        stats.chunk_count = chunks.len();

        // Step 2: Extract entities and relationships
        let mut extractions = Vec::new();

        if self.config.enable_entity_extraction || self.config.enable_relationship_extraction {
            if let Some(extractor) = &self.extractor {
                for chunk in &chunks {
                    let extraction = extractor.extract(chunk).await?;
                    stats.entity_count += extraction.entities.len();
                    stats.relationship_count += extraction.relationships.len();
                    stats.llm_calls += 1;
                    extractions.push(extraction);
                }
            }
        }

        // Step 3: Generate embeddings
        if let Some(provider) = &self.embedding_provider {
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
                        let embeddings = provider.embed(&relationship_texts).await.map_err(|e| {
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

        Ok(ProcessingResult {
            document_id: document_id.to_string(),
            chunks,
            extractions,
            stats,
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
    }
}
