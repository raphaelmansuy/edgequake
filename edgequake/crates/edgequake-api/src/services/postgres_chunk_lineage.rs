//! Document lineage from PostgreSQL chunk link tables (migration 066).
//!
//! Fallback when AGE graph nodes lack `source_id` / `source_ids` properties
//! (common for documents ingested before graph provenance was wired).

#[cfg(feature = "postgres")]
use std::collections::HashSet;

#[cfg(feature = "postgres")]
use edgequake_storage::traits::GraphStorageReadOps;
#[cfg(feature = "postgres")]
use sqlx::PgPool;

#[cfg(feature = "postgres")]
use crate::error::{ApiError, ApiResult};
#[cfg(feature = "postgres")]
use crate::handlers::lineage_types::{EntitySummaryResponse, RelationshipSummaryResponse};
#[cfg(feature = "postgres")]
use crate::middleware::TenantContext;
#[cfg(feature = "postgres")]
use crate::services::entity_graph_lookup::lookup_entity_node_for_context;

/// Inclusive/exclusive bounds for chunk ids of one document.
///
/// `chunk_id LIKE $prefix` hides the prefix from a generic plan, so a miss
/// seq-scans every link. `chunk_id >= lower AND chunk_id < upper` is an index
/// condition on `chunk_entity_links_pkey`.
pub fn document_chunk_id_bounds(document_id: &str) -> (String, String) {
    let lower = format!("{document_id}-chunk-");
    let upper = prefix_successor(&lower).unwrap_or_else(|| format!("{lower}\u{10FFFF}"));
    (lower, upper)
}

/// Smallest string strictly greater than every string with this prefix.
fn prefix_successor(prefix: &str) -> Option<String> {
    let mut bytes = prefix.as_bytes().to_vec();
    while let Some(last) = bytes.pop() {
        if last < 0xff {
            bytes.push(last + 1);
            return String::from_utf8(bytes).ok();
        }
    }
    None
}

/// Load document-scoped entities/relationships from `chunk_entity_links` /
/// `chunk_relation_links` when graph source-prefix queries return nothing.
#[cfg(feature = "postgres")]
pub async fn load_document_lineage_from_chunk_links(
    pool: &PgPool,
    tenant_ctx: &TenantContext,
    document_id: &str,
    graph: &dyn GraphStorageReadOps,
) -> ApiResult<(Vec<EntitySummaryResponse>, Vec<RelationshipSummaryResponse>)> {
    let workspace_id = tenant_ctx.workspace_id.as_deref().ok_or_else(|| {
        ApiError::BadRequest("Workspace context required for document lineage".into())
    })?;

    let (chunk_lower, chunk_upper) = document_chunk_id_bounds(document_id);

    let entity_rows: Vec<(String, Vec<String>)> = sqlx::query_as(
        "SELECT entity_name, array_agg(DISTINCT chunk_id ORDER BY chunk_id) AS source_chunks
         FROM chunk_entity_links
         WHERE workspace_id = $1 AND chunk_id >= $2 AND chunk_id < $3
         GROUP BY entity_name
         ORDER BY entity_name",
    )
    .bind(workspace_id)
    .bind(&chunk_lower)
    .bind(&chunk_upper)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("chunk_entity_links query failed: {e}")))?;

    if entity_rows.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    let entity_names: Vec<String> = entity_rows.iter().map(|(n, _)| n.clone()).collect();

    // `idx_cel_entity_workspace_chunk` covers chunk_id, so this count
    // is an index-only scan. The heap index stops at the names but
    // still fetches every link.
    let shared_entities: HashSet<String> = sqlx::query_scalar(
        "SELECT entity_name
         FROM chunk_entity_links
         WHERE workspace_id = $1 AND entity_name = ANY($2)
         GROUP BY entity_name
         HAVING COUNT(DISTINCT split_part(chunk_id, '-chunk-', 1)) > 1",
    )
    .bind(workspace_id)
    .bind(&entity_names)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("shared entity query failed: {e}")))?
    .into_iter()
    .collect();

    let mut entities = Vec::with_capacity(entity_rows.len());
    for (name, source_chunks) in entity_rows {
        let summary = match lookup_entity_node_for_context(graph, &name, tenant_ctx).await {
            Ok(node) => {
                let label = crate::handlers::graph::graph_node_label(&node);
                let bare = edgequake_pipeline::bare_entity_id(&node.id);
                let display_name = if edgequake_storage::is_opaque_identifier(bare) {
                    label.clone()
                } else {
                    bare.to_string()
                };
                let entity_type = node
                    .properties
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let description = node
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
                EntitySummaryResponse {
                    id: node.id.clone(),
                    name: display_name,
                    label,
                    entity_type,
                    source_chunks,
                    is_shared: shared_entities.contains(&name),
                    description,
                }
            }
            Err(_) => {
                let entity_type = sqlx::query_scalar::<_, String>(
                    "SELECT entity_type FROM entities
                     WHERE workspace_id = $1::uuid AND name = $2
                     LIMIT 1",
                )
                .bind(workspace_id)
                .bind(&name)
                .fetch_optional(pool)
                .await
                .map_err(|e| ApiError::Internal(format!("entities lookup failed: {e}")))?
                .unwrap_or_else(|| "unknown".to_string());
                let bare = edgequake_pipeline::bare_entity_id(&name);
                let label = if edgequake_storage::is_opaque_identifier(bare) {
                    edgequake_pipeline::soft_label_opaque(Some(&entity_type), None)
                } else {
                    bare.to_string()
                };
                let display_name = if edgequake_storage::is_opaque_identifier(bare) {
                    label.clone()
                } else {
                    bare.to_string()
                };
                EntitySummaryResponse {
                    id: name.clone(),
                    name: display_name,
                    label,
                    entity_type,
                    source_chunks,
                    is_shared: shared_entities.contains(&name),
                    description: None,
                }
            }
        };
        entities.push(summary);
    }

    let relation_rows: Vec<(String, String, Vec<String>)> = sqlx::query_as(
        "SELECT source_entity, target_entity,
                array_agg(DISTINCT chunk_id ORDER BY chunk_id) AS source_chunks
         FROM chunk_relation_links
         WHERE workspace_id = $1 AND chunk_id >= $2 AND chunk_id < $3
         GROUP BY source_entity, target_entity
         ORDER BY source_entity, target_entity",
    )
    .bind(workspace_id)
    .bind(&chunk_lower)
    .bind(&chunk_upper)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("chunk_relation_links query failed: {e}")))?;

    let mut relationships = Vec::with_capacity(relation_rows.len());
    for (source, target, source_chunks) in relation_rows {
        let keywords = graph
            .get_edge(&source, &target)
            .await
            .ok()
            .flatten()
            .and_then(|edge| {
                edge.properties
                    .get("keywords")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_default();

        relationships.push(RelationshipSummaryResponse {
            source,
            target,
            keywords,
            source_chunks,
        });
    }

    Ok((entities, relationships))
}

#[cfg(test)]
mod tests {
    use super::document_chunk_id_bounds;

    #[test]
    fn chunk_bounds_cover_one_document_prefix() {
        let (lower, upper) = document_chunk_id_bounds("019f419f-e29e-7cc9-ae40-cb6b28286f45");
        assert_eq!(lower, "019f419f-e29e-7cc9-ae40-cb6b28286f45-chunk-");
        assert_eq!(upper, "019f419f-e29e-7cc9-ae40-cb6b28286f45-chunk.");
        let inside = format!("{lower}12");
        assert!(inside >= lower && inside < upper);
        assert!(upper >= lower);
        let other = "019f419f-e29e-7cc9-ae40-cb6b28286f45-chunka";
        assert!(other >= upper.as_str());
    }
}
