//! Global query mode — relationship-centric retrieval with community expansion.

use std::collections::HashMap;
use std::sync::Arc;

use crate::context::{QueryContext, RetrievedRelationship};
use crate::error::Result;
use crate::helpers::{build_entity_from_node, build_relationship_from_edge};
use crate::keywords::ExtractedKeywords;
use crate::vector_filter::{filter_by_type, VectorType};

use edgequake_storage::traits::VectorStorage;

use super::super::{QueryEmbeddings, QueryEngine};
use super::chunk_retrieval::append_score_ranked_chunks;
use super::make_scope_metadata_filter;

impl QueryEngine {
    /// Global mode with workspace-specific vector storage.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::engine_impl) async fn query_global_with_vector_storage(
        &self,
        query_text: &str,
        keywords: &ExtractedKeywords,
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
        let mut entity_ids: Vec<String> = Vec::new();
        let mut seen_relationships = std::collections::HashSet::new();
        // SPEC-031: push document scope filter to SQL layer (Tier 1 pre-filter)
        // SPEC-058: push vector_type=relationship to SQL (Naive already pushes chunk).
        let mf = make_scope_metadata_filter(
            tenant_id.clone(),
            workspace_id.clone(),
            allowed_document_ids,
            Some("relationship"),
        );

        let vector_results = vector_storage
            .query_filtered(
                &embeddings.high_level,
                self.config.max_relationships * 3,
                None,
                mf.as_ref(),
            )
            .await?;

        let relationship_vectors = filter_by_type(vector_results.clone(), VectorType::Relationship);

        // SPEC-046 EQ-046-11: optional community_report vectors for L3 thematic context
        if edgequake_storage::community_reports_enabled() {
            let report_hits = filter_by_type(vector_results.clone(), VectorType::CommunityReport);
            let report_hits: Vec<_> = report_hits
                .into_iter()
                .filter(|r| r.score >= self.config.min_score)
                .collect();
            crate::community_global::append_community_report_vector_chunks(
                &mut context,
                &report_hits,
                self.config.max_chunks.min(8),
            );
        }

        for result in relationship_vectors
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .take(self.config.max_relationships)
        {
            let src_id = result
                .metadata
                .get("src_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let tgt_id = result
                .metadata
                .get("tgt_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let rel_type = result
                .metadata
                .get("relation_type")
                .and_then(|v| v.as_str())
                .unwrap_or("RELATED_TO");
            let description = result
                .metadata
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if !src_id.is_empty() && !tgt_id.is_empty() {
                let src_graph =
                    crate::helpers::graph_entity_id_for_workspace(src_id, workspace_id.as_deref());
                let tgt_graph =
                    crate::helpers::graph_entity_id_for_workspace(tgt_id, workspace_id.as_deref());
                let rel_key = format!("{}->{}:{}", src_id, tgt_id, rel_type);
                if seen_relationships.insert(rel_key) {
                    let source_chunk_ids: Vec<String> = result
                        .metadata
                        .get("source_chunk_ids")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let source_chunk_id = result
                        .metadata
                        .get("source_chunk_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let source_document_id = result
                        .metadata
                        .get("source_document_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let source_file_path = result
                        .metadata
                        .get("source_file_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    // Prompt/display keep bare names; graph lookups use scoped ids.
                    let mut rel = RetrievedRelationship::new(src_id, tgt_id, rel_type.to_string())
                        .with_description(description.to_string())
                        .with_score(result.score);
                    if !source_chunk_ids.is_empty() {
                        rel = rel.with_source_chunk_ids(source_chunk_ids);
                    } else if let Some(chunk_id) = source_chunk_id {
                        rel = rel.with_source_chunk_id(chunk_id);
                    }
                    if let Some(doc_id) = source_document_id {
                        rel = rel.with_source_document_id(doc_id);
                    }
                    if let Some(file_path) = source_file_path {
                        rel = rel.with_source_file_path(file_path);
                    }
                    context.add_relationship(rel);
                    if !src_graph.is_empty() && !entity_ids.contains(&src_graph) {
                        entity_ids.push(src_graph);
                    }
                    if !tgt_graph.is_empty() && !entity_ids.contains(&tgt_graph) {
                        entity_ids.push(tgt_graph);
                    }
                }
            }
        }

        if entity_ids.is_empty() {
            if crate::keyword_boost::popular_node_fallback_enabled() {
                tracing::debug!(
                    workspace_id = ?workspace_id,
                    "OODA-231: No relationship vectors found, falling back to popular entities from graph"
                );
                crate::retrieval_telemetry::mark_popular_node_fallback(&mut context, "global");
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

                for (node, degree) in &popular {
                    let entity = build_entity_from_node(&node.id, &node.properties, *degree, 0.0);
                    context.add_entity(entity);
                    entity_ids.push(node.id.clone());
                }

                if !entity_ids.is_empty() {
                    let edges = crate::graph_expand::expand_neighborhood_edges_scoped(
                        &graph,
                        &entity_ids,
                        self.config.graph_depth,
                        self.config.max_relationships,
                        self.config.graph_walk,
                        tenant_id.as_deref(),
                        workspace_id.as_deref(),
                        allowed_document_ids,
                    )
                    .await?;
                    for edge in edges {
                        let rel_key = format!("{}->{}:{}", edge.source, edge.target, "RELATED_TO");
                        if seen_relationships.insert(rel_key) {
                            let rel = build_relationship_from_edge(
                                &edge.source,
                                &edge.target,
                                &edge.properties,
                            );
                            context.add_relationship(rel);
                        }
                    }
                }
            } else {
                tracing::debug!(
                    workspace_id = ?workspace_id,
                    "No relationship vectors; popular-node fallback disabled (EDGEQUAKE_POPULAR_NODE_FALLBACK=0)"
                );
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
                    let entity = build_entity_from_node(id, &node.properties, degree, 0.5);
                    context.add_entity(entity);
                }
            }
            if crate::keyword_boost::keyword_lexical_boost_enabled() {
                let kw = keywords.all_keywords();
                crate::keyword_boost::boost_entities_by_keywords(&mut context.entities, &kw);
            }
        }

        // 038 SELECT: Exploratory exact-name topic entities → Mix chunk pool
        crate::topic_entity_admit::admit_topic_entities(
            self.graph_read(),
            &mut context,
            query_text,
            keywords,
            workspace_id.as_deref(),
        )
        .await?;

        // SPEC-146 M3 / R9: skip community inject under ABAC allow-set (mixed-label risk).
        if allowed_document_ids.is_none() {
            crate::community_global::expand_global_context_with_communities(
                &self.config,
                &mut context,
                &mut entity_ids,
                self.graph_read(),
                tenant_id.clone(),
                workspace_id.clone(),
            )
            .await?;
        } else {
            tracing::debug!(
                "SPEC-146: skipping community global expansion under document allow-set"
            );
        }

        // Chunk fetch SSOT uses vector_type=chunk (not the relationship ANN `mf` above).
        // 078 R3: Mix post_truncate skips per-arm pick (re-pick after E/R truncate).
        if !crate::kg_chunk_pick::arm_kg_chunks_skipped() {
            let (chunks, sparse_outcome) = append_score_ranked_chunks(
                self,
                &context,
                query_text,
                &embeddings.high_level,
                tenant_id,
                workspace_id,
                vector_storage,
                &retrieval_config,
                allowed_document_ids,
                "global",
            )
            .await?;

            crate::retrieval_telemetry::mark_sparse_outcome(
                &mut context,
                sparse_outcome.as_str(),
                sparse_outcome.is_fts_fallback(),
            );
            for chunk in chunks {
                context.add_chunk(chunk);
            }
        }

        // 073: soft-label relationship endpoints for Connections / LLM context.
        if let Err(e) = crate::helpers::resolve_relationship_endpoint_labels(
            &self.graph_read(),
            &mut context.relationships,
        )
        .await
        {
            tracing::warn!(error = %e, "failed to resolve relationship endpoint labels");
        }

        Ok(context)
    }
}
