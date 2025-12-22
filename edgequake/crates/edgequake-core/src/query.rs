use crate::error::Result;
use crate::types::{
    ContextChunk, ContextEntity, ContextRelationship, QueryContext, QueryMode, QueryParams,
    QueryResult, QueryStats,
};
use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use edgequake_storage::traits::{GraphStorage, VectorStorage};
use std::sync::Arc;

/// Engine for executing RAG queries.
pub struct QueryEngine {
    llm: Arc<dyn LLMProvider>,
    embedding: Arc<dyn EmbeddingProvider>,
    graph_storage: Arc<dyn GraphStorage>,
    vector_storage: Arc<dyn VectorStorage>,
}

impl QueryEngine {
    /// Create a new query engine.
    pub fn new(
        llm: Arc<dyn LLMProvider>,
        embedding: Arc<dyn EmbeddingProvider>,
        graph_storage: Arc<dyn GraphStorage>,
        vector_storage: Arc<dyn VectorStorage>,
    ) -> Self {
        Self {
            llm,
            embedding,
            graph_storage,
            vector_storage,
        }
    }

    /// Execute a query.
    pub async fn query(&self, query: &str, params: QueryParams) -> Result<QueryResult> {
        let start = std::time::Instant::now();

        let result = match params.mode {
            QueryMode::Naive => self.query_naive(query, &params).await?,
            QueryMode::Local => self.query_local(query, &params).await?,
            _ => self.query_naive(query, &params).await?, // Fallback for now
        };

        let mut final_result = result;
        final_result.stats.total_time_ms = start.elapsed().as_millis() as u64;

        Ok(final_result)
    }

    /// Naive RAG: Simple vector search on chunks.
    async fn query_naive(&self, query: &str, params: &QueryParams) -> Result<QueryResult> {
        let retrieval_start = std::time::Instant::now();

        // 1. Embed query
        let query_embeddings = self
            .embedding
            .embed(&[query.to_string()])
            .await
            .map_err(|e| crate::error::Error::internal(format!("Embedding error: {}", e)))?;

        let query_embedding = query_embeddings
            .first()
            .ok_or_else(|| crate::error::Error::internal("No embedding generated"))?;

        // 2. Search vector store for chunks
        let search_results = self
            .vector_storage
            .query(query_embedding, params.top_k, None)
            .await
            .map_err(|e| crate::error::Error::internal(format!("Vector search error: {}", e)))?;

        let retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;

        // 3. Build context
        let mut context_chunks = Vec::new();
        let mut context_text = String::new();

        for result in search_results {
            let id = result.id;
            let score = result.score;
            let metadata = result.metadata;

            let content = metadata
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let doc_id = metadata
                .get("document_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            context_chunks.push(ContextChunk {
                chunk_id: id.clone(),
                document_id: doc_id,
                content: content.clone(),
                score,
            });

            context_text.push_str(&format!("--- Chunk {} ---\n{}\n\n", id, content));
        }

        let generation_start = std::time::Instant::now();

        // 4. Generate response
        let prompt = format!(
            "Answer the following question based on the provided context.\n\nContext:\n{}\n\nQuestion: {}\n\nAnswer:",
            context_text, query
        );

        let response = self
            .llm
            .complete(&prompt)
            .await
            .map_err(|e| crate::error::Error::internal(format!("LLM error: {}", e)))?;

        let generation_time_ms = generation_start.elapsed().as_millis() as u64;

        Ok(QueryResult {
            response: response.content,
            mode: QueryMode::Naive,
            context: QueryContext {
                chunks: context_chunks,
                ..Default::default()
            },
            stats: QueryStats {
                retrieval_time_ms,
                generation_time_ms,
                total_time_ms: 0, // Set by caller
                chunks_retrieved: context_text.len(),
                prompt_tokens: response.prompt_tokens,
                response_tokens: response.completion_tokens,
                ..Default::default()
            },
        })
    }

    /// Local RAG: Entity-centric retrieval.
    async fn query_local(&self, query: &str, params: &QueryParams) -> Result<QueryResult> {
        let retrieval_start = std::time::Instant::now();

        // 1. Embed query
        let query_embeddings = self
            .embedding
            .embed(&[query.to_string()])
            .await
            .map_err(|e| crate::error::Error::internal(format!("Embedding error: {}", e)))?;

        let query_embedding = query_embeddings
            .first()
            .ok_or_else(|| crate::error::Error::internal("No embedding generated"))?;

        // 2. Search vector store for entities
        let entity_results = self
            .vector_storage
            .query(query_embedding, params.top_k, None)
            .await
            .map_err(|e| crate::error::Error::internal(format!("Vector search error: {}", e)))?;

        // 3. Retrieve entity details and neighbors from graph
        let mut context_entities = Vec::new();
        let mut context_relationships = Vec::new();
        let mut context_text = String::new();

        for result in entity_results {
            let entity_id = result.id;
            let score = result.score;

            if let Some(node) = self.graph_storage.get_node(&entity_id).await? {
                let name = node
                    .properties
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&entity_id)
                    .to_string();
                let entity_type = node
                    .properties
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string();
                let description = node
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                context_entities.push(ContextEntity {
                    name: name.clone(),
                    entity_type,
                    description: description.clone(),
                    score,
                });

                context_text.push_str(&format!("--- Entity: {} ---\n{}\n\n", name, description));

                // Get neighbors (relationships)
                let edges = self.graph_storage.get_node_edges(&entity_id).await?;
                for edge in edges {
                    context_relationships.push(ContextRelationship {
                        source: edge.source.clone(),
                        target: edge.target.clone(),
                        relation_type: "RELATED".to_string(), // Default for now
                        description: edge
                            .properties
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        score: 1.0,
                    });
                }
            }
        }

        let retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;
        let generation_start = std::time::Instant::now();

        // 4. Generate response
        let prompt = format!(
            "Answer the following question based on the provided knowledge graph context.\n\nContext:\n{}\n\nQuestion: {}\n\nAnswer:",
            context_text, query
        );

        let response = self
            .llm
            .complete(&prompt)
            .await
            .map_err(|e| crate::error::Error::internal(format!("LLM error: {}", e)))?;

        let generation_time_ms = generation_start.elapsed().as_millis() as u64;

        Ok(QueryResult {
            response: response.content,
            mode: QueryMode::Local,
            context: QueryContext {
                entities: context_entities,
                relationships: context_relationships,
                ..Default::default()
            },
            stats: QueryStats {
                retrieval_time_ms,
                generation_time_ms,
                total_time_ms: 0,
                entities_retrieved: context_text.len(),
                prompt_tokens: response.prompt_tokens,
                response_tokens: response.completion_tokens,
                ..Default::default()
            },
        })
    }
}

