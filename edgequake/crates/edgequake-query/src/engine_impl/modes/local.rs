//! Local query mode — entity-centric retrieval with graph context.

use std::collections::HashMap;
use std::sync::Arc;

use crate::context::QueryContext;
use crate::error::Result;
use crate::helpers::{build_entity_from_node, build_relationship_from_edge};
use crate::keywords::ExtractedKeywords;
use crate::vector_filter::{filter_by_type, VectorType};

use edgequake_storage::traits::VectorStorage;

use super::super::{QueryEmbeddings, QueryEngine};
use super::chunk_retrieval::append_score_ranked_chunks;
use super::make_scope_metadata_filter;

impl QueryEngine {
    /// Local mode with workspace-specific vector storage.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::engine_impl) async fn query_local_with_vector_storage(
        &self,
        query_text: &str,
        _keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
        // Document IDs to restrict vector search to (SPEC-031 Tier 1 pre-filter).
        allowed_document_ids: Option<&[String]>,
        vector_storage: &Arc<dyn VectorStorage>,
        max_chunks: usize,
    ) -> Result<QueryContext> {
        let retrieval_config = self.config_with_max_chunks(max_chunks);
        let mut context = QueryContext::new();
        // SPEC-031: push document scope filter to SQL layer (Tier 1 pre-filter)
        let mf = make_scope_metadata_filter(
            tenant_id.clone(),
            workspace_id.clone(),
            allowed_document_ids,
            None,
        );

        let vector_results = vector_storage
            .query_filtered(
                &embeddings.low_level,
                self.config.max_entities * 3,
                None,
                mf.as_ref(),
            )
            .await?;

        let entity_vectors = filter_by_type(vector_results, VectorType::Entity);

        let entity_scores: HashMap<String, f32> = entity_vectors
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .map(|r| {
                let entity_name =
                    crate::helpers::decode_entity_name_from_result(&r.id, &r.metadata);
                let entity_name = if entity_name.is_empty() {
                    r.id.clone()
                } else {
                    entity_name
                };
                (entity_name, r.score)
            })
            .collect();

        let entity_ids: Vec<String> = entity_vectors
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .map(|r| {
                let name = crate::helpers::decode_entity_name_from_result(&r.id, &r.metadata);
                if name.is_empty() {
                    r.id.clone()
                } else {
                    name
                }
            })
            .take(self.config.max_entities)
            .collect();

        if entity_ids.is_empty() {
            tracing::debug!(
                workspace_id = ?workspace_id,
                "OODA-231: No entity vectors found, falling back to popular entities from graph"
            );
            let graph = self.graph_read();
            let popular = graph
                .get_popular_nodes_with_degree(
                    self.config.max_entities,
                    None,
                    None,
                    tenant_id.as_deref(),
                    workspace_id.as_deref(),
                )
                .await?;

            let fallback_entity_ids: Vec<String> =
                popular.iter().map(|(n, _)| n.id.clone()).collect();

            for (node, degree) in popular {
                let entity = build_entity_from_node(&node.id, &node.properties, degree, 0.0);
                context.add_entity(entity);
            }

            if !fallback_entity_ids.is_empty() {
                let edges = crate::graph_hops::edges_within_depth(
                    &graph,
                    &fallback_entity_ids,
                    self.config.graph_depth,
                    self.config.max_relationships,
                )
                .await?;
                for edge in edges {
                    let rel =
                        build_relationship_from_edge(&edge.source, &edge.target, &edge.properties);
                    context.add_relationship(rel);
                }
            }
        } else {
            let graph = self.graph_read();
            let (nodes_map, degrees) = tokio::join!(
                graph.get_nodes_batch(&entity_ids),
                graph.node_degrees_batch(&entity_ids),
            );

            let nodes_map = nodes_map?;
            let degrees: HashMap<String, usize> = degrees?.into_iter().collect();

            for id in &entity_ids {
                if let Some(node) = nodes_map.get(id) {
                    let degree = degrees.get(id).copied().unwrap_or(0);
                    let entity_score = entity_scores.get(id).copied().unwrap_or(0.0);
                    let entity = build_entity_from_node(id, &node.properties, degree, entity_score);
                    context.add_entity(entity);
                }
            }

            let edges = crate::graph_hops::edges_within_depth(
                &graph,
                &entity_ids,
                self.config.graph_depth,
                self.config.max_relationships,
            )
            .await?;

            for edge in edges {
                let rel =
                    build_relationship_from_edge(&edge.source, &edge.target, &edge.properties);
                context.add_relationship(rel);
            }
        }

        let chunks = append_score_ranked_chunks(
            self,
            &context,
            query_text,
            &embeddings.low_level,
            tenant_id,
            workspace_id,
            vector_storage,
            &retrieval_config,
            mf.as_ref(),
            "local",
        )
        .await?;

        for chunk in chunks {
            context.add_chunk(chunk);
        }

        Ok(context)
    }
}
