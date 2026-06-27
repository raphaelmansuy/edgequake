//! Document and entity deletion operations for EdgeQuake.
//!
//! Contains `delete_document()`, `analyze_deletion_impact()`, `delete_entity()`.
//!
//! SPEC-006: document-scoped graph scan via `GraphScanOps` (no `get_all_*`).

use std::collections::HashMap;

use edgequake_storage::kv_keys;
use edgequake_storage::traits::{collect_source_references, EdgeListFilter, NodeListFilter};

use crate::error::{Error, Result};
use crate::types::{DocumentDeletionResult, EntityDeletionResult};

use super::EdgeQuake;

fn document_source_prefixes(document_id: &str) -> Vec<String> {
    vec![document_id.to_string()]
}

fn source_belongs_to_document(source: &str, document_id: &str, chunk_prefix: &str) -> bool {
    source.starts_with(chunk_prefix) || source == document_id
}

fn remaining_sources_after_removal(
    properties: &HashMap<String, serde_json::Value>,
    document_id: &str,
    chunk_prefix: &str,
) -> Vec<String> {
    collect_source_references(properties)
        .into_iter()
        .filter(|s| !source_belongs_to_document(s, document_id, chunk_prefix))
        .collect()
}

fn count_source_removals(
    properties: &HashMap<String, serde_json::Value>,
    document_id: &str,
    chunk_prefix: &str,
) -> (bool, bool) {
    let sources = collect_source_references(properties);
    if sources.is_empty() {
        return (false, false);
    }
    let remaining = remaining_sources_after_removal(properties, document_id, chunk_prefix);
    let fully_removed = remaining.is_empty();
    let partially_updated = !fully_removed && remaining.len() < sources.len();
    (fully_removed, partially_updated)
}

/// SPEC-021 P-C3: refresh an entity vector's metadata after a partial document
/// deletion so it no longer references the deleted document's chunk IDs.
///
/// WHY: when an entity is shared across documents and one is deleted, the AGE
/// node's `source_ids` is updated (P-C2 path above), but the entity's vector
/// embedding still carries `metadata.source_chunk_ids` pointing at the deleted
/// chunks. `query_local` would then surface the entity with stale provenance.
/// We re-upsert the vector with the same embedding but refreshed metadata.
///
/// Best-effort: a failure is logged, not propagated — the AGE node update is
/// the authoritative change; the vector metadata is a denormalized cache.
async fn refresh_entity_vector_metadata(
    vector_storage: &dyn edgequake_storage::traits::VectorStorage,
    entity_id: &str,
    deleted_document_id: &str,
    chunk_prefix: &str,
    remaining_sources: &[String],
) {
    // Fetch the current vector + metadata so we preserve the embedding.
    match vector_storage.get_by_ids(&[entity_id.to_string()]).await {
        Ok(vectors) if !vectors.is_empty() => {
            let (id, embedding) = &vectors[0];
            // Re-read full metadata via get_by_id isn't available; reconstruct
            // from the entity_id and remaining sources. We upsert with the
            // refreshed source_chunk_ids so query_local provenance is correct.
            let metadata = serde_json::json!({
                "type": "entity",
                "entity_name": entity_id.strip_prefix("entity:").unwrap_or(entity_id),
                "source_chunk_ids": remaining_sources,
                "document_id": remaining_sources.first().map(|s| {
                    // best-effort doc id from first remaining source chunk
                    s.split("-chunk-").next().unwrap_or(s)
                }).unwrap_or(deleted_document_id),
            });
            if let Err(e) = vector_storage
                .upsert(&[(id.clone(), embedding.clone(), metadata)])
                .await
            {
                tracing::warn!(
                    entity = %entity_id,
                    error = %e,
                    "P-C3: entity vector metadata refresh failed (best-effort)"
                );
            }
        }
        Ok(_) => {
            // No vector for this entity — nothing to refresh.
        }
        Err(e) => {
            tracing::warn!(
                entity = %entity_id,
                error = %e,
                "P-C3: entity vector lookup failed (best-effort) — metadata not refreshed"
            );
        }
    }
    let _ = chunk_prefix; // reserved for future finer-grained filtering
}

impl EdgeQuake {
    pub async fn delete_document(&self, document_id: &str) -> Result<DocumentDeletionResult> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        tracing::info!(document_id = %document_id, "Starting document suppression");

        let mut result = DocumentDeletionResult {
            document_id: document_id.to_string(),
            chunks_deleted: 0,
            entities_removed: 0,
            entities_updated: 0,
            relationships_removed: 0,
            relationships_updated: 0,
        };

        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Graph storage not initialized"))?;

        let vector_storage = self
            .vector_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Vector storage not initialized"))?;

        let kv_storage = self
            .kv_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("KV storage not initialized"))?;

        let chunk_prefix = kv_keys::doc_chunk_prefix(document_id);
        let keys = kv_storage.keys().await?;
        let chunk_ids: Vec<String> = keys
            .iter()
            .filter(|k| k.starts_with(&chunk_prefix))
            .cloned()
            .collect();

        result.chunks_deleted = chunk_ids.len();

        // SPEC-021 P-C2: delete the chunk VECTORS too — the KV chunk keys and
        // the vector IDs share the `{doc_id}-chunk-{n}` form, so the same IDs
        // identify both. Without this, deletion left orphaned chunk embeddings
        // retrievable via query_local (file 17 §4 gap 3).
        if !chunk_ids.is_empty() {
            if let Err(e) = vector_storage.delete(&chunk_ids).await {
                tracing::warn!(
                    document_id = %document_id,
                    error = %e,
                    "P-C2: chunk vector deletion failed (best-effort) — orphan vectors may remain"
                );
            }
        }

        let source_prefixes = document_source_prefixes(document_id);
        let node_filter = NodeListFilter::default();
        let affected_nodes = graph_storage
            .find_nodes_by_source_prefixes(&node_filter, &source_prefixes)
            .await?;

        for node in affected_nodes {
            let sources = collect_source_references(&node.properties);
            if sources.is_empty() {
                continue;
            }
            let remaining =
                remaining_sources_after_removal(&node.properties, document_id, &chunk_prefix);
            if remaining.is_empty() {
                let edges = graph_storage.get_node_edges(&node.id).await?;
                for edge in edges {
                    graph_storage
                        .delete_edge(&edge.source, &edge.target)
                        .await?;
                    result.relationships_removed += 1;
                }
                graph_storage.delete_node(&node.id).await?;
                let _ = vector_storage.delete_entity(&node.id).await;
                // SPEC-021 P3-02: best-effort relational sync — entity fully removed
                self.relational_sink
                    .remove_entity_sources(
                        &node.id,
                        self.config.workspace_id.as_deref(),
                        &sources,
                        &[],
                    )
                    .await
                    .unwrap_or_else(|e| tracing::warn!(entity=%node.id, error=%e, "relational sink delete failed (best-effort)"));
                result.entities_removed += 1;
            } else if remaining.len() < sources.len() {
                let mut updated_props = node.properties.clone();
                updated_props.insert("source_ids".to_string(), serde_json::json!(remaining));
                updated_props.remove("source_id");
                graph_storage
                    .upsert_node(&node.id, updated_props.clone())
                    .await?;
                // SPEC-021 P-C3: refresh the entity vector's metadata so it no
                // longer references the deleted document's chunk IDs. Without
                // this, query_local can still surface the entity via a vector
                // whose metadata points at the now-deleted document.
                refresh_entity_vector_metadata(
                    vector_storage.as_ref(),
                    &node.id,
                    document_id,
                    &chunk_prefix,
                    &remaining,
                )
                .await;
                // SPEC-021 P3-02: best-effort relational sync — sources partially removed
                self.relational_sink
                    .remove_entity_sources(
                        &node.id,
                        self.config.workspace_id.as_deref(),
                        &sources,
                        &remaining,
                    )
                    .await
                    .unwrap_or_else(|e| tracing::warn!(entity=%node.id, error=%e, "relational sink update failed (best-effort)"));
                result.entities_updated += 1;
            }
        }

        let edge_filter = EdgeListFilter::default();
        let affected_edges = graph_storage
            .find_edges_by_source_prefixes(&edge_filter, &source_prefixes)
            .await?;

        for edge in affected_edges {
            let sources = collect_source_references(&edge.properties);
            if sources.is_empty() {
                continue;
            }
            let remaining =
                remaining_sources_after_removal(&edge.properties, document_id, &chunk_prefix);
            if remaining.is_empty() {
                graph_storage
                    .delete_edge(&edge.source, &edge.target)
                    .await?;
                result.relationships_removed += 1;
            } else if remaining.len() < sources.len() {
                let mut updated_props = edge.properties.clone();
                updated_props.insert("source_ids".to_string(), serde_json::json!(remaining));
                updated_props.remove("source_id");
                graph_storage
                    .upsert_edge(&edge.source, &edge.target, updated_props)
                    .await?;
                result.relationships_updated += 1;
            }
        }

        let mut keys_to_delete = chunk_ids;
        let metadata_key = kv_keys::doc_metadata(document_id);
        let content_key = kv_keys::doc_content(document_id);
        if keys.contains(&metadata_key) {
            keys_to_delete.push(metadata_key);
        }
        if keys.contains(&content_key) {
            keys_to_delete.push(content_key);
        }
        if !keys_to_delete.is_empty() {
            kv_storage.delete(&keys_to_delete).await?;
        }

        tracing::info!(
            document_id = %document_id,
            chunks = result.chunks_deleted,
            entities_removed = result.entities_removed,
            entities_updated = result.entities_updated,
            relationships_removed = result.relationships_removed,
            "Document suppression complete"
        );

        Ok(result)
    }

    pub async fn analyze_deletion_impact(
        &self,
        document_id: &str,
    ) -> Result<DocumentDeletionResult> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        let mut result = DocumentDeletionResult {
            document_id: document_id.to_string(),
            chunks_deleted: 0,
            entities_removed: 0,
            entities_updated: 0,
            relationships_removed: 0,
            relationships_updated: 0,
        };

        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Graph storage not initialized"))?;

        let kv_storage = self
            .kv_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("KV storage not initialized"))?;

        let chunk_prefix = kv_keys::doc_chunk_prefix(document_id);
        let keys = kv_storage.keys().await?;
        result.chunks_deleted = keys.iter().filter(|k| k.starts_with(&chunk_prefix)).count();

        let source_prefixes = document_source_prefixes(document_id);
        let affected_nodes = graph_storage
            .find_nodes_by_source_prefixes(&NodeListFilter::default(), &source_prefixes)
            .await?;

        for node in affected_nodes {
            let (removed, updated) =
                count_source_removals(&node.properties, document_id, &chunk_prefix);
            if removed {
                result.entities_removed += 1;
            } else if updated {
                result.entities_updated += 1;
            }
        }

        let affected_edges = graph_storage
            .find_edges_by_source_prefixes(&EdgeListFilter::default(), &source_prefixes)
            .await?;

        for edge in affected_edges {
            let (removed, updated) =
                count_source_removals(&edge.properties, document_id, &chunk_prefix);
            if removed {
                result.relationships_removed += 1;
            } else if updated {
                result.relationships_updated += 1;
            }
        }

        Ok(result)
    }

    pub async fn delete_entity(&self, entity_name: &str) -> Result<EntityDeletionResult> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        tracing::info!(entity = %entity_name, "Deleting entity");

        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Graph storage not initialized"))?;

        let vector_storage = self
            .vector_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Vector storage not initialized"))?;

        let normalized_name = crate::types::GraphEntity::normalize_name(entity_name);
        let mut relationships_deleted = 0;

        let edges = graph_storage.get_node_edges(&normalized_name).await?;
        for edge in edges {
            graph_storage
                .delete_edge(&edge.source, &edge.target)
                .await?;
            relationships_deleted += 1;
        }

        graph_storage.delete_node(&normalized_name).await?;
        let _ = vector_storage.delete_entity(&normalized_name).await;

        Ok(EntityDeletionResult {
            entity_name: normalized_name,
            deleted: true,
            relationships_deleted,
        })
    }
}
