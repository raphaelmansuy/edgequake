//! Document and entity deletion operations for EdgeQuake.
//!
//! Contains `delete_document()`, `analyze_deletion_impact()`, `delete_entity()`.

use crate::error::{Error, Result};
use crate::types::{DocumentDeletionResult, EntityDeletionResult};

use super::EdgeQuake;

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

        // 1. Find and delete chunks belonging to this document
        let chunk_prefix = format!("{}-chunk-", document_id);
        let keys = kv_storage.keys().await?;
        let chunk_ids: Vec<String> = keys
            .iter()
            .filter(|k| k.starts_with(&chunk_prefix))
            .cloned()
            .collect();

        result.chunks_deleted = chunk_ids.len();

        // 2. Process graph entities - remove document sources
        let all_nodes = graph_storage.get_all_nodes().await?;
        for node in all_nodes {
            // Check if this node has any sources from the deleted document
            if let Some(source_id) = node.properties.get("source_id").and_then(|v| v.as_str()) {
                let sources: Vec<&str> = source_id.split('|').collect();
                let remaining_sources: Vec<&str> = sources
                    .into_iter()
                    .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(document_id))
                    .collect();

                if remaining_sources.is_empty() {
                    // No sources left - delete the entity entirely
                    // First delete all connected edges
                    let edges = graph_storage.get_node_edges(&node.id).await?;
                    for edge in edges {
                        graph_storage
                            .delete_edge(&edge.source, &edge.target)
                            .await?;
                        result.relationships_removed += 1;
                    }
                    // Then delete the node
                    graph_storage.delete_node(&node.id).await?;
                    // Also delete from vector storage
                    let _ = vector_storage.delete_entity(&node.id).await;
                    result.entities_removed += 1;
                } else if remaining_sources.len() < source_id.split('|').count() {
                    // Some sources were removed - update the entity
                    let mut updated_props = node.properties.clone();
                    updated_props.insert(
                        "source_id".to_string(),
                        serde_json::json!(remaining_sources.join("|")),
                    );
                    graph_storage.upsert_node(&node.id, updated_props).await?;
                    result.entities_updated += 1;
                }
            }
        }

        // 3. Process graph edges - remove document sources
        let all_edges = graph_storage.get_all_edges().await?;
        for edge in all_edges {
            if let Some(source_id) = edge.properties.get("source_id").and_then(|v| v.as_str()) {
                let sources: Vec<&str> = source_id.split('|').collect();
                let remaining_sources: Vec<&str> = sources
                    .into_iter()
                    .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(document_id))
                    .collect();

                if remaining_sources.is_empty() {
                    // No sources left - delete the relationship
                    graph_storage
                        .delete_edge(&edge.source, &edge.target)
                        .await?;
                    result.relationships_removed += 1;
                } else if remaining_sources.len() < source_id.split('|').count() {
                    // Some sources were removed - update the relationship
                    let mut updated_props = edge.properties.clone();
                    updated_props.insert(
                        "source_id".to_string(),
                        serde_json::json!(remaining_sources.join("|")),
                    );
                    graph_storage
                        .upsert_edge(&edge.source, &edge.target, updated_props)
                        .await?;
                    result.relationships_updated += 1;
                }
            }
        }

        // 4. Delete chunks and document metadata from KV storage
        let mut keys_to_delete = chunk_ids;
        let metadata_key = format!("{}-metadata", document_id);
        let content_key = format!("{}-content", document_id);
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

    /// Analyze the impact of deleting a document before actually deleting it.
    ///
    /// # Implements
    ///
    /// - **UC0006**: Preview Document Deletion Impact
    /// - **FEAT0012**: Deletion Impact Analysis
    ///
    /// # WHY: Pre-Flight Impact Visibility
    ///
    /// Before destructive operations, users need to understand what will change.
    /// This method performs a dry-run of deletion to show:
    /// - How many chunks will be removed
    /// - Which entities will be fully deleted vs. partially updated
    /// - Which relationships will be affected
    ///
    /// This implements impact analysis (P4-06) from the LightRAG specification.
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

        // Count chunks
        let chunk_prefix = format!("{}-chunk-", document_id);
        let keys = kv_storage.keys().await?;
        result.chunks_deleted = keys.iter().filter(|k| k.starts_with(&chunk_prefix)).count();

        // Analyze entities
        let all_nodes = graph_storage.get_all_nodes().await?;
        for node in all_nodes {
            if let Some(source_id) = node.properties.get("source_id").and_then(|v| v.as_str()) {
                let sources: Vec<&str> = source_id.split('|').collect();
                let remaining = sources
                    .iter()
                    .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(document_id))
                    .count();

                if remaining == 0 {
                    result.entities_removed += 1;
                } else if remaining < sources.len() {
                    result.entities_updated += 1;
                }
            }
        }

        // Analyze edges
        let all_edges = graph_storage.get_all_edges().await?;
        for edge in all_edges {
            if let Some(source_id) = edge.properties.get("source_id").and_then(|v| v.as_str()) {
                let sources: Vec<&str> = source_id.split('|').collect();
                let remaining = sources
                    .iter()
                    .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(document_id))
                    .count();

                if remaining == 0 {
                    result.relationships_removed += 1;
                } else if remaining < sources.len() {
                    result.relationships_updated += 1;
                }
            }
        }

        Ok(result)
    }

    /// Delete an entity and its relationships from the knowledge graph.
    ///
    /// # Implements
    ///
    /// - **UC0103**: Delete Entity from Graph
    /// - **FEAT0203**: Graph Mutation Operations
    ///
    /// # Enforces
    ///
    /// - **BR0008**: Entity names are normalized (UPPERCASE with underscores)
    /// - **BR0201**: Tenant isolation (deletion scoped to tenant)
    ///
    /// # WHY: Cascade Edge Deletion
    ///
    /// When an entity is deleted, all connected edges must also be deleted.
    /// Orphan edges would corrupt graph traversal queries. The deletion order is:
    /// 1. Find and delete all edges where entity is source or target
    /// 2. Delete the node itself from graph storage
    /// 3. Delete the entity embedding from vector storage
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

        // First, delete all edges connected to this entity
        let edges = graph_storage.get_node_edges(&normalized_name).await?;
        for edge in edges {
            graph_storage
                .delete_edge(&edge.source, &edge.target)
                .await?;
            relationships_deleted += 1;
        }

        // Delete the node from graph storage
        graph_storage.delete_node(&normalized_name).await?;

        // Delete from vector storage
        let _ = vector_storage.delete_entity(&normalized_name).await;

        Ok(EntityDeletionResult {
            entity_name: normalized_name,
            deleted: true,
            relationships_deleted,
        })
    }
}
