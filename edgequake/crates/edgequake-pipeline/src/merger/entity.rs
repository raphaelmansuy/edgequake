//! Entity merge, update, and creation logic for the knowledge graph.
//!
//! # SPEC-032 changes
//! - W-03: `merge_entities_batch` deduplicates within-document before graph read
//! - W-06: Similarity gate skips LLM summarizer when descriptions are near-identical
//!
//! # SPEC-047 P7b / P7d
//! - Parallel unique-entity description merges (`merge_max_async`)
//! - SOURCE_IDS KEEP: skip saturated description updates

use std::collections::HashMap;

use edgequake_storage::{EntityId, GraphNode, GraphStorage, VectorStorage};
use futures::stream::{self, StreamExt};

use crate::error::Result;
use crate::extractor::{ExtractedEntity, ExtractionResult};

use super::entity_type_vote::{
    apply_entity_type_vote, merge_type_into_entity, ENTITY_TYPE_VOTES_KEY,
};
use super::merge_limits::{
    apply_source_ids_limit, merge_source_ids, should_skip_description_update_keep,
    source_chunk_ids_from_properties,
};
use super::{metadata, MergeStats};

/// Outcome of one unique-entity merge (P7b parallel collect).
struct EntityMergeOutcome {
    node_id: String,
    properties: HashMap<String, serde_json::Value>,
    is_new: bool,
    skipped_saturated: bool,
    vector_id: Option<String>,
    graph_key_created: Option<String>,
}

/// Jaccard word-overlap similarity between two strings (pub for tests and external use).
pub fn description_similarity(a: &str, b: &str) -> f32 {
    if a == b {
        // Identical strings: if both empty → 0 (no overlap to measure); if same non-empty → 1.0
        return if a.is_empty() { 0.0 } else { 1.0 };
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();
    let intersection = a_words.intersection(&b_words).count();
    let union = a_words.union(&b_words).count();
    if union == 0 {
        1.0
    } else {
        intersection as f32 / union as f32
    }
}

/// Threshold above which we skip LLM summarization (descriptions are near-identical).
/// Tunable via `MergerConfig.description_similarity_threshold`.
/// Default exposed here for tests; runtime value comes from `MergerConfig`.
#[cfg(test)]
#[allow(dead_code)]
pub(crate) const DEFAULT_SIMILARITY_THRESHOLD: f32 = 0.85;

impl<G: GraphStorage + ?Sized, V: VectorStorage + ?Sized> super::KnowledgeGraphMerger<G, V> {
    /// Collect batched entity vector upserts for all extractions (P-G4-merger).
    ///
    /// SPEC-047 P6: dedupe by `EntityId` before upsert so HNSW pays O(unique),
    /// not O(mentions). Embeddings are already unique-before-embed; this is the
    /// storage-side half of the same first principle.
    pub(super) fn collect_entity_vector_batch(
        &self,
        results: &[ExtractionResult],
    ) -> Vec<(String, Vec<f32>, serde_json::Value)> {
        let all: Vec<_> = results
            .iter()
            .flat_map(|r| r.entities.iter().cloned())
            .collect();
        let unique = crate::pipeline::helpers::unique_embed::dedupe_entities_by_id(&all);
        let mut batch = Vec::with_capacity(unique.len());
        for entity in &unique {
            let entity_id = EntityId::new(&entity.name);
            if entity_id.is_empty() {
                continue;
            }
            let Some(embedding) = entity.embedding.as_ref() else {
                continue;
            };
            let scope = metadata::TenantScope {
                tenant_id: &self.tenant_id,
                workspace_id: &self.workspace_id,
            };
            let metadata = metadata::entity_vector_metadata(entity, &entity_id, scope);
            batch.push((entity_id.as_vector_id(), embedding.clone(), metadata));
        }
        batch
    }

    /// Merge entities with one `get_nodes_batch` + one `upsert_nodes_batch` (P-G4-graph).
    ///
    /// # SPEC-032 W-03: Within-document deduplication
    ///
    /// When the same entity appears in multiple chunks of the same document,
    /// the previous per-chunk loop would issue N get+upsert pairs.
    /// Now we deduplicate first: same-named entities within one call are merged
    /// in-memory (concatenating source_chunk_ids, taking the longer description)
    /// before a single get_nodes_batch reads the existing graph state.
    ///
    /// Edge case (D-32): same name but different types → majority/confidence
    /// vote (never silent first-wins); conflicts are logged.
    ///
    /// X-17: when `EDGEQUAKE_ENTITY_FUZZY=1`, near-duplicate names may collapse
    /// onto an existing EntityId via blocking + Levenshtein/Jaccard.
    pub(super) async fn merge_entities_batch(
        &self,
        entities: Vec<ExtractedEntity>,
        stats: &mut MergeStats,
    ) -> Result<()> {
        if entities.is_empty() {
            return Ok(());
        }

        // ── Within-batch deduplication (SPEC-032 W-03) ───────────────────
        // If the same entity name appears in multiple chunks of this document,
        // merge them in-memory before hitting the database.
        // Use Vec<(key, entity)> to preserve first-seen order for determinism.
        let mut dedup_keys: Vec<String> = Vec::new();
        let mut dedup_map: HashMap<String, ExtractedEntity> = HashMap::new();
        let mut type_votes_by_key: HashMap<String, HashMap<String, f64>> = HashMap::new();

        for entity in entities {
            let entity_id = EntityId::new(&entity.name);
            if entity_id.is_empty() {
                if edgequake_storage::is_opaque_identifier(&entity.name) {
                    tracing::warn!(
                        raw_name = %entity.name,
                        metric = "opaque_entity_name_rejected",
                        "Skipping opaque identifier entity name (067)"
                    );
                } else {
                    tracing::warn!(
                        raw_name = %entity.name,
                        "Skipping entity with empty normalized name"
                    );
                }
                continue;
            }
            // SPEC-032 / B3b: workspace-scoped AGE node_id so Acc WS cannot
            // collide with foreign tenants on bare EntityId.
            let mut key = entity_id.graph_node_id_for_workspace(self.workspace_id.as_deref());
            // SPEC-162 R13: within-batch fold collapse (hyphen ↔ underscore).
            let fold = edgequake_storage::fold_slug_key(entity_id.as_str());
            if !fold.is_empty() {
                if let Some(existing_key) = dedup_keys.iter().find(|k| {
                    edgequake_storage::fold_slug_key(EntityId::bare_name_from_graph_node_id(k))
                        == fold
                }) {
                    key = existing_key.clone();
                }
            }
            if let Some(existing) = dedup_map.get_mut(&key) {
                // Merge descriptions: keep longer (richer)
                if entity.description.len() > existing.description.len() {
                    existing.description = entity.description.clone();
                }
                // Accumulate source chunks
                for cid in &entity.source_chunk_ids {
                    if !existing.source_chunk_ids.contains(cid) {
                        existing.source_chunk_ids.push(cid.clone());
                    }
                }
                // Merge source spans
                for span in &entity.source_spans {
                    if !existing.source_spans.contains(span) {
                        existing.source_spans.push(span.clone());
                    }
                }
                // D-32: majority/confidence type vote (not silent first-wins)
                let votes = type_votes_by_key.entry(key.clone()).or_default();
                merge_type_into_entity(
                    &mut existing.entity_type,
                    votes,
                    &entity.entity_type,
                    entity.importance,
                    &entity.name,
                );
                // Take max importance
                if entity.importance > existing.importance {
                    existing.importance = entity.importance;
                }
            } else {
                let mut votes = HashMap::new();
                let mut seeded_type = entity.entity_type.clone();
                merge_type_into_entity(
                    &mut seeded_type,
                    &mut votes,
                    &entity.entity_type,
                    entity.importance,
                    &entity.name,
                );
                let mut entity = entity;
                entity.entity_type = seeded_type;
                type_votes_by_key.insert(key.clone(), votes);
                dedup_keys.push(key.clone());
                dedup_map.insert(key, entity);
            }
        }

        // Collect in insertion order (deterministic)
        let (mut keys, mut valid): (Vec<String>, Vec<ExtractedEntity>) = dedup_keys
            .into_iter()
            .filter_map(|k| dedup_map.remove(&k).map(|e| (k, e)))
            .unzip();

        if valid.is_empty() {
            return Ok(());
        }

        // X-17: optional within-batch fuzzy collapse (default off).
        if edgequake_storage::entity_fuzzy_enabled() {
            self.apply_within_batch_fuzzy(&mut keys, &mut valid);
        }

        // Store entity types for relational sink (borrow before move into loop)
        let entity_types: Vec<String> = valid.iter().map(|e| e.entity_type.clone()).collect();
        let descriptions: Vec<String> = valid.iter().map(|e| e.description.clone()).collect();
        let source_chunk_ids: Vec<Vec<String>> =
            valid.iter().map(|e| e.source_chunk_ids.clone()).collect();

        let mut existing_map = self.graph_storage.get_nodes_batch(&keys).await?;

        // SPEC-162 R13: fold-equivalent resolve before create (hyphen ↔ underscore).
        super::key_resolver::resolve_fold_keys(
            self.graph_storage.as_ref(),
            self.workspace_id.as_deref(),
            &mut keys,
            &mut existing_map,
        )
        .await?;
        self.collapse_duplicate_keys(&mut keys, &mut valid);

        // X-17: fuzzy resolve against graph for exact misses (bounded sample).
        if edgequake_storage::entity_fuzzy_enabled() {
            self.apply_graph_fuzzy_resolution(&mut keys, &mut valid, &mut existing_map)
                .await?;
            // Collapse any duplicate keys created by fuzzy remapping.
            self.collapse_duplicate_keys(&mut keys, &mut valid);
        }

        let concurrency = self.config.merge_max_async.max(1);

        // SPEC-047 P7b: resolve unique entity merges concurrently (LLM-bound).
        let indexed: Vec<(usize, ExtractedEntity, String)> = valid
            .into_iter()
            .zip(keys.iter().cloned())
            .enumerate()
            .map(|(i, (entity, key))| (i, entity, key))
            .collect();

        let outcomes: Vec<Result<(usize, EntityMergeOutcome)>> = stream::iter(indexed)
            .map(|(i, entity, key)| {
                let existing = existing_map.get(&key).cloned();
                async move {
                    let outcome = self
                        .build_entity_merge_outcome(&entity, existing.as_ref())
                        .await?;
                    Ok((i, outcome))
                }
            })
            .buffer_unordered(concurrency)
            .collect()
            .await;

        let mut ordered: Vec<(usize, EntityMergeOutcome)> = Vec::with_capacity(outcomes.len());
        for item in outcomes {
            match item {
                Ok(pair) => ordered.push(pair),
                Err(e) => {
                    stats.record_error(e.to_string());
                    tracing::warn!(
                        error.source = "pipeline_merger",
                        error.action = "merge_entity",
                        error.message = %e,
                        "Failed to merge entity"
                    );
                }
            }
        }
        ordered.sort_by_key(|(i, _)| *i);

        let mut node_batch: Vec<(String, HashMap<String, serde_json::Value>)> =
            Vec::with_capacity(ordered.len());
        let mut sink_rows: Vec<crate::merger::EntitySinkRow> = Vec::with_capacity(ordered.len());
        let mut lineage_links: Vec<crate::merger::EntityLineageLink> = Vec::new();
        let ws = self.workspace_id.as_deref().unwrap_or("default");

        for (i, outcome) in ordered {
            let key = &keys[i];
            // SPEC-098 LAW-098-2: KEEP is AGE-only. Saturated entities skip
            // graph mutation but must still ensure the relational spine so
            // typed fleet mirror can resolve entity:NAME → entities.id.
            if outcome.skipped_saturated {
                stats.entities_skipped_saturated += 1;
                stats.entities_spine_ensured_saturated += 1;
                sink_rows.push(crate::merger::EntitySinkRow {
                    name: key.clone(),
                    entity_type: entity_types[i].clone(),
                    description: descriptions[i].clone(),
                    tenant_id: self.tenant_id.clone(),
                    workspace_id: self.workspace_id.clone(),
                    source_chunk_ids: source_chunk_ids[i].clone(),
                });
                continue;
            }
            // Entity vector IDs are recorded in upsert_vectors_chunked (SPEC-057 P3).
            let _ = outcome.vector_id;
            if let Some(graph_key) = outcome.graph_key_created {
                stats.artifacts.graph_nodes_created.push(graph_key);
            }
            node_batch.push((outcome.node_id, outcome.properties));
            if outcome.is_new {
                stats.entities_created += 1;
            } else {
                stats.entities_updated += 1;
            }

            // SPEC-091 IP1: collect CQRS rows for one batch upsert (LAW-IP2).
            sink_rows.push(crate::merger::EntitySinkRow {
                name: key.clone(),
                entity_type: entity_types[i].clone(),
                description: descriptions[i].clone(),
                tenant_id: self.tenant_id.clone(),
                workspace_id: self.workspace_id.clone(),
                source_chunk_ids: source_chunk_ids[i].clone(),
            });
            for chunk_id in &source_chunk_ids[i] {
                lineage_links.push(crate::merger::EntityLineageLink {
                    chunk_id: chunk_id.clone(),
                    entity_name: key.to_string(),
                    workspace_id: ws.to_string(),
                });
            }
        }

        if !node_batch.is_empty() {
            self.graph_storage.upsert_nodes_batch(&node_batch).await?;
        }

        if !sink_rows.is_empty() {
            if let Err(e) = self.relational_sink.upsert_entities_batch(&sink_rows).await {
                tracing::warn!(
                    count = sink_rows.len(),
                    error = %e,
                    "Relational entity sink batch failed"
                );
                if self.typed_reads() {
                    return Err(e);
                }
            }
        }

        if !lineage_links.is_empty() {
            self.lineage_sink
                .record_entity_links_batch(&lineage_links)
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!(
                        count = lineage_links.len(),
                        error = %e,
                        "Lineage sink record_entity_links_batch failed (best-effort)"
                    );
                });
        }

        Ok(())
    }

    async fn build_entity_merge_outcome(
        &self,
        entity: &ExtractedEntity,
        existing: Option<&GraphNode>,
    ) -> Result<EntityMergeOutcome> {
        let entity_id = EntityId::new(&entity.name);
        let entity_key = entity_id.graph_node_id_for_workspace(self.workspace_id.as_deref());

        // Foreign-workspace hit on a legacy bare node_id: never merge across WS.
        let existing = existing.filter(|n| {
            let node_ws = n.properties.get("workspace_id").and_then(|v| v.as_str());
            match (self.workspace_id.as_deref(), node_ws) {
                (Some(w), Some(nw)) => w == nw,
                (Some(_), None) => false,
                (None, _) => true,
            }
        });

        match existing.cloned() {
            Some(mut node) => {
                let mutated = self.update_entity_node(&mut node, entity).await?;
                if !mutated {
                    return Ok(EntityMergeOutcome {
                        node_id: node.id.clone(),
                        properties: node.properties,
                        is_new: false,
                        skipped_saturated: true,
                        vector_id: None,
                        graph_key_created: None,
                    });
                }
                Ok(EntityMergeOutcome {
                    node_id: node.id.clone(),
                    properties: node.properties,
                    is_new: false,
                    skipped_saturated: false,
                    vector_id: None,
                    graph_key_created: None,
                })
            }
            None => {
                let node = self.create_entity_node(entity)?;
                let vector_id = entity.embedding.as_ref().map(|_| entity_id.as_vector_id());
                Ok(EntityMergeOutcome {
                    node_id: node.id,
                    properties: node.properties,
                    is_new: true,
                    skipped_saturated: false,
                    vector_id,
                    graph_key_created: Some(entity_key),
                })
            }
        }
    }

    /// Update an existing entity node with new information.
    ///
    /// Returns `false` when SPEC-047 P7d KEEP saturated → caller skips upsert.
    ///
    /// # SPEC-047 P7a + SPEC-032 W-06
    ///
    /// Description merge is delegated to [`super::decide_description_merge`]
    /// (LightRAG fragment gate + Jaccard soft-resume skip). LLM runs only when
    /// the policy returns [`super::DescriptionMergeDecision::NeedsLlm`].
    async fn update_entity_node(
        &self,
        node: &mut GraphNode,
        entity: &ExtractedEntity,
    ) -> Result<bool> {
        let existing_chunk_ids = source_chunk_ids_from_properties(&node.properties);
        if should_skip_description_update_keep(
            &existing_chunk_ids,
            &entity.source_chunk_ids,
            self.config.max_source_ids_per_entity,
            self.config.source_ids_limit_method,
        ) {
            tracing::debug!(
                entity = %entity.name,
                existing = existing_chunk_ids.len(),
                max = self.config.max_source_ids_per_entity,
                "P7d KEEP: skip entity description update (saturated)"
            );
            return Ok(false);
        }

        let existing_desc = node
            .properties
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let merged_desc = self
            .resolve_entity_description(&entity.name, existing_desc, &entity.description)
            .await?;

        node.properties.insert(
            "description".to_string(),
            serde_json::Value::String(merged_desc),
        );

        // D-32: majority/confidence type vote; always log concrete conflicts.
        apply_entity_type_vote(
            &mut node.properties,
            &entity.name,
            &entity.entity_type,
            entity.importance,
        );

        // Update importance (take max)
        let existing_importance = node
            .properties
            .get("importance")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32;

        let new_importance = existing_importance.max(entity.importance);
        node.properties.insert(
            "importance".to_string(),
            serde_json::Value::Number(serde_json::Number::from_f64(new_importance as f64).unwrap()),
        );

        // Merge source spans
        let mut sources: Vec<String> = node
            .properties
            .get("sources")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        for span in &entity.source_spans {
            if !sources.contains(span) && sources.len() < self.config.max_sources {
                sources.push(span.clone());
            }
        }

        node.properties
            .insert("sources".to_string(), serde_json::json!(sources));

        // Merge + cap source chunk IDs (P7d KEEP/FIFO).
        // D-33: compute document lineage from the full merged set BEFORE capping
        // stored chunk ids, so docs beyond the cap remain in source_document_ids.
        let merged_ids = merge_source_ids(&existing_chunk_ids, &entity.source_chunk_ids);
        super::lineage::merge_and_insert_document_lineage(
            &mut node.properties,
            entity.source_document_id.as_deref(),
            &merged_ids,
        );
        let source_chunk_ids = apply_source_ids_limit(
            &merged_ids,
            self.config.max_source_ids_per_entity,
            self.config.source_ids_limit_method,
        );
        super::lineage::insert_chunk_lineage_properties(&mut node.properties, &source_chunk_ids);

        // Update source file path if not already set
        if !node.properties.contains_key("source_file_path") {
            if let Some(ref file_path) = entity.source_file_path {
                node.properties.insert(
                    "source_file_path".to_string(),
                    serde_json::Value::String(file_path.clone()),
                );
            }
        }

        // 066: refresh multimodal display props when present on the incoming entity.
        if let Some(ref display_name) = entity.display_name {
            node.properties.insert(
                "display_name".to_string(),
                serde_json::Value::String(display_name.clone()),
            );
        }
        if let Some(page) = entity.page_num {
            node.properties.insert(
                "page_num".to_string(),
                serde_json::Value::Number(serde_json::Number::from(page)),
            );
        }
        if let Some(fig) = entity.figure_index {
            node.properties.insert(
                "figure_index".to_string(),
                serde_json::Value::Number(serde_json::Number::from(fig)),
            );
        }
        if let Some(ref asset_id) = entity.asset_id {
            node.properties.insert(
                "asset_id".to_string(),
                serde_json::Value::String(asset_id.clone()),
            );
        }
        if let Some(ref mm_subtype) = entity.mm_subtype {
            node.properties.insert(
                "mm_subtype".to_string(),
                serde_json::Value::String(mm_subtype.clone()),
            );
        }

        Ok(true)
    }

    /// Create a new entity node.
    fn create_entity_node(&self, entity: &ExtractedEntity) -> Result<GraphNode> {
        let entity_id = EntityId::new(&entity.name);
        let entity_key = entity_id.graph_node_id_for_workspace(self.workspace_id.as_deref());
        // Identity label stays bare normalized name (not the scoped id).
        // Human surface uses `display_name` when set (066 multimodal entities).
        let label = entity_id.as_str().to_string();

        let mut properties = HashMap::new();
        properties.insert(
            "entity_type".to_string(),
            serde_json::Value::String(entity.entity_type.clone()),
        );
        // Seed D-32 vote map so later conflicts have a baseline.
        properties.insert(
            ENTITY_TYPE_VOTES_KEY.to_string(),
            serde_json::json!({
                (entity.entity_type.as_str()): f64::from(entity.importance.clamp(0.05, 1.0))
            }),
        );
        properties.insert(
            "description".to_string(),
            serde_json::Value::String(entity.description.clone()),
        );
        properties.insert(
            "importance".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(entity.importance as f64).unwrap(),
            ),
        );
        properties.insert(
            "sources".to_string(),
            serde_json::json!(entity.source_spans),
        );
        properties.insert("label".to_string(), serde_json::Value::String(label));
        if let Some(ref display_name) = entity.display_name {
            properties.insert(
                "display_name".to_string(),
                serde_json::Value::String(display_name.clone()),
            );
        }
        if let Some(page) = entity.page_num {
            properties.insert(
                "page_num".to_string(),
                serde_json::Value::Number(serde_json::Number::from(page)),
            );
        }
        if let Some(fig) = entity.figure_index {
            properties.insert(
                "figure_index".to_string(),
                serde_json::Value::Number(serde_json::Number::from(fig)),
            );
        }
        if let Some(ref asset_id) = entity.asset_id {
            properties.insert(
                "asset_id".to_string(),
                serde_json::Value::String(asset_id.clone()),
            );
        }
        if let Some(ref mm_subtype) = entity.mm_subtype {
            properties.insert(
                "mm_subtype".to_string(),
                serde_json::Value::String(mm_subtype.clone()),
            );
        }

        // Source tracking for citations (LightRAG parity) + analytics reconcile.
        // D-33: document lineage from full chunk set, then cap stored chunk ids.
        super::lineage::merge_and_insert_document_lineage(
            &mut properties,
            entity.source_document_id.as_deref(),
            &entity.source_chunk_ids,
        );
        let capped = apply_source_ids_limit(
            &entity.source_chunk_ids,
            self.config.max_source_ids_per_entity,
            self.config.source_ids_limit_method,
        );
        super::lineage::insert_chunk_lineage_properties(&mut properties, &capped);
        if let Some(ref file_path) = entity.source_file_path {
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

        Ok(GraphNode {
            id: entity_key,
            properties,
        })
    }

    /// X-17: collapse near-duplicate keys inside the current batch onto the
    /// first-seen canonical key when fuzzy matching is enabled.
    fn apply_within_batch_fuzzy(&self, keys: &mut Vec<String>, valid: &mut Vec<ExtractedEntity>) {
        if keys.len() < 2 {
            return;
        }
        let threshold = edgequake_storage::fuzzy_match_threshold();
        let mut remap: HashMap<usize, usize> = HashMap::new();
        for i in 1..keys.len() {
            let bare_i = EntityId::bare_name_from_graph_node_id(&keys[i]).to_string();
            let prior: Vec<&str> = keys[..i]
                .iter()
                .enumerate()
                .filter(|(j, _)| !remap.contains_key(j))
                .map(|(_, k)| EntityId::bare_name_from_graph_node_id(k))
                .collect();
            if let Some(hit) =
                edgequake_storage::find_best_fuzzy_match(&bare_i, prior.iter().copied(), threshold)
            {
                if let Some(j) = keys[..i]
                    .iter()
                    .enumerate()
                    .find(|(idx, k)| {
                        !remap.contains_key(idx) && EntityId::bare_name_from_graph_node_id(k) == hit
                    })
                    .map(|(idx, _)| idx)
                {
                    tracing::info!(
                        from = %keys[i],
                        onto = %keys[j],
                        metric = "entity_fuzzy_within_batch",
                        "X-17: fuzzy merge within batch"
                    );
                    remap.insert(i, j);
                }
            }
        }
        if remap.is_empty() {
            return;
        }
        // Fold remapped entities into targets, then drop remapped slots.
        let mut keep = vec![true; keys.len()];
        for (from, onto) in &remap {
            keep[*from] = false;
            // Merge description / chunks / type votes into target.
            if valid[*from].description.len() > valid[*onto].description.len() {
                valid[*onto].description = valid[*from].description.clone();
            }
            for cid in valid[*from].source_chunk_ids.clone() {
                if !valid[*onto].source_chunk_ids.contains(&cid) {
                    valid[*onto].source_chunk_ids.push(cid);
                }
            }
            let incoming_type = valid[*from].entity_type.clone();
            let incoming_importance = valid[*from].importance;
            let incoming_name = valid[*from].name.clone();
            let mut votes = HashMap::new();
            merge_type_into_entity(
                &mut valid[*onto].entity_type,
                &mut votes,
                &incoming_type,
                incoming_importance,
                &incoming_name,
            );
            if incoming_importance > valid[*onto].importance {
                valid[*onto].importance = incoming_importance;
            }
        }
        let mut new_keys = Vec::new();
        let mut new_valid = Vec::new();
        for (i, keep_i) in keep.into_iter().enumerate() {
            if keep_i {
                new_keys.push(keys[i].clone());
                new_valid.push(valid[i].clone());
            }
        }
        *keys = new_keys;
        *valid = new_valid;
    }

    /// X-17: for exact EntityId misses, try bounded fuzzy match against graph.
    async fn apply_graph_fuzzy_resolution(
        &self,
        keys: &mut [String],
        valid: &mut [ExtractedEntity],
        existing_map: &mut HashMap<String, GraphNode>,
    ) -> Result<()> {
        use edgequake_storage::traits::NodeListFilter;

        let miss_idxs: Vec<usize> = keys
            .iter()
            .enumerate()
            .filter(|(i, k)| !existing_map.contains_key(k.as_str()) && valid.get(*i).is_some())
            .map(|(i, _)| i)
            .collect();
        if miss_idxs.is_empty() {
            return Ok(());
        }

        let page = self
            .graph_storage
            .list_nodes_filtered(
                &NodeListFilter {
                    tenant_id: self.tenant_id.clone(),
                    workspace_id: self.workspace_id.clone(),
                    ..Default::default()
                },
                0,
                500,
            )
            .await?;
        if page.items.is_empty() {
            return Ok(());
        }
        let candidates: Vec<String> = page.items.iter().map(|n| n.id.clone()).collect();
        let threshold = edgequake_storage::fuzzy_match_threshold();

        for i in miss_idxs {
            let bare = EntityId::bare_name_from_graph_node_id(&keys[i]).to_string();
            let Some(hit) = edgequake_storage::find_best_fuzzy_match(
                &bare,
                candidates.iter().map(String::as_str),
                threshold,
            ) else {
                continue;
            };
            if hit == keys[i] {
                continue;
            }
            // Load the fuzzy target if not already in the map.
            if !existing_map.contains_key(hit) {
                if let Some(node) = self.graph_storage.get_node(hit).await? {
                    existing_map.insert(hit.to_string(), node);
                } else {
                    continue;
                }
            }
            tracing::info!(
                from = %keys[i],
                onto = %hit,
                metric = "entity_fuzzy_graph",
                "X-17: fuzzy resolve onto existing graph node"
            );
            keys[i] = hit.to_string();
            // Align entity name so build_entity_merge_outcome uses the same key.
            valid[i].name = EntityId::bare_name_from_graph_node_id(hit).to_string();
        }
        Ok(())
    }

    /// Fold entities that share the same graph key after fuzzy remapping.
    fn collapse_duplicate_keys(&self, keys: &mut Vec<String>, valid: &mut Vec<ExtractedEntity>) {
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut keep = vec![true; keys.len()];
        for i in 0..keys.len() {
            if let Some(&onto) = seen.get(&keys[i]) {
                keep[i] = false;
                if valid[i].description.len() > valid[onto].description.len() {
                    valid[onto].description = valid[i].description.clone();
                }
                for cid in valid[i].source_chunk_ids.clone() {
                    if !valid[onto].source_chunk_ids.contains(&cid) {
                        valid[onto].source_chunk_ids.push(cid);
                    }
                }
                let incoming_type = valid[i].entity_type.clone();
                let incoming_importance = valid[i].importance;
                let incoming_name = valid[i].name.clone();
                let mut votes = HashMap::new();
                merge_type_into_entity(
                    &mut valid[onto].entity_type,
                    &mut votes,
                    &incoming_type,
                    incoming_importance,
                    &incoming_name,
                );
                if incoming_importance > valid[onto].importance {
                    valid[onto].importance = incoming_importance;
                }
            } else {
                seen.insert(keys[i].clone(), i);
            }
        }
        if keep.iter().all(|&k| k) {
            return;
        }
        let mut new_keys = Vec::new();
        let mut new_valid = Vec::new();
        for (i, keep_i) in keep.into_iter().enumerate() {
            if keep_i {
                new_keys.push(keys[i].clone());
                new_valid.push(valid[i].clone());
            }
        }
        *keys = new_keys;
        *valid = new_valid;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merger::{KnowledgeGraphMerger, MergeStats, MergerConfig};
    use edgequake_storage::traits::{GraphStorage, GraphStorageReadOps};
    use edgequake_storage::{MemoryGraphStorage, MemoryVectorStorage};
    use std::sync::Arc;

    fn make_entity(name: &str, entity_type: &str, importance: f32, chunk: &str) -> ExtractedEntity {
        ExtractedEntity {
            name: name.to_string(),
            entity_type: entity_type.to_string(),
            description: format!("{name} as {entity_type}"),
            importance,
            source_spans: vec![],
            source_chunk_ids: vec![chunk.to_string()],
            embedding: None,
            source_document_id: Some("doc-d32".to_string()),
            source_file_path: None,
            display_name: None,
            page_num: None,
            figure_index: None,
            asset_id: None,
            mm_subtype: None,
        }
    }

    /// D-32: conflicting types resolve by majority/confidence (not silent first-wins).
    #[tokio::test]
    async fn e2e_entity_type_conflict_logged_and_resolved() {
        let graph = Arc::new(MemoryGraphStorage::new("d32"));
        let vector = Arc::new(MemoryVectorStorage::new("d32", 4));
        graph.initialize().await.unwrap();
        vector.initialize().await.unwrap();

        let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector);

        // First mention: PERSON (weak)
        let mut stats = MergeStats::default();
        merger
            .merge_entities_batch(
                vec![make_entity("Acme Corp", "PERSON", 0.4, "c0")],
                &mut stats,
            )
            .await
            .unwrap();

        // Two stronger ORGANIZATION votes should win.
        merger
            .merge_entities_batch(
                vec![
                    make_entity("Acme Corp", "ORGANIZATION", 0.9, "c1"),
                    make_entity("Acme Corp", "ORGANIZATION", 0.85, "c2"),
                ],
                &mut stats,
            )
            .await
            .unwrap();

        let node = graph
            .get_node("ACME_CORP")
            .await
            .unwrap()
            .expect("node must exist");
        assert_eq!(
            node.properties.get("entity_type").and_then(|v| v.as_str()),
            Some("ORGANIZATION"),
            "majority/confidence must override first-wins PERSON"
        );
        assert!(
            node.properties.contains_key(ENTITY_TYPE_VOTES_KEY),
            "vote map must be persisted for observability"
        );
    }

    #[tokio::test]
    async fn contract_x_17_fuzzy_collapses_near_duplicate_when_enabled() {
        std::env::set_var("EDGEQUAKE_ENTITY_FUZZY", "1");
        // Lower threshold so ACME_CORP ↔ ACME_CORP_INC collapses in CI.
        std::env::set_var("EDGEQUAKE_ENTITY_FUZZY_THRESHOLD", "0.60");

        let graph = Arc::new(MemoryGraphStorage::new("x17"));
        let vector = Arc::new(MemoryVectorStorage::new("x17", 4));
        graph.initialize().await.unwrap();
        vector.initialize().await.unwrap();
        let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector);

        let mut stats = MergeStats::default();
        merger
            .merge_entities_batch(
                vec![make_entity("Acme Corp Inc", "ORGANIZATION", 0.8, "c0")],
                &mut stats,
            )
            .await
            .unwrap();
        merger
            .merge_entities_batch(
                vec![make_entity("Acme Corp", "ORGANIZATION", 0.8, "c1")],
                &mut stats,
            )
            .await
            .unwrap();

        // Fuzzy should map ACME_CORP onto ACME_CORP_INC (or vice versa) → 1 node.
        let a = graph.get_node("ACME_CORP_INC").await.unwrap();
        let b = graph.get_node("ACME_CORP").await.unwrap();
        let present = a.is_some() as u8 + b.is_some() as u8;
        assert_eq!(
            present, 1,
            "fuzzy on: near-duplicate names must collapse to one node"
        );

        std::env::remove_var("EDGEQUAKE_ENTITY_FUZZY");
        std::env::remove_var("EDGEQUAKE_ENTITY_FUZZY_THRESHOLD");
    }
}
