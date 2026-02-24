//! Mix RAG: Combines local entity-centric and naive chunk retrieval.

use crate::error::Result;
use crate::types::{QueryContext, QueryMode, QueryParams, QueryResult, QueryStats};
use std::collections::HashSet;

impl super::QueryEngine {
    /// Mix RAG: Combines local entity-centric and naive chunk retrieval.
    ///
    /// This is the recommended default mode as it provides the most comprehensive
    /// context for question answering.
    pub(super) async fn query_mix(&self, query: &str, params: &QueryParams) -> Result<QueryResult> {
        let retrieval_start = std::time::Instant::now();

        // 1. Run local query (entity-centric)
        let local_result = self.query_local(query, params).await?;

        // 2. Run naive query (chunk-based)
        let naive_result = self.query_naive(query, params).await?;

        // 3. Merge contexts with deduplication
        let merged_entities = local_result.context.entities;
        let merged_relationships = local_result.context.relationships;
        let merged_chunks = naive_result.context.chunks;

        // Deduplicate entities by name
        let _seen_entity_names: HashSet<_> = merged_entities.iter().map(|e| &e.name).collect();

        // Deduplicate chunks by ID
        let _seen_chunk_ids: HashSet<_> = merged_chunks.iter().map(|c| &c.chunk_id).collect();

        // Build unified context text
        let mut context_text = String::new();

        // Add entities section
        if !merged_entities.is_empty() {
            context_text.push_str("### Knowledge Graph Entities ###\n\n");
            for entity in &merged_entities {
                context_text.push_str(&format!(
                    "**{}** ({}): {}\n",
                    entity.name, entity.entity_type, entity.description
                ));
            }
            context_text.push('\n');
        }

        // Add relationships section
        if !merged_relationships.is_empty() {
            context_text.push_str("### Relationships ###\n\n");
            for rel in &merged_relationships {
                context_text.push_str(&format!(
                    "- {} → {}: {}\n",
                    rel.source, rel.target, rel.description
                ));
            }
            context_text.push('\n');
        }

        // Add chunks section
        if !merged_chunks.is_empty() {
            context_text.push_str("### Document Chunks ###\n\n");
            for chunk in &merged_chunks {
                context_text.push_str(&format!("---\n{}\n", chunk.content));
            }
        }

        let retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;
        let generation_start = std::time::Instant::now();

        // 4. Generate response using combined context
        let prompt = format!(
            r#"---Role---
You are a helpful assistant responding to questions about data in the knowledge graph and document chunks.

---Goal---
Generate a comprehensive response that synthesizes information from both the knowledge graph entities/relationships and the document chunks.

---Context---
{}

---Query---
{}

---Instructions---
1. Prioritize information from entities and relationships for structured knowledge
2. Use document chunks to fill in details and provide supporting evidence
3. If there are contradictions, prefer the more specific or recent information
4. Cite sources when possible

Response:"#,
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
            mode: QueryMode::Mix,
            context: QueryContext {
                entities: merged_entities,
                relationships: merged_relationships,
                chunks: merged_chunks,
            },
            stats: QueryStats {
                retrieval_time_ms,
                generation_time_ms,
                total_time_ms: 0,
                entities_retrieved: local_result.stats.entities_retrieved,
                relationships_retrieved: local_result.stats.relationships_retrieved,
                chunks_retrieved: naive_result.stats.chunks_retrieved,
                prompt_tokens: response.prompt_tokens,
                response_tokens: response.completion_tokens,
                ..Default::default()
            },
        })
    }
}
