//! Relationship merge, update, creation, and placeholder node logic.
//!
//! # SPEC-047 P7b / P7d
//! - Parallel unique-edge description merges (`merge_max_async`)
//! - SOURCE_IDS KEEP: skip saturated description updates

use std::collections::HashMap;

use edgequake_storage::{EntityId, GraphEdge, GraphStorage, VectorStorage};
use futures::stream::{self, StreamExt};

use crate::error::Result;
use crate::extractor::ExtractedRelationship;

use super::merge_limits::{
    apply_source_ids_limit, merge_source_ids, should_skip_description_update_keep,
    source_chunk_ids_from_properties,
};
use super::merge_progress::{self, MergeProgressCtx};
use super::{metadata, MergeStats, RelationLineageLink};

/// One unique-edge merge result (P7b parallel collect).
struct RelMergeOutcome {
    source: String,
    target: String,
    properties: HashMap<String, serde_json::Value>,
    is_new: bool,
    skipped_saturated: bool,
    vector_id: Option<String>,
    /// Updated edge for map coherence (None when saturated skip).
    edge_for_map: Option<GraphEdge>,
}

/// New AGE placeholder awaiting entities_vdb upsert (050 B7).
struct PlaceholderVdbSpec {
    label: String,
    description: String,
    chunk_ids: Vec<String>,
}

/// Keep the longest incident relation description (LightRAG node_description).
fn retain_longest_description(map: &mut HashMap<String, String>, key: &str, candidate: &str) {
    if candidate.is_empty() {
        return;
    }
    let entry = map.entry(key.to_string()).or_default();
    if candidate.len() > entry.len() {
        *entry = candidate.to_string();
    }
}

impl<G: GraphStorage + ?Sized, V: VectorStorage + ?Sized> super::KnowledgeGraphMerger<G, V> {
    /// Collect batched relationship vector upserts (P-G4-merger).
    ///
    /// SPEC-047 P6: dedupe by `(src, tgt, type)` before upsert (O(unique)).
    pub(super) fn collect_relationship_vector_batch(
        &self,
        relationships: &[ExtractedRelationship],
    ) -> Vec<(String, Vec<f32>, serde_json::Value)> {
        let unique = crate::pipeline::helpers::unique_embed::dedupe_relationships_by_endpoints(
            relationships,
        );
        let mut batch = Vec::with_capacity(unique.len());
        for rel in &unique {
            let source_id = EntityId::new(&rel.source);
            let target_id = EntityId::new(&rel.target);
            // Vector metadata keeps bare names; graph endpoints use scoped ids.
            let source_bare = source_id.as_graph_node_id();
            let target_bare = target_id.as_graph_node_id();
            if source_bare.is_empty() || target_bare.is_empty() || source_bare == target_bare {
                continue;
            }
            let Some(embedding) = rel.embedding.as_ref() else {
                continue;
            };
            // SPEC-098 LAW-098-3: vector id relation_type matches sink uppercase SSOT.
            // SPEC-130: legacy key formatter SSOT shared with sink report.
            let rel_id = edgequake_storage::format_relationship_legacy_key(
                source_bare,
                target_bare,
                &rel.relation_type,
            );
            let scope = metadata::TenantScope {
                tenant_id: &self.tenant_id,
                workspace_id: &self.workspace_id,
            };
            let metadata =
                metadata::relationship_vector_metadata(rel, source_bare, target_bare, scope);
            batch.push((rel_id, embedding.clone(), metadata));
        }
        batch
    }

    /// Merge relationships with batched graph reads/writes (P-G4-graph).
    ///
    /// SPEC-045: Emits incremental merge progress per chunk and batches lineage
    /// writes so the UI advances during long runs (fixes frozen 0/N counters).
    ///
    /// # Within-batch dedup (SPEC-083 D-30 multigraph)
    ///
    /// Arbiter is `(source, target, relation_type)`. Same endpoints with
    /// different types both persist. Same type collapses (last-write + weight
    /// policy) before `upsert_edges_batch`.
    pub(super) async fn merge_relationships_batch(
        &self,
        relationships: Vec<ExtractedRelationship>,
        stats: &mut MergeStats,
        progress: Option<MergeProgressCtx<'_>>,
    ) -> Result<crate::merger::RelationshipSinkReport> {
        if relationships.is_empty() {
            return Ok(crate::merger::RelationshipSinkReport::default());
        }

        let mut valid = Vec::new();
        let mut endpoint_keys = Vec::new();

        let ws = self.workspace_id.as_deref();
        for rel in relationships {
            let source_id = EntityId::new(&rel.source);
            let target_id = EntityId::new(&rel.target);
            let source_key = source_id.graph_node_id_for_workspace(ws);
            let target_key = target_id.graph_node_id_for_workspace(ws);

            if source_key == target_key {
                tracing::debug!(
                    source = %source_key,
                    "Merger: skipping self-referencing relationship (BR0006)"
                );
                continue;
            }
            if source_key.is_empty() || target_key.is_empty() {
                tracing::debug!(
                    raw_source = %rel.source,
                    raw_target = %rel.target,
                    "Merger: skipping relationship with empty normalized endpoint"
                );
                continue;
            }

            if !endpoint_keys.contains(&source_key) {
                endpoint_keys.push(source_key.clone());
            }
            if !endpoint_keys.contains(&target_key) {
                endpoint_keys.push(target_key.clone());
            }
            valid.push((rel, source_key, target_key));
        }

        if valid.is_empty() {
            return Ok(crate::merger::RelationshipSinkReport::default());
        }

        // SPEC-162 R13 / F-162-C: remap endpoints onto fold-equivalent nodes.
        {
            let mut remap_keys = endpoint_keys.clone();
            let mut existing_map = self.graph_storage.get_nodes_batch(&remap_keys).await?;
            super::key_resolver::resolve_fold_keys(
                self.graph_storage.as_ref(),
                ws,
                &mut remap_keys,
                &mut existing_map,
            )
            .await?;
            let remap: std::collections::HashMap<String, String> =
                endpoint_keys.iter().cloned().zip(remap_keys).collect();
            for (_, sk, tk) in &mut valid {
                if let Some(n) = remap.get(sk) {
                    *sk = n.clone();
                }
                if let Some(n) = remap.get(tk) {
                    *tk = n.clone();
                }
            }
            endpoint_keys = remap.values().cloned().collect();
            endpoint_keys.sort();
            endpoint_keys.dedup();
        }

        // Domain dedup: one ExtractedRelationship per (source, target, rel_type).
        let valid = dedupe_relationships_by_endpoints(valid);

        let total_valid = valid.len();
        if let Some(ctx) = progress {
            ctx.emit_relationship_graph(
                0,
                stats.relationships_created,
                stats.relationships_updated,
            );
        }

        let existing_nodes = self.graph_storage.get_nodes_batch(&endpoint_keys).await?;
        let incident_edges = self
            .graph_storage
            .get_edges_for_nodes_batch(&endpoint_keys)
            .await?;
        // D-30: key includes relation_type so KNOWS ≠ WORKS_WITH.
        let mut edge_map: HashMap<(String, String, String), GraphEdge> = HashMap::new();
        for edge in incident_edges {
            let rel = edgequake_storage::normalize_rel_type(&edge.properties);
            edge_map.insert((edge.source.clone(), edge.target.clone(), rel), edge);
        }

        // Horizon B5 (044): relation endpoints missing a full entity extract still
        // inherit the relation's source_chunk_id — GraphRAG/LightRAG provenance law.
        // Horizon B7 (050): also seed description from the longest incident relation
        // description and upsert entities_vdb (LightRAG operate.py ~1916).
        // Also enrich existing zero-chunk stubs when later relation batches arrive.
        let mut placeholder_batch: Vec<(String, HashMap<String, serde_json::Value>)> = Vec::new();
        let mut placeholders: HashMap<String, String> = HashMap::new();
        let mut placeholder_chunk_ids: HashMap<String, Vec<String>> = HashMap::new();
        let mut placeholder_descriptions: HashMap<String, String> = HashMap::new();
        let mut stub_enrich_chunk_ids: HashMap<String, Vec<String>> = HashMap::new();
        for (rel, source_key, target_key) in &valid {
            let rel_chunk_ids = rel.all_source_chunk_ids();
            for (key, raw_label) in [
                (source_key, rel.source.as_str()),
                (target_key, rel.target.as_str()),
            ] {
                if rel_chunk_ids.is_empty() {
                    if !existing_nodes.contains_key(key) {
                        placeholders
                            .entry(key.clone())
                            .or_insert_with(|| raw_label.to_string());
                        retain_longest_description(
                            &mut placeholder_descriptions,
                            key,
                            &rel.description,
                        );
                    }
                    continue;
                }
                if let Some(node) = existing_nodes.get(key) {
                    let existing_ids = source_chunk_ids_from_properties(&node.properties);
                    if existing_ids.is_empty() {
                        let ids = stub_enrich_chunk_ids.entry(key.clone()).or_default();
                        for chunk_id in &rel_chunk_ids {
                            if !ids.contains(chunk_id) {
                                ids.push(chunk_id.clone());
                            }
                        }
                    }
                } else {
                    placeholders
                        .entry(key.clone())
                        .or_insert_with(|| raw_label.to_string());
                    retain_longest_description(
                        &mut placeholder_descriptions,
                        key,
                        &rel.description,
                    );
                    let ids = placeholder_chunk_ids.entry(key.clone()).or_default();
                    for chunk_id in &rel_chunk_ids {
                        if !ids.contains(chunk_id) {
                            ids.push(chunk_id.clone());
                        }
                    }
                }
            }
        }
        let mut new_placeholder_specs: Vec<PlaceholderVdbSpec> = Vec::new();
        for (key, raw_label) in placeholders {
            let label = EntityId::new(&raw_label).as_str().to_string();
            let label = if label.is_empty() { raw_label } else { label };
            let chunk_ids = placeholder_chunk_ids.get(&key).cloned().unwrap_or_default();
            let description = placeholder_descriptions
                .get(&key)
                .cloned()
                .unwrap_or_default();
            placeholder_batch.push((
                key.clone(),
                self.placeholder_node_properties(&label, &chunk_ids, &description),
            ));
            new_placeholder_specs.push(PlaceholderVdbSpec {
                label,
                description,
                chunk_ids,
            });
            stats.artifacts.graph_nodes_created.push(key);
        }
        for (key, chunk_ids) in stub_enrich_chunk_ids {
            if let Some(node) = existing_nodes.get(&key) {
                let mut props = node.properties.clone();
                // D-33: document lineage from full chunk set, then cap stored chunk ids.
                super::lineage::merge_and_insert_document_lineage(&mut props, None, &chunk_ids);
                let capped = apply_source_ids_limit(
                    &chunk_ids,
                    self.config.max_source_ids_per_entity,
                    self.config.source_ids_limit_method,
                );
                super::lineage::insert_chunk_lineage_properties(&mut props, &capped);
                placeholder_batch.push((key, props));
            }
        }

        if !placeholder_batch.is_empty() {
            self.graph_storage
                .upsert_nodes_batch(&placeholder_batch)
                .await?;
        }

        // B7: AGE placeholders without entity extract must still be Local/Mix-retrievable.
        if !new_placeholder_specs.is_empty() {
            self.upsert_placeholder_entity_vectors(&new_placeholder_specs)
                .await?;
        }

        let mut edge_batch: Vec<(String, String, HashMap<String, serde_json::Value>)> =
            Vec::with_capacity(total_valid);

        let ws = self
            .workspace_id
            .as_deref()
            .unwrap_or("default")
            .to_string();
        let chunk_size = merge_progress::relationship_merge_chunk_size();
        let concurrency = self.config.merge_max_async.max(1);

        for chunk_start in (0..total_valid).step_by(chunk_size) {
            let chunk_end = (chunk_start + chunk_size).min(total_valid);
            let mut lineage_batch: Vec<RelationLineageLink> = Vec::new();

            let works: Vec<(
                usize,
                ExtractedRelationship,
                String,
                String,
                Option<GraphEdge>,
            )> = valid[chunk_start..chunk_end]
                .iter()
                .enumerate()
                .map(|(offset, (rel, source_key, target_key))| {
                    let rel_type = if rel.relation_type.trim().is_empty() {
                        "RELATED_TO".to_string()
                    } else {
                        rel.relation_type.trim().to_ascii_uppercase()
                    };
                    let key = (source_key.clone(), target_key.clone(), rel_type);
                    let existing = edge_map.get(&key).cloned();
                    (
                        chunk_start + offset,
                        rel.clone(),
                        source_key.clone(),
                        target_key.clone(),
                        existing,
                    )
                })
                .collect();

            for (rel, source_key, target_key) in &valid[chunk_start..chunk_end] {
                for chunk_id in rel.all_source_chunk_ids() {
                    lineage_batch.push(RelationLineageLink {
                        chunk_id,
                        source_entity: source_key.clone(),
                        target_entity: target_key.clone(),
                        workspace_id: ws.clone(),
                    });
                }
            }

            // SPEC-047 P7b: parallel unique-edge description merges within chunk.
            let outcomes: Vec<Result<(usize, RelMergeOutcome)>> = stream::iter(works)
                .map(|(idx, rel, source_key, target_key, existing)| async move {
                    let outcome = self
                        .build_relationship_merge_outcome(&rel, &source_key, &target_key, existing)
                        .await?;
                    Ok((idx, outcome))
                })
                .buffer_unordered(concurrency)
                .collect()
                .await;

            let mut ordered: Vec<(usize, RelMergeOutcome)> = Vec::with_capacity(outcomes.len());
            for item in outcomes {
                match item {
                    Ok(pair) => ordered.push(pair),
                    Err(e) => {
                        stats.record_error(e.to_string());
                        tracing::warn!(
                            error.source = "pipeline_merger",
                            error.action = "merge_relationship",
                            error.message = %e,
                            "Failed to merge relationship"
                        );
                    }
                }
            }
            ordered.sort_by_key(|(i, _)| *i);

            for (_idx, outcome) in ordered {
                if outcome.skipped_saturated {
                    stats.relationships_skipped_saturated += 1;
                    continue;
                }
                // Relationship vector IDs are recorded in upsert_vectors_chunked (SPEC-057 P3).
                let _ = outcome.vector_id;
                if outcome.is_new {
                    stats
                        .artifacts
                        .graph_edges_created
                        .push((outcome.source.clone(), outcome.target.clone()));
                    stats.relationships_created += 1;
                } else {
                    stats.relationships_updated += 1;
                }
                if let Some(edge) = outcome.edge_for_map {
                    let rel = edgequake_storage::normalize_rel_type(&edge.properties);
                    edge_map.insert((edge.source.clone(), edge.target.clone(), rel), edge);
                }
                edge_batch.push((outcome.source, outcome.target, outcome.properties));
            }

            if !lineage_batch.is_empty() {
                self.lineage_sink
                    .record_relation_links_batch(&lineage_batch)
                    .await
                    .unwrap_or_else(|e| {
                        tracing::warn!(
                            count = lineage_batch.len(),
                            error = %e,
                            "Lineage sink record_relation_links_batch failed (best-effort)"
                        );
                    });
            }

            if let Some(ctx) = progress {
                ctx.emit_relationship_graph(
                    chunk_end,
                    stats.relationships_created,
                    stats.relationships_updated,
                );
            }
        }

        // Storage-layer dedupe is the SSOT for ON CONFLICT safety; still collapse
        // here so create/update stats and artifacts stay aligned with one edge/pair.
        let edge_batch = edgequake_storage::dedupe_edges_by_endpoints(&edge_batch);

        if !edge_batch.is_empty() {
            if let Some(ctx) = progress {
                ctx.emit_relationship_graph(
                    total_valid,
                    stats.relationships_created,
                    stats.relationships_updated,
                );
            }
            self.graph_storage.upsert_edges_batch(&edge_batch).await?;
        }

        // SPEC-091 IP1: relational relationships spine — one batch (LAW-IP2).
        // SPEC-098 LAW-098-2: sink from full `valid` (including saturated) so
        // relational spine exists before RelVectors fleet mirror.
        let rel_sink_rows: Vec<crate::merger::RelationshipSinkRow> = valid
            .iter()
            .map(|(rel, source_key, target_key)| {
                let rel_type = edgequake_storage::normalize_relation_type_str(&rel.relation_type);
                crate::merger::RelationshipSinkRow {
                    source_name: source_key.clone(),
                    target_name: target_key.clone(),
                    relation_type: rel_type,
                    description: rel.description.clone(),
                    weight: rel.weight,
                    tenant_id: self.tenant_id.clone(),
                    workspace_id: self.workspace_id.clone(),
                    source_chunk_ids: rel.all_source_chunk_ids(),
                }
            })
            .collect();
        let mut sink_report = crate::merger::RelationshipSinkReport::default();
        if !rel_sink_rows.is_empty() {
            match self
                .relational_sink
                .upsert_relationships_batch(&rel_sink_rows)
                .await
            {
                Ok(report) => sink_report = report,
                Err(e) => {
                    tracing::warn!(
                        count = rel_sink_rows.len(),
                        error = %e,
                        "Relational relationship sink batch failed"
                    );
                    if edgequake_storage::vector_backend_reads_typed(
                        edgequake_storage::vector_backend_from_env(),
                    ) {
                        return Err(e);
                    }
                }
            }
        }

        if let Some(ctx) = progress {
            ctx.emit_relationship_graph(
                total_valid,
                stats.relationships_created,
                stats.relationships_updated,
            );
        }

        Ok(sink_report)
    }

    /// Embed + upsert entities_vdb for newly created AGE placeholders (050 B7).
    ///
    /// LightRAG writes `{name}\n{relation_description}` into entities_vdb when
    /// creating UNKNOWN relation endpoints. Without this, Local/Mix cannot
    /// retrieve AGE-only stubs (age_over_vectors ≫ 1).
    async fn upsert_placeholder_entity_vectors(&self, specs: &[PlaceholderVdbSpec]) -> Result<()> {
        let Some(embedder) = self.text_embedder.as_ref() else {
            tracing::debug!(
                count = specs.len(),
                "Placeholder VDB skipped: no text_embedder wired (AGE-only stubs)"
            );
            return Ok(());
        };
        if specs.is_empty() {
            return Ok(());
        }

        let texts: Vec<String> = specs
            .iter()
            .map(|s| {
                crate::pipeline::helpers::unique_embed::entity_embed_text(&s.label, &s.description)
            })
            .collect();
        let embeddings = embedder
            .embed_texts(&texts)
            .await
            .map_err(|e| crate::error::PipelineError::EmbeddingError(e.to_string()))?;
        if embeddings.len() != specs.len() {
            return Err(crate::error::PipelineError::EmbeddingError(format!(
                "placeholder embed count mismatch: texts={} embeddings={}",
                specs.len(),
                embeddings.len()
            )));
        }

        let mut batch = Vec::with_capacity(specs.len());
        for (spec, embedding) in specs.iter().zip(embeddings) {
            let entity_id = EntityId::new(&spec.label);
            if entity_id.is_empty() {
                continue;
            }
            let mut entity =
                crate::extractor::ExtractedEntity::new(&spec.label, "UNKNOWN", &spec.description);
            entity.source_chunk_ids = spec.chunk_ids.clone();
            let scope = metadata::TenantScope {
                tenant_id: &self.tenant_id,
                workspace_id: &self.workspace_id,
            };
            let metadata = metadata::entity_vector_metadata(&entity, &entity_id, scope);
            batch.push((entity_id.as_vector_id(), embedding, metadata));
        }
        if batch.is_empty() {
            return Ok(());
        }
        self.vector_storage.upsert(&batch).await?;
        tracing::info!(
            count = batch.len(),
            "Placeholder entity VDB upserted (050 B7 / LightRAG parity)"
        );
        Ok(())
    }

    fn placeholder_node_properties(
        &self,
        label: &str,
        source_chunk_ids: &[String],
        description: &str,
    ) -> HashMap<String, serde_json::Value> {
        let mut properties = HashMap::new();
        properties.insert(
            "entity_type".to_string(),
            serde_json::Value::String("UNKNOWN".to_string()),
        );
        properties.insert(
            "description".to_string(),
            serde_json::Value::String(description.to_string()),
        );
        properties.insert(
            "label".to_string(),
            serde_json::Value::String(label.to_string()),
        );
        // Inherit relation chunk lineage so Mix/graph can still surface evidence
        // for endpoints that never received a full entity extract.
        // D-33: document lineage from full chunk set, then cap stored chunk ids.
        super::lineage::merge_and_insert_document_lineage(&mut properties, None, source_chunk_ids);
        let capped = apply_source_ids_limit(
            source_chunk_ids,
            self.config.max_source_ids_per_entity,
            self.config.source_ids_limit_method,
        );
        super::lineage::insert_chunk_lineage_properties(&mut properties, &capped);
        if let Some(tenant_id) = &self.tenant_id {
            properties.insert(
                "tenant_id".to_string(),
                serde_json::Value::String(tenant_id.clone()),
            );
        }
        if let Some(workspace_id) = &self.workspace_id {
            properties.insert(
                "workspace_id".to_string(),
                serde_json::Value::String(workspace_id.clone()),
            );
        }
        properties
    }

    async fn build_relationship_merge_outcome(
        &self,
        rel: &ExtractedRelationship,
        source_key: &str,
        target_key: &str,
        existing: Option<GraphEdge>,
    ) -> Result<RelMergeOutcome> {
        if let Some(mut edge) = existing {
            let mutated = self.update_relationship_edge(&mut edge, rel).await?;
            if !mutated {
                return Ok(RelMergeOutcome {
                    source: edge.source.clone(),
                    target: edge.target.clone(),
                    properties: edge.properties,
                    is_new: false,
                    skipped_saturated: true,
                    vector_id: None,
                    edge_for_map: None,
                });
            }
            return Ok(RelMergeOutcome {
                source: edge.source.clone(),
                target: edge.target.clone(),
                properties: edge.properties.clone(),
                is_new: false,
                skipped_saturated: false,
                vector_id: None,
                edge_for_map: Some(edge),
            });
        }

        let edge = self.create_relationship_edge(source_key, target_key, rel)?;
        let vector_id = rel
            .embedding
            .as_ref()
            .map(|_| format!("{}->{}:{}", source_key, target_key, rel.relation_type));
        Ok(RelMergeOutcome {
            source: edge.source.clone(),
            target: edge.target.clone(),
            properties: edge.properties.clone(),
            is_new: true,
            skipped_saturated: false,
            vector_id,
            edge_for_map: Some(edge),
        })
    }

    /// Update an existing relationship edge (SPEC-047 P7a fragment gate + P7d KEEP).
    ///
    /// Returns `false` when KEEP-saturated → caller skips upsert.
    async fn update_relationship_edge(
        &self,
        edge: &mut GraphEdge,
        rel: &ExtractedRelationship,
    ) -> Result<bool> {
        let incoming_ids = rel.all_source_chunk_ids();
        let existing_chunk_ids = source_chunk_ids_from_properties(&edge.properties);
        if should_skip_description_update_keep(
            &existing_chunk_ids,
            &incoming_ids,
            self.config.max_source_ids_per_relation,
            self.config.source_ids_limit_method,
        ) {
            tracing::debug!(
                source = %rel.source,
                target = %rel.target,
                existing = existing_chunk_ids.len(),
                max = self.config.max_source_ids_per_relation,
                "P7d KEEP: skip relationship description update (saturated)"
            );
            return Ok(false);
        }

        let existing_desc = edge
            .properties
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let merged_desc = self
            .resolve_relationship_description(
                &rel.source,
                &rel.target,
                existing_desc,
                &rel.description,
            )
            .await?;

        edge.properties.insert(
            "description".to_string(),
            serde_json::Value::String(merged_desc),
        );

        // D-31: WeightPolicy SSOT (default Max — associative).
        super::WeightPolicy::from_env().apply(&mut edge.properties, rel.weight);

        // Prefer newer relation_type when present (last-write-wins for type label).
        if !rel.relation_type.is_empty() {
            edge.properties.insert(
                "relation_type".to_string(),
                serde_json::Value::String(rel.relation_type.clone()),
            );
        }

        // Merge keywords
        let mut keywords: Vec<String> = edge
            .properties
            .get("keywords")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        for keyword in &rel.keywords {
            if !keywords.contains(keyword) {
                keywords.push(keyword.clone());
            }
        }

        // BR0004: Relationship keywords max 5 per edge
        // WHY: Excessive keywords dilute semantic relevance and inflate storage.
        // Keep the first 5 (oldest = most established context).
        keywords.truncate(5);

        edge.properties
            .insert("keywords".to_string(), serde_json::json!(keywords));

        // Merge + cap source chunk IDs (P7d)
        // D-33: document lineage from full merged set BEFORE capping stored chunks.
        let merged_ids = merge_source_ids(&existing_chunk_ids, &incoming_ids);
        super::lineage::merge_and_insert_document_lineage(
            &mut edge.properties,
            rel.source_document_id.as_deref(),
            &merged_ids,
        );
        let capped = apply_source_ids_limit(
            &merged_ids,
            self.config.max_source_ids_per_relation,
            self.config.source_ids_limit_method,
        );
        super::lineage::insert_chunk_lineage_properties(&mut edge.properties, &capped);
        if let Some(first) = capped.first() {
            edge.properties.insert(
                "source_chunk_id".to_string(),
                serde_json::Value::String(first.clone()),
            );
        }

        Ok(true)
    }

    /// Create a new relationship edge.
    fn create_relationship_edge(
        &self,
        source_key: &str,
        target_key: &str,
        rel: &ExtractedRelationship,
    ) -> Result<GraphEdge> {
        let mut properties = HashMap::new();
        properties.insert(
            "relation_type".to_string(),
            serde_json::Value::String(rel.relation_type.clone()),
        );
        properties.insert(
            "description".to_string(),
            serde_json::Value::String(rel.description.clone()),
        );
        properties.insert(
            "weight".to_string(),
            serde_json::Value::Number(serde_json::Number::from_f64(rel.weight as f64).unwrap()),
        );
        properties.insert("keywords".to_string(), serde_json::json!(rel.keywords));

        // Source tracking for citations (LightRAG parity / 049 multi-chunk union)
        // D-33: document lineage from full incoming set BEFORE capping stored chunks.
        let incoming = rel.all_source_chunk_ids();
        // SPEC-091 RM2: fail-closed citation contract (LAW-RM7).
        super::lineage::require_citation(&incoming)
            .map_err(crate::error::PipelineError::GraphError)?;
        super::lineage::merge_and_insert_document_lineage(
            &mut properties,
            rel.source_document_id.as_deref(),
            &incoming,
        );
        let capped = apply_source_ids_limit(
            &incoming,
            self.config.max_source_ids_per_relation,
            self.config.source_ids_limit_method,
        );
        super::lineage::insert_chunk_lineage_properties(&mut properties, &capped);
        if let Some(first) = capped.first() {
            properties.insert(
                "source_chunk_id".to_string(),
                serde_json::Value::String(first.clone()),
            );
        }
        if let Some(ref file_path) = rel.source_file_path {
            properties.insert(
                "source_file_path".to_string(),
                serde_json::Value::String(file_path.clone()),
            );
        }

        // Add tenant context
        if let Some(tenant_id) = &self.tenant_id {
            properties.insert(
                "tenant_id".to_string(),
                serde_json::Value::String(tenant_id.clone()),
            );
        }
        if let Some(workspace_id) = &self.workspace_id {
            properties.insert(
                "workspace_id".to_string(),
                serde_json::Value::String(workspace_id.clone()),
            );
        }

        Ok(GraphEdge {
            source: source_key.to_string(),
            target: target_key.to_string(),
            properties,
        })
    }
}

/// Collapse extracted relationships to one row per `(source, target, rel_type)`.
///
/// # Policy (SPEC-083 D-30 / D-31)
///
/// - Distinct relation types between the same endpoints **both** persist
/// - Same type: last-write description/embeddings; keyword union; WeightPolicy
/// - **049:** `source_chunk_ids` **union**
fn dedupe_relationships_by_endpoints(
    rows: Vec<(ExtractedRelationship, String, String)>,
) -> Vec<(ExtractedRelationship, String, String)> {
    let weight_policy = super::WeightPolicy::from_env();
    let mut order: Vec<(String, String, String)> = Vec::new();
    let mut map: HashMap<(String, String, String), ExtractedRelationship> = HashMap::new();

    for (rel, source_key, target_key) in rows {
        let rel_type = if rel.relation_type.trim().is_empty() {
            "RELATED_TO".to_string()
        } else {
            rel.relation_type.trim().to_ascii_uppercase()
        };
        let key = (source_key.clone(), target_key.clone(), rel_type.clone());
        if let Some(existing) = map.get_mut(&key) {
            if rel.description.len() > existing.description.len() {
                existing.description = rel.description.clone();
            }
            existing.weight = weight_policy.combine(existing.weight, rel.weight);
            for kw in &rel.keywords {
                if !existing.keywords.contains(kw) {
                    existing.keywords.push(kw.clone());
                }
            }
            if rel.embedding.is_some() {
                existing.embedding = rel.embedding.clone();
            }
            for cid in rel.all_source_chunk_ids() {
                existing.add_source_chunk_id(cid);
            }
            if rel.source_document_id.is_some() {
                existing.source_document_id = rel.source_document_id.clone();
            }
            if rel.source_file_path.is_some() {
                existing.source_file_path = rel.source_file_path.clone();
            }
        } else {
            let mut normalized = rel;
            normalized.relation_type = rel_type;
            for cid in normalized.all_source_chunk_ids() {
                normalized.add_source_chunk_id(cid);
            }
            order.push(key.clone());
            map.insert(key, normalized);
        }
    }

    order
        .into_iter()
        .filter_map(|(src, tgt, rel)| {
            map.remove(&(src.clone(), tgt.clone(), rel))
                .map(|rel| (rel, src, tgt))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractor::ExtractionResult;
    use crate::merger::{KnowledgeGraphMerger, MergerConfig, SourceIdsLimitMethod};
    use async_trait::async_trait;
    use edgequake_storage::{
        EntityId, MemoryGraphStorage, MemoryVectorStorage, TextEmbedder, VectorStorage,
    };
    use std::sync::Arc;

    fn test_merger() -> KnowledgeGraphMerger<MemoryGraphStorage, MemoryVectorStorage> {
        let graph = Arc::new(MemoryGraphStorage::new("b5-placeholder"));
        let vector = Arc::new(MemoryVectorStorage::new("b5-placeholder", 4));
        KnowledgeGraphMerger::new(MergerConfig::default(), graph, vector)
            .with_tenant_context(Some("tenant-b5".to_string()), Some("ws-b5".to_string()))
    }

    struct FixedEmbedder;

    #[async_trait]
    impl TextEmbedder for FixedEmbedder {
        async fn embed_texts(
            &self,
            texts: &[String],
        ) -> edgequake_storage::error::Result<Vec<Vec<f32>>> {
            Ok(texts.iter().map(|_| vec![0.1, 0.2, 0.3, 0.4]).collect())
        }
    }

    #[test]
    fn placeholder_inherits_relation_source_chunk_ids() {
        let merger = test_merger();
        let props = merger.placeholder_node_properties(
            "AJCC",
            &["doc-1-chunk-3".to_string(), "doc-1-chunk-7".to_string()],
            "AJCC staging system",
        );
        let ids = source_chunk_ids_from_properties(&props);
        assert_eq!(
            ids,
            vec!["doc-1-chunk-3".to_string(), "doc-1-chunk-7".to_string()]
        );
        assert_eq!(
            props.get("entity_type").and_then(|v| v.as_str()),
            Some("UNKNOWN")
        );
        assert_eq!(props.get("label").and_then(|v| v.as_str()), Some("AJCC"));
        assert_eq!(
            props.get("description").and_then(|v| v.as_str()),
            Some("AJCC staging system")
        );
        let source_ids = props
            .get("source_ids")
            .and_then(|v| v.as_array())
            .expect("source_ids");
        assert_eq!(source_ids.len(), 2);
        assert_eq!(
            props.get("workspace_id").and_then(|v| v.as_str()),
            Some("ws-b5")
        );
    }

    #[test]
    fn placeholder_empty_chunks_still_writes_empty_lineage_arrays() {
        let merger = test_merger();
        let props = merger.placeholder_node_properties("ORPHAN", &[], "");
        let ids = source_chunk_ids_from_properties(&props);
        assert!(ids.is_empty());
        assert!(props.contains_key("source_chunk_ids"));
        assert!(props.contains_key("source_ids"));
    }

    /// SPEC-083 D-33: document lineage keeps docs beyond the stored chunk-id cap.
    #[test]
    fn e2e_lineage_includes_docs_beyond_source_cap() {
        let cfg = MergerConfig {
            max_source_ids_per_entity: 2,
            source_ids_limit_method: SourceIdsLimitMethod::Fifo,
            ..Default::default()
        };
        let graph = Arc::new(MemoryGraphStorage::new("d33-lineage"));
        let vector = Arc::new(MemoryVectorStorage::new("d33-lineage", 4));
        let merger = KnowledgeGraphMerger::new(cfg, graph, vector)
            .with_tenant_context(Some("t".into()), Some("w".into()));

        // Three docs → three derived document ids; chunk storage capped at 2.
        let chunks = vec![
            "doc-a-chunk-1".to_string(),
            "doc-b-chunk-1".to_string(),
            "doc-c-chunk-1".to_string(),
        ];
        let props = merger.placeholder_node_properties("ENTITY", &chunks, "desc");
        let stored = source_chunk_ids_from_properties(&props);
        assert_eq!(stored.len(), 2, "chunk ids must be capped");
        let docs = crate::merger::lineage::source_document_ids_from_properties(&props);
        assert!(
            docs.len() >= 3,
            "document lineage must retain all docs beyond chunk cap, got {docs:?}"
        );
        assert!(docs.iter().any(|d| d == "doc-a"));
        assert!(docs.iter().any(|d| d == "doc-b"));
        assert!(docs.iter().any(|d| d == "doc-c"));
    }

    #[test]
    fn retain_longest_description_keeps_longer_candidate() {
        let mut map = HashMap::new();
        retain_longest_description(&mut map, "AJCC", "short");
        retain_longest_description(&mut map, "AJCC", "a much longer description");
        retain_longest_description(&mut map, "AJCC", "mid");
        assert_eq!(
            map.get("AJCC").map(String::as_str),
            Some("a much longer description")
        );
    }

    #[tokio::test]
    async fn placeholder_endpoints_get_entity_vdb_rows() {
        let graph = Arc::new(MemoryGraphStorage::new("b7-placeholder-vdb"));
        let vector = Arc::new(MemoryVectorStorage::new("b7-placeholder-vdb", 4));
        vector.initialize().await.unwrap();
        let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph, vector.clone())
            .with_tenant_context(Some("tenant-b7".into()), Some("ws-b7".into()))
            .with_text_embedder(Arc::new(FixedEmbedder));

        // Only a relationship — endpoints are AGE placeholders, not extracted entities.
        let rel = ExtractedRelationship::new("Alice", "Bob", "KNOWS")
            .with_description("Alice knows Bob from the clinic")
            .with_source_chunk_id("chunk-7");
        let extraction = ExtractionResult {
            entities: vec![],
            relationships: vec![rel],
            source_chunk_id: "chunk-7".into(),
            ..Default::default()
        };

        merger
            .merge(vec![extraction])
            .await
            .expect("merge placeholders");

        for name in ["Alice", "Bob"] {
            let vid = EntityId::new(name).as_vector_id();
            let got = vector
                .get_by_id(&vid)
                .await
                .expect("get vector")
                .unwrap_or_else(|| panic!("missing entity VDB for placeholder {name} ({vid})"));
            assert_eq!(got.len(), 4, "{name} embedding dim");
        }
    }

    #[test]
    fn dedupe_relationships_keeps_multigraph_types() {
        // D-30: distinct types between same endpoints both survive.
        let rows = vec![
            (
                ExtractedRelationship::new("Alice", "Bob", "KNOWS").with_description("a"),
                "ALICE".into(),
                "BOB".into(),
            ),
            (
                ExtractedRelationship::new("Alice", "Bob", "WORKS_WITH")
                    .with_description("longer description")
                    .with_weight(0.9)
                    .with_keywords(vec!["x".into()]),
                "ALICE".into(),
                "BOB".into(),
            ),
            (
                ExtractedRelationship::new("Carol", "Dave", "RELATED"),
                "CAROL".into(),
                "DAVE".into(),
            ),
        ];
        let out = dedupe_relationships_by_endpoints(rows);
        assert_eq!(out.len(), 3);
        let types: Vec<_> = out
            .iter()
            .map(|(r, _, _)| r.relation_type.as_str())
            .collect();
        assert!(types.contains(&"KNOWS"));
        assert!(types.contains(&"WORKS_WITH"));
        assert!(types.contains(&"RELATED"));
    }

    #[test]
    fn dedupe_relationships_unions_source_chunk_ids_same_type() {
        let rows = vec![
            (
                ExtractedRelationship::new("Alice", "Bob", "KNOWS")
                    .with_description("a")
                    .with_source_chunk_id("chunk-0"),
                "ALICE".into(),
                "BOB".into(),
            ),
            (
                ExtractedRelationship::new("Alice", "Bob", "KNOWS")
                    .with_description("longer description")
                    .with_source_chunk_id("chunk-1"),
                "ALICE".into(),
                "BOB".into(),
            ),
        ];
        let out = dedupe_relationships_by_endpoints(rows);
        assert_eq!(out.len(), 1);
        let ids = out[0].0.all_source_chunk_ids();
        assert!(ids.contains(&"chunk-0".to_string()), "{ids:?}");
        assert!(ids.contains(&"chunk-1".to_string()), "{ids:?}");
        assert_eq!(ids.len(), 2, "{ids:?}");
    }
}
