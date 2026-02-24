//! Global RAG: Relationship-centric high-level retrieval with batch graph operations.

use crate::error::Result;
use crate::keyword_extractor::{ExtractedKeywords, KeywordExtractor};
use crate::types::{
    ContextEntity, ContextRelationship, QueryContext, QueryMode, QueryParams, QueryResult,
    QueryStats,
};
use std::collections::HashSet;
use std::sync::Arc;

impl super::QueryEngine {
    /// Global RAG: Relationship-centric high-level retrieval using BATCH operations.
    ///
    /// This mode extracts high-level keywords from the query, searches for
    /// relationships (edges) in the graph, and aggregates global context
    /// from the entire knowledge graph.
    ///
    /// Uses LightRAG-inspired batch operations for O(1) entity retrieval.
    pub(super) async fn query_global(
        &self,
        query: &str,
        params: &QueryParams,
    ) -> Result<QueryResult> {
        let retrieval_start = std::time::Instant::now();

        // 1. Extract high-level keywords from query
        let keyword_extractor = KeywordExtractor::new(Arc::clone(&self.llm));
        let keywords = keyword_extractor.extract(query).await?;

        tracing::debug!(
            high_level = ?keywords.high_level,
            low_level = ?keywords.low_level,
            "Extracted keywords for global query"
        );

        // 2. Embed high-level keywords for relationship search
        let keyword_texts: Vec<String> = keywords.high_level.clone();

        // If no high-level keywords, fall back to query embedding
        let search_texts = if keyword_texts.is_empty() {
            vec![query.to_string()]
        } else {
            keyword_texts
        };

        let keyword_embeddings = self
            .embedding
            .embed(&search_texts)
            .await
            .map_err(|e| crate::error::Error::internal(format!("Embedding error: {}", e)))?;

        // 3. Search for relationships using keyword embeddings
        let mut all_relationships = Vec::new();
        let mut seen_relations = HashSet::new();
        let per_keyword_k = (params.top_k / keyword_embeddings.len().max(1)).max(5);

        for keyword_embedding in &keyword_embeddings {
            let results = self
                .vector_storage
                .query(keyword_embedding, per_keyword_k, None)
                .await
                .map_err(|e| {
                    crate::error::Error::internal(format!("Vector search error: {}", e))
                })?;

            for result in results {
                // Filter by tenant/workspace
                if !self.matches_tenant(
                    &result.metadata,
                    params.tenant_id.as_deref(),
                    params.workspace_id.as_deref(),
                ) {
                    continue;
                }

                // Filter by type (only relationships for global mode)
                if result.metadata.get("type").and_then(|v| v.as_str()) != Some("relationship") {
                    continue;
                }

                // Use edge-like identifiers as keys for deduplication
                let relation_key = result.id.clone();
                if !seen_relations.contains(&relation_key) {
                    seen_relations.insert(relation_key);
                    all_relationships.push(result);
                }
            }
        }

        // Sort by score descending
        all_relationships.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 4. Collect all entity IDs from relationships first (for batch query)
        let mut entity_ids_to_fetch = Vec::new();
        let mut relationship_data = Vec::new();

        for rel_result in all_relationships.iter().take(params.top_k) {
            let description = rel_result
                .metadata
                .get("content")
                .or_else(|| rel_result.metadata.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let source_id = rel_result
                .metadata
                .get("source")
                .or_else(|| rel_result.metadata.get("source_id"))
                .and_then(|v| v.as_str())
                .unwrap_or(&rel_result.id)
                .to_string();

            let target_id = rel_result
                .metadata
                .get("target")
                .or_else(|| rel_result.metadata.get("target_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Collect entity IDs for batch fetch
            if !source_id.is_empty() && !entity_ids_to_fetch.contains(&source_id) {
                entity_ids_to_fetch.push(source_id.clone());
            }
            if !target_id.is_empty() && !entity_ids_to_fetch.contains(&target_id) {
                entity_ids_to_fetch.push(target_id.clone());
            }

            relationship_data.push((
                source_id,
                target_id,
                description.to_string(),
                rel_result.score,
            ));
        }

        // 5. BATCH: Fetch all entities at once (LightRAG pattern - O(1) instead of O(N))
        let nodes_map = self
            .graph_storage
            .get_nodes_batch(&entity_ids_to_fetch)
            .await
            .map_err(|e| {
                crate::error::Error::internal(format!("Batch node retrieval error: {}", e))
            })?;

        // 6. Build context from relationships and batch-fetched entities
        let mut context_relationships = Vec::new();
        let mut context_entities = Vec::new();
        let mut seen_entity_ids = HashSet::new();
        let mut context_text = String::new();

        context_text.push_str("### High-Level Relationships ###\n\n");

        for (source_id, target_id, description, score) in &relationship_data {
            context_relationships.push(ContextRelationship {
                source: source_id.clone(),
                target: target_id.clone(),
                relation_type: "RELATED".to_string(),
                description: description.clone(),
                score: *score,
            });

            context_text.push_str(&format!(
                "- {} → {}: {}\n",
                source_id, target_id, description
            ));
        }

        // Build entity context from batch results
        for entity_id in &entity_ids_to_fetch {
            if seen_entity_ids.contains(entity_id) {
                continue;
            }
            seen_entity_ids.insert(entity_id.clone());

            if let Some(node) = nodes_map.get(entity_id) {
                // Filter node by tenant/workspace
                if !self.matches_tenant(
                    &serde_json::Value::Object(node.properties.clone().into_iter().collect()),
                    params.tenant_id.as_deref(),
                    params.workspace_id.as_deref(),
                ) {
                    continue;
                }

                let entity_desc = node
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let entity_type = node
                    .properties
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN");

                context_entities.push(ContextEntity {
                    name: entity_id.clone(),
                    entity_type: entity_type.to_string(),
                    description: entity_desc.to_string(),
                    score: 1.0,
                });
            }
        }

        // Add entity descriptions to context
        if !context_entities.is_empty() {
            context_text.push_str("\n### Related Entities ###\n\n");
            for entity in &context_entities {
                if !entity.description.is_empty() {
                    context_text.push_str(&format!(
                        "**{}** ({}): {}\n",
                        entity.name, entity.entity_type, entity.description
                    ));
                }
            }
        }

        let retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;
        let generation_start = std::time::Instant::now();

        // 7. Generate response using global context
        let prompt = Self::build_global_prompt(query, &context_text, &keywords);

        let response = self
            .llm
            .complete(&prompt)
            .await
            .map_err(|e| crate::error::Error::internal(format!("LLM error: {}", e)))?;

        let generation_time_ms = generation_start.elapsed().as_millis() as u64;

        // Calculate stats before moving values
        let entities_count = seen_entity_ids.len();
        let relationships_count = context_relationships.len();
        let keywords_count = keywords.len();

        Ok(QueryResult {
            response: response.content,
            mode: QueryMode::Global,
            context: QueryContext {
                entities: context_entities,
                relationships: context_relationships,
                ..Default::default()
            },
            stats: QueryStats {
                retrieval_time_ms,
                generation_time_ms,
                total_time_ms: 0,
                entities_retrieved: entities_count,
                relationships_retrieved: relationships_count,
                keywords_extracted: keywords_count,
                prompt_tokens: response.prompt_tokens,
                response_tokens: response.completion_tokens,
                ..Default::default()
            },
        })
    }

    /// Build prompt for global query mode.
    fn build_global_prompt(query: &str, context: &str, keywords: &ExtractedKeywords) -> String {
        format!(
            r#"---Role---
You are a helpful assistant responding to questions about data in the provided tables and relationships.

---Goal---
Generate a response of the target length and format that responds to the user's question, summarizing all information in the input data tables appropriate for the response length and format, and incorporating any relevant general knowledge.

If you don't know the answer, just say so. Do not make anything up.

Points supported by data should list their sources at the end of the response.

---Target response length and format---
Multiple paragraphs

---Data tables---
{context}

---Keywords identified---
High-level themes: {high_level_keywords}
Specific terms: {low_level_keywords}

---Query---
{query}

Response:"#,
            context = context,
            high_level_keywords = keywords.high_level.join(", "),
            low_level_keywords = keywords.low_level.join(", "),
            query = query
        )
    }
}
