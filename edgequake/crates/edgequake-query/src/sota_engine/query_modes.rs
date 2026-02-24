use std::collections::HashMap;

use crate::context::{QueryContext, RetrievedRelationship};
use crate::error::Result;
use crate::helpers::{
    build_chunk_from_result, build_entity_from_node, build_relationship_from_edge,
};
use crate::keywords::ExtractedKeywords;
use crate::vector_filter::{filter_by_type, VectorType};

use super::{QueryEmbeddings, SOTAQueryEngine};

impl SOTAQueryEngine {
    /// Local mode: Entity-centric search with low-level keywords.
    ///
    /// # WHY: Local Mode Strategy
    ///
    /// Local mode answers specific factual questions (e.g., "Who is the CEO of Apple?"):
    ///
    /// 1. **Low-level embedding** - Uses entity-focused keywords ("Apple", "CEO")
    ///    WHY: These keywords match entity descriptions, not relationships
    ///
    /// 2. **Entity vector filter** - Only search entity vectors, ignore relationships
    ///    WHY: Reduces noise; relationships are for Global mode
    ///
    /// 3. **1-hop graph expansion** - Fetch connected entities/relationships
    ///    WHY: Immediate neighbors provide supporting context
    ///
    /// 4. **Degree-based ranking** - Higher-degree entities ranked first
    ///    WHY: Well-connected entities are typically more important
    pub(super) async fn query_local(
        &self,
        _keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();

        // Step 1: Vector search with LOW-level keyword embedding
        // This finds entities relevant to specific terms
        let vector_results = self
            .vector_storage
            .query(&embeddings.low_level, self.config.max_entities * 3, None)
            .await?;

        // Step 2: Filter to entity vectors only (LightRAG Local mode)
        let entity_vectors = filter_by_type(vector_results, VectorType::Entity);

        // Step 2.5: Build entity scores map to preserve vector similarity scores
        let entity_scores: HashMap<String, f32> = entity_vectors
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .filter(|r| self.matches_tenant_filter(&r.metadata, &tenant_id, &workspace_id))
            .map(|r| {
                let entity_name = r
                    .metadata
                    .get("entity_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| r.id.clone());
                (entity_name, r.score)
            })
            .collect();

        // Step 3: Extract entity IDs from vector results
        let entity_ids: Vec<String> = entity_vectors
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .filter(|r| self.matches_tenant_filter(&r.metadata, &tenant_id, &workspace_id))
            .filter_map(|r| {
                // Try to get entity_name from metadata, fallback to id
                r.metadata
                    .get("entity_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| Some(r.id.clone()))
            })
            .take(self.config.max_entities)
            .collect();

        if entity_ids.is_empty() {
            // Fallback to popular entities
            return self.fallback_to_popular(tenant_id, workspace_id).await;
        }

        // Step 4: Batch fetch nodes and degrees (LightRAG optimization)
        let (nodes_map, degrees) = tokio::join!(
            self.graph_storage.get_nodes_batch(&entity_ids),
            self.graph_storage.node_degrees_batch(&entity_ids),
        );

        let nodes_map = nodes_map?;
        let degrees: HashMap<String, usize> = degrees?.into_iter().collect();

        // Step 5: Build entity context with source tracking
        //
        // WHY: Iterate in vector search score order (entity_ids) instead of HashMap iteration.
        // HashMap iteration is non-deterministic, causing same query → different results.
        // By preserving entity_ids order (Vec), we maintain deterministic entity ordering.
        //
        //   Before (Random):              After (Deterministic):
        //   Vector Search                 Vector Search
        //      ↓                             ↓
        //   entity_ids=[A,B,C]           entity_ids=[A,B,C]
        //   (score order)                (score order)
        //      ↓                             ↓
        //   nodes_map={A,C,B}            nodes_map={A,C,B}
        //   (HashMap - random)           (HashMap - lookup only)
        //      ↓                             ↓
        //   for (id,node) in map         for id in entity_ids
        //      ↓                             ↓
        //   [C,A,B] ← RANDOM!            [A,B,C] ← STABLE!
        //
        for id in &entity_ids {
            if let Some(node) = nodes_map.get(id) {
                let degree = degrees.get(id).copied().unwrap_or(0);
                // Use preserved similarity score from vector search (fixes score=0.0 bug)
                let entity_score = entity_scores.get(id).copied().unwrap_or(0.0);
                let entity = build_entity_from_node(id, &node.properties, degree, entity_score);
                context.add_entity(entity);
            }
        }

        // Step 6: Batch fetch edges for these entities
        let edges = self
            .graph_storage
            .get_edges_for_nodes_batch(&entity_ids)
            .await?;

        for edge in edges.iter().take(self.config.max_relationships) {
            if !self.matches_tenant_filter_props(&edge.properties, &tenant_id, &workspace_id) {
                continue;
            }

            let rel = build_relationship_from_edge(&edge.source, &edge.target, &edge.properties);
            context.add_relationship(rel);
        }

        // Step 7: Retrieve chunks from source_chunk_ids
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
            "Local mode chunk collection"
        );

        // Retrieve chunks from vector storage if any chunk IDs were collected
        if !chunk_ids.is_empty() {
            // WHY: Pass ALL candidate chunk IDs to vector storage and let cosine similarity
            // determine the best max_chunks. The old approach sorted alphabetically and
            // truncated before scoring, which could discard high-relevance chunks.
            // VectorStorage.query() returns results sorted by score descending (contract).
            let chunk_ids_vec: Vec<String> = chunk_ids.into_iter().collect();

            let results = self
                .vector_storage
                .query(
                    &embeddings.low_level,
                    self.config.max_chunks,
                    Some(&chunk_ids_vec),
                )
                .await?;

            for result in results {
                if !self.matches_tenant_filter(&result.metadata, &tenant_id, &workspace_id) {
                    continue;
                }

                context.add_chunk(build_chunk_from_result(&result));
            }
        }

        Ok(context)
    }

    /// Global mode: Relationship-centric search with high-level keywords.
    ///
    /// # WHY: Global Mode Strategy
    ///
    /// Global mode answers thematic/analytical questions (e.g., "How do tech companies compete?"):
    ///
    /// 1. **High-level embedding** - Uses relationship-focused keywords ("compete", "partnership")
    ///    WHY: These keywords match relationship descriptions, not entities
    ///
    /// 2. **Relationship vector filter** - Only search relationship vectors
    ///    WHY: Relationships capture "how" and "why" connections between entities
    ///
    /// 3. **Entity hydration** - Fetch source/target entities for each relationship
    ///    WHY: Relationships are meaningless without their endpoint context
    ///
    /// 4. **Community summaries** - Include pre-computed graph cluster summaries
    ///    WHY: Provides high-level thematic context for broad questions
    ///
    /// @implements FEAT0102 (Global Search Mode - relationship-focused retrieval)
    pub(super) async fn query_global(
        &self,
        _keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();
        let mut entity_ids: Vec<String> = Vec::new();
        let mut seen_relationships = std::collections::HashSet::new();

        // Step 1: Vector search with HIGH-level keyword embedding
        // This finds relationships relevant to broader concepts
        let vector_results = self
            .vector_storage
            .query(
                &embeddings.high_level,
                self.config.max_relationships * 3,
                None,
            )
            .await?;

        // Step 2: Filter to relationship vectors only (LightRAG Global mode)
        let relationship_vectors = filter_by_type(vector_results.clone(), VectorType::Relationship);

        // Step 3: Extract relationships from vector results
        for result in relationship_vectors
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .filter(|r| self.matches_tenant_filter(&r.metadata, &tenant_id, &workspace_id))
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
                    // Extract source tracking from vector metadata
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
                    // Collect entity IDs from relationships
                    if !entity_ids.contains(&src_id.to_string()) {
                        entity_ids.push(src_id.to_string());
                    }
                    if !entity_ids.contains(&tgt_id.to_string()) {
                        entity_ids.push(tgt_id.to_string());
                    }
                }
            }
        }

        // Step 4: Fallback to popular entities if no relationship vectors found
        if entity_ids.is_empty() {
            let popular = self
                .graph_storage
                .get_popular_nodes_with_degree(
                    self.config.max_entities,
                    Some(2), // Min degree
                    None,
                    tenant_id.as_deref(),
                    workspace_id.as_deref(),
                )
                .await?;

            entity_ids = popular.iter().map(|(n, _)| n.id.clone()).collect();

            for (node, degree) in popular {
                let entity = build_entity_from_node(&node.id, &node.properties, degree, 0.0);
                context.add_entity(entity);
            }

            // Get edges between popular entities
            if !entity_ids.is_empty() {
                let edges = self
                    .graph_storage
                    .get_edges_for_nodes_batch(&entity_ids)
                    .await?;
                for edge in edges.iter().take(self.config.max_relationships) {
                    let rel =
                        build_relationship_from_edge(&edge.source, &edge.target, &edge.properties);
                    context.add_relationship(rel);
                }
            }
        } else {
            // Step 5: Batch fetch entities from relationship endpoints
            let (nodes_map, degrees) = tokio::join!(
                self.graph_storage.get_nodes_batch(&entity_ids),
                self.graph_storage.node_degrees_batch(&entity_ids),
            );

            let nodes_map = nodes_map?;
            let degrees: HashMap<String, usize> = degrees?.into_iter().collect();

            // WHY: Iterate in relationship discovery order (entity_ids) instead of HashMap.
            // Global mode discovers entities from relationship endpoints in vector search score order.
            // Preserving that order ensures deterministic retrieval (same query → same results).
            for id in &entity_ids {
                if let Some(node) = nodes_map.get(id) {
                    let degree = degrees.get(id).copied().unwrap_or(0);
                    let entity = build_entity_from_node(id, &node.properties, degree, 0.0);
                    context.add_entity(entity);
                }
            }
        }

        // Step 6: Add chunks from vector search (filter to chunks)
        let chunk_vectors = filter_by_type(vector_results, VectorType::Chunk);
        for result in chunk_vectors
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .filter(|r| self.matches_tenant_filter(&r.metadata, &tenant_id, &workspace_id))
            .take(self.config.max_chunks)
        {
            context.add_chunk(build_chunk_from_result(result));
        }

        // Step 7: Also retrieve chunks from source_chunk_ids tracked in entities/relationships
        let mut source_chunk_ids = std::collections::HashSet::new();

        // Collect chunk IDs from entities
        for entity in &context.entities {
            for chunk_id in &entity.source_chunk_ids {
                source_chunk_ids.insert(chunk_id.clone());
            }
        }

        // Collect chunk IDs from relationships
        for rel in &context.relationships {
            if let Some(chunk_id) = &rel.source_chunk_id {
                source_chunk_ids.insert(chunk_id.clone());
            }
        }

        // Retrieve source chunks if any were collected and we haven't hit max chunks
        if !source_chunk_ids.is_empty() && context.chunks.len() < self.config.max_chunks {
            let remaining_slots = self.config.max_chunks - context.chunks.len();

            // WHY: Pass ALL candidate chunk IDs to vector storage and let cosine similarity
            // rank them. VectorStorage.query() returns results sorted by score descending.
            let chunk_ids_vec: Vec<String> = source_chunk_ids.into_iter().collect();

            let results = self
                .vector_storage
                .query(
                    &embeddings.high_level,
                    remaining_slots,
                    Some(&chunk_ids_vec),
                )
                .await?;

            // Track which chunks we already have to avoid duplicates
            let existing_chunk_ids: std::collections::HashSet<_> =
                context.chunks.iter().map(|c| c.id.clone()).collect();

            for result in results {
                if existing_chunk_ids.contains(&result.id) {
                    continue;
                }
                if !self.matches_tenant_filter(&result.metadata, &tenant_id, &workspace_id) {
                    continue;
                }

                context.add_chunk(build_chunk_from_result(&result));
            }
        }

        Ok(context)
    }

    /// Hybrid mode: Combine local and global with round-robin merging.
    ///
    /// @implements FEAT0103 (Hybrid Search Mode - combined local+global)
    pub(super) async fn query_hybrid(
        &self,
        keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Result<QueryContext> {
        // Run local and global in parallel
        let (local_result, global_result) = tokio::join!(
            self.query_local(
                keywords,
                embeddings,
                tenant_id.clone(),
                workspace_id.clone()
            ),
            self.query_global(
                keywords,
                embeddings,
                tenant_id.clone(),
                workspace_id.clone()
            ),
        );

        let local = local_result?;
        let global = global_result?;

        // Round-robin merge with deduplication
        let mut context = QueryContext::new();
        let mut seen_entities = std::collections::HashSet::new();
        let mut seen_relationships = std::collections::HashSet::new();

        // Interleave entities
        let max_len = local.entities.len().max(global.entities.len());
        for i in 0..max_len {
            if let Some(e) = local.entities.get(i) {
                if seen_entities.insert(e.name.clone()) {
                    context.add_entity(e.clone());
                }
            }
            if let Some(e) = global.entities.get(i) {
                if seen_entities.insert(e.name.clone()) {
                    context.add_entity(e.clone());
                }
            }
        }

        // Interleave relationships
        let max_len = local.relationships.len().max(global.relationships.len());
        for i in 0..max_len {
            if let Some(r) = local.relationships.get(i) {
                let key = format!("{}-{}-{}", r.source, r.relation_type, r.target);
                if seen_relationships.insert(key) {
                    context.add_relationship(r.clone());
                }
            }
            if let Some(r) = global.relationships.get(i) {
                let key = format!("{}-{}-{}", r.source, r.relation_type, r.target);
                if seen_relationships.insert(key) {
                    context.add_relationship(r.clone());
                }
            }
        }

        // WHY: Round-robin interleave chunks for balanced source diversity.
        // The old approach chained local-then-global, giving local chunks priority.
        // Round-robin ensures the top chunk from each source is represented first,
        // matching the entity/relationship interleaving pattern above.
        let mut seen_chunks = std::collections::HashSet::new();
        let max_chunk_len = local.chunks.len().max(global.chunks.len());
        for i in 0..max_chunk_len {
            if let Some(c) = local.chunks.get(i) {
                if seen_chunks.insert(c.id.clone()) {
                    context.add_chunk(c.clone());
                }
            }
            if let Some(c) = global.chunks.get(i) {
                if seen_chunks.insert(c.id.clone()) {
                    context.add_chunk(c.clone());
                }
            }
        }

        Ok(context)
    }

    /// Mix mode: Hybrid plus direct chunk search.
    ///
    /// @implements FEAT0105 (Mix Weighted Search - hybrid + direct chunks)
    pub(super) async fn query_mix(
        &self,
        keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Result<QueryContext> {
        // Run hybrid and direct chunk search in parallel
        let (hybrid_result, chunk_results) = tokio::join!(
            self.query_hybrid(
                keywords,
                embeddings,
                tenant_id.clone(),
                workspace_id.clone()
            ),
            self.vector_storage
                .query(&embeddings.query, self.config.max_chunks * 2, None),
        );

        let mut context = hybrid_result?;
        let chunk_results = chunk_results?;

        // Filter to chunk vectors only
        let chunk_vectors = filter_by_type(chunk_results, VectorType::Chunk);

        // Add direct chunks (deduplicated)
        let existing_chunk_ids: std::collections::HashSet<_> =
            context.chunks.iter().map(|c| c.id.clone()).collect();

        for result in chunk_vectors
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .filter(|r| self.matches_tenant_filter(&r.metadata, &tenant_id, &workspace_id))
            .take(self.config.max_chunks)
        {
            if !existing_chunk_ids.contains(&result.id) {
                context.add_chunk(build_chunk_from_result(result));
            }
        }

        Ok(context)
    }

    /// Naive mode: Direct chunk vector search only.
    ///
    /// @implements FEAT0106 (Bypass Mode - direct vector search without graph)
    pub(super) async fn query_naive(
        &self,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();

        let results = self
            .vector_storage
            .query(&embeddings.query, self.config.max_chunks * 2, None)
            .await?;

        // Filter to chunk vectors only
        let chunk_results = filter_by_type(results, VectorType::Chunk);

        for result in chunk_results
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .filter(|r| self.matches_tenant_filter(&r.metadata, &tenant_id, &workspace_id))
            .take(self.config.max_chunks)
        {
            context.add_chunk(build_chunk_from_result(result));
        }

        Ok(context)
    }

    /// Fallback to popular entities when no vector matches.
    pub(super) async fn fallback_to_popular(
        &self,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();

        let popular = self
            .graph_storage
            .get_popular_nodes_with_degree(
                self.config.max_entities,
                None,
                None,
                tenant_id.as_deref(),
                workspace_id.as_deref(),
            )
            .await?;

        let entity_ids: Vec<String> = popular.iter().map(|(n, _)| n.id.clone()).collect();

        for (node, degree) in popular {
            let entity = build_entity_from_node(&node.id, &node.properties, degree, 0.0);
            context.add_entity(entity);
        }

        // Get edges
        if !entity_ids.is_empty() {
            let edges = self
                .graph_storage
                .get_edges_for_nodes_batch(&entity_ids)
                .await?;
            for edge in edges.iter().take(self.config.max_relationships) {
                let rel =
                    build_relationship_from_edge(&edge.source, &edge.target, &edge.properties);
                context.add_relationship(rel);
            }
        }

        Ok(context)
    }
}
