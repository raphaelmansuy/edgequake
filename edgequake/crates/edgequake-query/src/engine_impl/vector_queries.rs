use std::collections::HashMap;
use std::sync::Arc;

use crate::context::{QueryContext, RetrievedChunk, RetrievedRelationship};
use crate::error::Result;
use crate::helpers::{
    build_chunk_from_result, build_entity_from_node, build_relationship_from_edge,
};
use crate::keywords::ExtractedKeywords;
use crate::vector_filter::{filter_by_type, VectorType};

use edgequake_storage::traits::{MetadataFilter, VectorStorage};

use super::{QueryEmbeddings, QueryEngine};

impl QueryEngine {
    pub(super) async fn query_naive_with_vector_storage(
        &self,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
        vector_storage: &Arc<dyn VectorStorage>,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();

        // WHY: Use vector_type filter at the SQL layer so LIMIT operates only on chunk
        // vectors. Without this, large graphs (60k+ entities) cause the top-k results
        // to be dominated by entity vectors, leaving 0 chunks after in-memory filtering.
        // SPEC-007: tenant/workspace/type filter pushed to storage layer via query_filtered.
        let mf = MetadataFilter::from_tenant_workspace_type(tenant_id, workspace_id, "chunk");

        let results = vector_storage
            .query_filtered(&embeddings.query, self.config.max_chunks, None, mf.as_ref())
            .await?;

        for result in results
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .take(self.config.max_chunks)
        {
            context.add_chunk(build_chunk_from_result(result));
        }

        Ok(context)
    }

    /// Local mode with workspace-specific vector storage.
    pub(super) async fn query_local_with_vector_storage(
        &self,
        _keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
        vector_storage: &Arc<dyn VectorStorage>,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();
        let mf = MetadataFilter::from_tenant_workspace(tenant_id.clone(), workspace_id.clone());

        // Step 1: Vector search with LOW-level keyword embedding
        // SPEC-007: tenant/workspace filter pushed to storage layer via query_filtered.
        let vector_results = vector_storage
            .query_filtered(
                &embeddings.low_level,
                self.config.max_entities * 3,
                None,
                mf.as_ref(),
            )
            .await?;

        // Step 2: Filter to entity vectors only
        let entity_vectors = filter_by_type(vector_results, VectorType::Entity);

        // Step 2.5: Build entity scores map
        // SPEC-021 P-E1: use the shared VectorId-based decoder so the storage
        // ID is the authoritative entity-name source (metadata can drift).
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

        // Step 3: Extract entity IDs
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

        // OODA-231: When no entity vectors exist (workspace-isolated storage often only has chunks),
        // fall back to popular entities from the graph, then continue to collect chunks.
        // WHY: Early return skipped chunk collection, causing 0 sources in response.
        if entity_ids.is_empty() {
            tracing::debug!(
                workspace_id = ?workspace_id,
                "OODA-231: No entity vectors found, falling back to popular entities from graph"
            );
            let graph = self.graph_read();
            // Populate context with popular entities from graph
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

            // Get edges for fallback entities
            if !fallback_entity_ids.is_empty() {
                let edges = graph
                    .get_edges_for_nodes_batch(&fallback_entity_ids)
                    .await?;
                for edge in edges.iter().take(self.config.max_relationships) {
                    let rel =
                        build_relationship_from_edge(&edge.source, &edge.target, &edge.properties);
                    context.add_relationship(rel);
                }
            }
            // NOTE: Don't return early - continue to chunk collection below
        } else {
            let graph = self.graph_read();
            // Step 4: Batch fetch nodes and degrees
            let (nodes_map, degrees) = tokio::join!(
                graph.get_nodes_batch(&entity_ids),
                graph.node_degrees_batch(&entity_ids),
            );

            let nodes_map = nodes_map?;
            let degrees: HashMap<String, usize> = degrees?.into_iter().collect();

            // Step 5: Build entity context
            // WHY: Use entity_ids (Vec) for deterministic ordering, not HashMap iteration.
            for id in &entity_ids {
                if let Some(node) = nodes_map.get(id) {
                    let degree = degrees.get(id).copied().unwrap_or(0);
                    let entity_score = entity_scores.get(id).copied().unwrap_or(0.0);
                    let entity = build_entity_from_node(id, &node.properties, degree, entity_score);
                    context.add_entity(entity);
                }
            }

            // Step 6: Batch fetch edges
            let edges = graph.get_edges_for_nodes_batch(&entity_ids).await?;

            for edge in edges.iter().take(self.config.max_relationships) {
                let rel =
                    build_relationship_from_edge(&edge.source, &edge.target, &edge.properties);
                context.add_relationship(rel);
            }
        }

        // Step 7: Collect source_chunk_ids from entities and relationships
        // WHY-OODA230: Must retrieve chunks via their IDs, not by semantic similarity.
        // The old approach (semantic search + filter_by_type) returned 0 chunks because
        // entity/relationship vectors often score higher than chunks for concept queries.
        let mut chunk_ids = std::collections::HashSet::new();

        // Collect chunk IDs from entities
        for entity in &context.entities {
            for chunk_id in &entity.source_chunk_ids {
                chunk_ids.insert(chunk_id.clone());
            }
        }

        // Collect chunk IDs from relationships
        for rel in &context.relationships {
            if let Some(chunk_id) = &rel.source_chunk_id {
                chunk_ids.insert(chunk_id.clone());
            }
        }

        tracing::info!(
            total_chunk_ids = chunk_ids.len(),
            entity_count = context.entities.len(),
            relationship_count = context.relationships.len(),
            "OODA-230: Local mode chunk collection (workspace)"
        );

        // Retrieve chunks from workspace vector storage using chunk IDs
        if !chunk_ids.is_empty() {
            // WHY: Pass ALL candidate chunk IDs to vector storage and let cosine similarity
            // rank them. VectorStorage.query() returns results sorted by score descending.
            let chunk_ids_vec: Vec<String> = chunk_ids.into_iter().collect();

            tracing::debug!(
                chunk_ids_count = chunk_ids_vec.len(),
                max_chunks = self.config.max_chunks,
                "OODA-231: Requesting chunks by ID from vector storage (score-ranked)"
            );

            // Query with filter to retrieve only the specific chunks, score-ranked
            // SPEC-007: tenant/workspace filter pushed to storage layer.
            let results = vector_storage
                .query_filtered(
                    &embeddings.low_level,
                    self.config.max_chunks,
                    Some(&chunk_ids_vec),
                    mf.as_ref(),
                )
                .await?;

            tracing::debug!(
                candidates = chunk_ids_vec.len(),
                returned = results.len(),
                "OODA-231: Chunk retrieval result (top-k by cosine similarity)"
            );

            for result in results {
                context.add_chunk(build_chunk_from_result(&result));
            }
        }

        Ok(context)
    }

    /// Global mode with workspace-specific vector storage.
    pub(super) async fn query_global_with_vector_storage(
        &self,
        _keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
        vector_storage: &Arc<dyn VectorStorage>,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();
        let mut entity_ids: Vec<String> = Vec::new();
        let mut seen_relationships = std::collections::HashSet::new();
        let mf = MetadataFilter::from_tenant_workspace(tenant_id.clone(), workspace_id.clone());

        // Step 1: Vector search with HIGH-level keyword embedding
        // SPEC-007: tenant/workspace filter pushed to storage layer via query_filtered.
        let vector_results = vector_storage
            .query_filtered(
                &embeddings.high_level,
                self.config.max_relationships * 3,
                None,
                mf.as_ref(),
            )
            .await?;

        // Step 2: Filter to relationship vectors only
        let relationship_vectors = filter_by_type(vector_results.clone(), VectorType::Relationship);

        // Step 3: Extract relationships from vector results
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
                let rel_key = format!("{}->{}:{}", src_id, tgt_id, rel_type);
                if seen_relationships.insert(rel_key) {
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

                    let mut rel = RetrievedRelationship::new(src_id, tgt_id, rel_type.to_string())
                        .with_description(description.to_string())
                        .with_score(result.score);
                    if let Some(chunk_id) = source_chunk_id {
                        rel = rel.with_source_chunk_id(chunk_id);
                    }
                    if let Some(doc_id) = source_document_id {
                        rel = rel.with_source_document_id(doc_id);
                    }
                    if let Some(file_path) = source_file_path {
                        rel = rel.with_source_file_path(file_path);
                    }
                    context.add_relationship(rel);
                    if !entity_ids.contains(&src_id.to_string()) {
                        entity_ids.push(src_id.to_string());
                    }
                    if !entity_ids.contains(&tgt_id.to_string()) {
                        entity_ids.push(tgt_id.to_string());
                    }
                }
            }
        }

        // Step 4: OODA-231: When no relationship vectors exist, fall back to popular entities
        // WHY: Early return skipped chunk collection, causing 0 sources in response.
        if entity_ids.is_empty() {
            tracing::debug!(
                workspace_id = ?workspace_id,
                "OODA-231: No relationship vectors found, falling back to popular entities from graph"
            );
            let graph = self.graph_read();
            // Populate context with popular entities from graph
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

            // Get edges for fallback entities
            if !entity_ids.is_empty() {
                let edges = graph.get_edges_for_nodes_batch(&entity_ids).await?;
                for edge in edges.iter().take(self.config.max_relationships) {
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
            // NOTE: Don't return early - continue to chunk collection below
        } else {
            let graph = self.graph_read();
            // Step 5: Batch fetch entity nodes AND degrees in one round-trip each
            // (RC-8 / P-G3): the previous loop issued `node_degree(id)` per entity,
            // an O(E) N+1 pattern. Local mode already used `node_degrees_batch`;
            // Global now does the same so a graph with E entities performs exactly
            // one degree-fetch call.
            let (nodes_map, degrees) = tokio::join!(
                graph.get_nodes_batch(&entity_ids),
                graph.node_degrees_batch(&entity_ids),
            );

            let nodes_map = nodes_map?;
            let degrees: HashMap<String, usize> = degrees?.into_iter().collect();

            // WHY: Iterate entity_ids (Vec) for deterministic ordering instead of HashMap.
            // HashMap iteration order is random, causing non-deterministic results.
            // E13: a node disappearing between `get_nodes_batch` and
            // `node_degrees_batch` resolves to degree 0 via `unwrap_or(0)`.
            for id in &entity_ids {
                if let Some(node) = nodes_map.get(id) {
                    let degree = degrees.get(id).copied().unwrap_or(0);
                    let entity = build_entity_from_node(id, &node.properties, degree, 0.5);
                    context.add_entity(entity);
                }
            }
        }

        // Step 6: Collect source_chunk_ids from entities and relationships
        // WHY-OODA230: Must retrieve chunks via their IDs, not by semantic similarity.
        // The old approach (semantic search + filter_by_type) returned 0 chunks because
        // entity/relationship vectors often score higher than chunks for concept queries.
        let mut chunk_ids = std::collections::HashSet::new();

        // Collect chunk IDs from entities
        for entity in &context.entities {
            for chunk_id in &entity.source_chunk_ids {
                chunk_ids.insert(chunk_id.clone());
            }
        }

        // Collect chunk IDs from relationships
        for rel in &context.relationships {
            if let Some(chunk_id) = &rel.source_chunk_id {
                chunk_ids.insert(chunk_id.clone());
            }
        }

        tracing::info!(
            total_chunk_ids = chunk_ids.len(),
            entity_count = context.entities.len(),
            relationship_count = context.relationships.len(),
            "OODA-230: Global mode chunk collection (workspace)"
        );

        // Retrieve chunks from workspace vector storage using chunk IDs
        if !chunk_ids.is_empty() {
            // WHY: Pass ALL candidate chunk IDs to vector storage and let cosine similarity
            // rank them. VectorStorage.query() returns results sorted by score descending.
            let chunk_ids_vec: Vec<String> = chunk_ids.into_iter().collect();

            let results = vector_storage
                .query_filtered(
                    &embeddings.high_level,
                    self.config.max_chunks,
                    Some(&chunk_ids_vec),
                    mf.as_ref(),
                )
                .await?;

            for result in results {
                context.add_chunk(build_chunk_from_result(&result));
            }
        }

        Ok(context)
    }

    /// Normalize Mix-mode weights to sum to 1 (P-G8).
    ///
    /// E24: if all weights are 0 (or non-finite), fall back to equal weights
    /// (1/3 each) and log a warning, so Mix never silently returns an empty
    /// blend. Returns `(w_local, w_global, w_naive)`.
    fn normalized_mix_weights(&self) -> (f32, f32, f32) {
        let l = self.config.mix_local_weight;
        let g = self.config.mix_global_weight;
        let n = self.config.mix_naive_weight;
        let sum = l + g + n;
        if !sum.is_finite() || sum <= 0.0 {
            tracing::warn!(
                mix_local_weight = l,
                mix_global_weight = g,
                mix_naive_weight = n,
                "Mix weights sum to 0 or are non-finite; falling back to equal weights (P-G8 E24)"
            );
            (1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0)
        } else {
            (l / sum, g / sum, n / sum)
        }
    }

    /// Hybrid mode with workspace-specific vector storage.
    pub(super) async fn query_hybrid_with_vector_storage(
        &self,
        keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
        vector_storage: &Arc<dyn VectorStorage>,
    ) -> Result<QueryContext> {
        // WHY: Run entity-based (local+global) AND naive chunk retrieval in parallel.
        // Entity-based retrieval provides graph context (entities, relationships),
        // but ONLY finds chunks linked to matching entities.
        // Naive retrieval finds chunks by direct semantic similarity to the query,
        // ensuring high recall even when entity extraction doesn't match.
        let (local_context, global_context, naive_context) = tokio::join!(
            self.query_local_with_vector_storage(
                keywords,
                embeddings,
                tenant_id.clone(),
                workspace_id.clone(),
                vector_storage,
            ),
            self.query_global_with_vector_storage(
                keywords,
                embeddings,
                tenant_id.clone(),
                workspace_id.clone(),
                vector_storage,
            ),
            self.query_naive_with_vector_storage(
                embeddings,
                tenant_id.clone(),
                workspace_id.clone(),
                vector_storage,
            ),
        );

        let local_context = local_context?;
        let global_context = global_context?;
        let naive_context = naive_context?;

        tracing::debug!(
            naive_chunks = naive_context.chunks.len(),
            local_chunks = local_context.chunks.len(),
            local_entities = local_context.entities.len(),
            global_chunks = global_context.chunks.len(),
            global_entities = global_context.entities.len(),
            "Hybrid merge: round-robin (local, global, naive)"
        );

        // WHY: Round-robin interleave chunks from local, global, and naive sources.
        // KG-derived chunks (local, global) go first at each position since they carry
        // entity/relationship context. The old approach gave naive all slots first,
        // which could starve KG-derived chunks even when they were more relevant.
        let mut merged = QueryContext::new();
        let mut seen_chunks = std::collections::HashSet::new();
        let max_chunk_len = local_context
            .chunks
            .len()
            .max(global_context.chunks.len())
            .max(naive_context.chunks.len());

        for i in 0..max_chunk_len {
            // KG-derived first (higher signal), then naive (broader recall)
            if let Some(c) = local_context.chunks.get(i) {
                if seen_chunks.insert(c.id.clone()) {
                    merged.add_chunk(c.clone());
                }
            }
            if let Some(c) = global_context.chunks.get(i) {
                if seen_chunks.insert(c.id.clone()) {
                    merged.add_chunk(c.clone());
                }
            }
            if let Some(c) = naive_context.chunks.get(i) {
                if seen_chunks.insert(c.id.clone()) {
                    merged.add_chunk(c.clone());
                }
            }
        }

        // Round-robin entities from local+global
        let mut seen_entities = std::collections::HashSet::new();
        let max_entity_len = local_context
            .entities
            .len()
            .max(global_context.entities.len());
        for i in 0..max_entity_len {
            if let Some(e) = local_context.entities.get(i) {
                if seen_entities.insert(e.name.clone()) {
                    merged.add_entity(e.clone());
                }
            }
            if let Some(e) = global_context.entities.get(i) {
                if seen_entities.insert(e.name.clone()) {
                    merged.add_entity(e.clone());
                }
            }
        }

        // Add relationships from local+global (dedup by key)
        let mut seen_rels = std::collections::HashSet::new();
        for rel in local_context
            .relationships
            .iter()
            .chain(global_context.relationships.iter())
        {
            let key = format!("{}-{}-{}", rel.source, rel.relation_type, rel.target);
            if seen_rels.insert(key) {
                merged.add_relationship(rel.clone());
            }
        }

        tracing::debug!(
            merged_chunks = merged.chunks.len(),
            merged_entities = merged.entities.len(),
            merged_relationships = merged.relationships.len(),
            "Hybrid merge complete (round-robin)"
        );

        Ok(merged)
    }

    /// Mix mode with workspace-specific vector storage.
    ///
    /// P-G8 / RC-13: a *real* weighted blend, not a Hybrid alias. Runs the same
    /// three arms as Hybrid (local, global, naive) in parallel, then blends them
    /// by weighted score instead of round-robin:
    /// - Each arm's chunk scores are min-max normalized to [0,1] (so an arm with
    ///   uniformly high cosine scores cannot dominate an arm with a wider range).
    /// - Each chunk's blended score = Σ arm_weight * normalized_score, where the
    ///   chunk takes its score from every arm that returned it (0 from arms that
    ///   did not). Weights are normalized to sum to 1 at use; an arm with weight 0
    ///   contributes nothing (E25).
    /// - Chunks are then sorted by blended score (descending) and truncated to
    ///   `max_chunks`. Entities and relationships are unioned across local+global
    ///   (dedup by name / by (src,rel,tgt)), the same as Hybrid, because their
    ///   scores are not cross-arm comparable.
    ///
    /// When all three weights are equal (the default), the weighted-sum ordering
    /// degenerates to the same ranking as Hybrid's round-robin on identical
    /// fixtures, preserving backward compatibility (E24: weights sum to 0 → fall
    /// back to equal weights).
    pub(super) async fn query_mix_with_vector_storage(
        &self,
        keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
        vector_storage: &Arc<dyn VectorStorage>,
    ) -> Result<QueryContext> {
        let (local_context, global_context, naive_context) = tokio::join!(
            self.query_local_with_vector_storage(
                keywords,
                embeddings,
                tenant_id.clone(),
                workspace_id.clone(),
                vector_storage,
            ),
            self.query_global_with_vector_storage(
                keywords,
                embeddings,
                tenant_id.clone(),
                workspace_id.clone(),
                vector_storage,
            ),
            self.query_naive_with_vector_storage(
                embeddings,
                tenant_id,
                workspace_id,
                vector_storage,
            ),
        );

        let local_context = local_context?;
        let global_context = global_context?;
        let naive_context = naive_context?;

        let (w_local, w_global, w_naive) = self.normalized_mix_weights();

        // Blend chunks by weighted normalized score.
        let mut blended: HashMap<String, (RetrievedChunk, f32)> = HashMap::new();
        for (ctx, weight) in [
            (&local_context, w_local),
            (&global_context, w_global),
            (&naive_context, w_naive),
        ] {
            let norm = min_max_normalize_scores(&ctx.chunks);
            for (chunk, &norm_score) in ctx.chunks.iter().zip(norm.iter()) {
                let contribution = weight * norm_score;
                blended
                    .entry(chunk.id.clone())
                    .and_modify(|(_, score)| {
                        if contribution > *score {
                            *score = contribution;
                        }
                    })
                    .or_insert_with(|| (chunk.clone(), contribution));
            }
        }

        let mut merged = QueryContext::new();
        let mut chunks: Vec<(RetrievedChunk, f32)> = blended.into_values().collect();
        chunks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let max_chunks = self.config.max_chunks;
        for (mut chunk, _) in chunks.into_iter().take(max_chunks) {
            // Preserve the blended score on the emitted chunk so downstream
            // reranking/truncation sees a monotonic relevance signal.
            chunk.score = chunk.score.max(0.0);
            merged.add_chunk(chunk);
        }

        // Entities + relationships: union across local+global (same as Hybrid;
        // their scores are not cross-arm comparable).
        let mut seen_entities = std::collections::HashSet::new();
        for e in local_context
            .entities
            .iter()
            .chain(global_context.entities.iter())
        {
            if seen_entities.insert(e.name.clone()) {
                merged.add_entity(e.clone());
            }
        }
        let mut seen_rels = std::collections::HashSet::new();
        for rel in local_context
            .relationships
            .iter()
            .chain(global_context.relationships.iter())
        {
            let key = format!("{}-{}-{}", rel.source, rel.relation_type, rel.target);
            if seen_rels.insert(key) {
                merged.add_relationship(rel.clone());
            }
        }

        tracing::debug!(
            merged_chunks = merged.chunks.len(),
            merged_entities = merged.entities.len(),
            merged_relationships = merged.relationships.len(),
            w_local,
            w_global,
            w_naive,
            "Mix merge complete (weighted blend)"
        );

        Ok(merged)
    }
}

/// Per-arm min-max normalization of chunk scores to [0,1].
///
/// First principles: cosine/dot scores are not comparable across arms (local
/// scores chunks via entity proximity, naive scores them via direct similarity).
/// Min-max normalization puts each arm on a common [0,1] scale before blending.
/// An arm with a single chunk (or all-equal scores) maps to 1.0 (max == min →
/// every score becomes 1.0), which is the correct "no information to rank
/// within arm" case.
fn min_max_normalize_scores(chunks: &[crate::context::RetrievedChunk]) -> Vec<f32> {
    if chunks.is_empty() {
        return Vec::new();
    }
    let (min, max) = chunks
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(mn, mx), c| {
            (mn.min(c.score), mx.max(c.score))
        });
    let range = max - min;
    chunks
        .iter()
        .map(|c| {
            if range <= 0.0 {
                1.0
            } else {
                (c.score - min) / range
            }
        })
        .collect()
}
