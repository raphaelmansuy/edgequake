//! Helper functions for query processing.
//!
//! This module provides common utilities for extracting metadata,
//! building context objects, and processing search results.
//!
//! # WHY This Module Exists
//!
//! Before this extraction, the same patterns were repeated 5-7 times
//! across `engine_impl.rs`. This caused:
//! - Inconsistent handling of edge cases
//! - Hard-to-maintain code (changes required 7 edits)
//! - Risk of divergent implementations
//!
//! By centralizing these patterns, we ensure consistent behavior and
//! reduce the surface area for bugs.

use std::collections::HashMap;

use serde_json::Value;

use crate::context::{RetrievedChunk, RetrievedEntity, RetrievedRelationship};
use edgequake_storage::traits::VectorSearchResult;

/// Decode the entity name from a vector search result (SPEC-021 P-E1 / R-SOLID-04).
///
/// WHY: the writer stores entity vectors with id `entity:{name}` (text_insert.rs)
/// and mirrors the name into `metadata.entity_name`. The graph node id is the
/// bare `{name}` (e.g. "ALPHA"). Both the storage ID and metadata encode the
/// same name at write time, but:
///   - `metadata.entity_name` is the explicit canonical name and is what the
///     graph node id uses, so it is preferred when present.
///   - The storage ID is used as a fallback for legacy vectors that carry no
///     `entity_name` metadata (the `entity:` prefix is stripped to recover the
///     bare name). This also covers the P-C3 drift case where a partial delete
///     refreshes metadata without `entity_name`.
///
/// This keeps a single decoding rule in one place (DRY) and avoids misreading
/// chunk/relationship IDs as entity names (SOLID — single responsibility).
///
/// P-G6a: moved here from the deleted `strategies/mod.rs`; the query engine is
/// the only remaining caller, so the helper lives with the other shared
/// retrieval helpers rather than in a dead strategies module.
/// Map a bare entity name (from vector metadata) to the AGE graph node id.
///
/// SPEC-032 / B3b: when `workspace_id` is set, writers store
/// `{workspace_id}::{NORMALIZED}`; query must use the same key for
/// `get_nodes_batch` / neighborhood expand.
pub(crate) fn graph_entity_id_for_workspace(bare_name: &str, workspace_id: Option<&str>) -> String {
    use edgequake_storage::EntityId;
    let id = EntityId::new(bare_name);
    if id.is_empty() {
        return String::new();
    }
    id.graph_node_id_for_workspace(workspace_id)
}

pub(crate) fn decode_entity_name_from_result(
    storage_id: &str,
    metadata: &serde_json::Value,
) -> String {
    use edgequake_storage::vector_id::VectorId;

    // 1. Prefer the explicit canonical name in metadata.
    if let Some(name) = metadata
        .get("entity_name")
        .and_then(|v| v.as_str())
        .map(|s| s.strip_prefix("entity:").unwrap_or(s).to_string())
    {
        if !name.is_empty() {
            return name;
        }
    }

    // 2. Fall back to decoding the storage ID (must be an Entity variant).
    VectorId::from_storage_id(storage_id)
        .and_then(|vid| match vid {
            VectorId::Entity { name } => Some(
                name.strip_prefix("entity:")
                    .map(|s| s.to_string())
                    .unwrap_or(name),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

/// Source tracking information extracted from entity nodes.
///
/// WHY: Entities in the knowledge graph track their provenance
/// (which chunks/documents they came from) for citation purposes.
#[derive(Debug, Default, Clone)]
pub struct EntitySourceTracking {
    /// Chunk IDs that contributed to this entity.
    pub source_chunk_ids: Vec<String>,
    /// Primary source document ID (legacy single-doc field).
    pub source_document_id: Option<String>,
    /// Union of all document IDs this entity was extracted from.
    /// @implements SPEC-031: Multi-document entity lineage
    pub source_document_ids: Vec<String>,
    /// File path of the source document.
    pub source_file_path: Option<String>,
}

/// Source tracking information extracted from relationship edges.
///
/// WHY: LightRAG joins all relation chunk ids in `source_id` and admits every
/// part into the Mix pool. EQ edges store the same as `source_chunk_ids[]`
/// (049 union) plus legacy singular `source_chunk_id`.
#[derive(Debug, Default, Clone)]
pub struct RelationshipSourceTracking {
    /// All chunk IDs that contributed this relationship (LightRAG parity / 052).
    pub source_chunk_ids: Vec<String>,
    /// Primary / first chunk ID (legacy singular field).
    pub source_chunk_id: Option<String>,
    /// Primary source document ID.
    pub source_document_id: Option<String>,
    /// File path of the source document.
    pub source_file_path: Option<String>,
}

/// Extract document UUID from chunk ID.
///
/// # WHY: Chunk ID Format
///
/// Chunk IDs follow the format: `{document-uuid}-chunk-{N}`
/// Example: `f0291a69-8b63-46d5-b44b-24095b3a8283-chunk-0`
///
/// This function extracts the UUID portion for document linking,
/// enabling the UI to link citations back to source documents.
pub fn extract_document_id(chunk_id: &str) -> Option<String> {
    if let Some(suffix_idx) = chunk_id.rfind("-chunk-") {
        if suffix_idx > 0 {
            return Some(chunk_id[..suffix_idx].to_string());
        }
    }
    None
}

/// Extract source tracking from entity node properties.
///
/// # WHY: Centralized Extraction
///
/// Entity nodes store source tracking in a specific schema:
/// - `source_chunk_ids`: JSON array of chunk IDs
/// - `source_document_id`: String document UUID
/// - `source_file_path`: String file path
///
/// This function handles JSON parsing and null checks consistently.
pub fn extract_entity_source_tracking(props: &HashMap<String, Value>) -> EntitySourceTracking {
    let source_chunk_ids: Vec<String> = props
        .get("source_chunk_ids")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    // Plural first (union of all documents this entity appears in — SPEC-031)
    let source_document_ids: Vec<String> = props
        .get("source_document_ids")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    // Singular fallback (legacy single-doc provenance field)
    let source_document_id = props
        .get("source_document_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let source_file_path = props
        .get("source_file_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    EntitySourceTracking {
        source_chunk_ids,
        source_document_ids,
        source_document_id,
        source_file_path,
    }
}

/// Extract source tracking from relationship edge properties.
///
/// Prefers plural `source_chunk_ids` (049 merger union). Falls back to singular
/// `source_chunk_id` so pre-B6 edges still cite one chunk.
pub fn extract_relationship_source_tracking(
    props: &HashMap<String, Value>,
) -> RelationshipSourceTracking {
    let mut source_chunk_ids: Vec<String> = props
        .get("source_chunk_ids")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let source_chunk_id = props
        .get("source_chunk_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if let Some(ref singular) = source_chunk_id {
        if !source_chunk_ids.iter().any(|id| id == singular) {
            source_chunk_ids.insert(0, singular.clone());
        }
    }
    // Keep singular = first plural for callers that still read Option.
    let source_chunk_id = source_chunk_id.or_else(|| source_chunk_ids.first().cloned());

    let source_document_id = props
        .get("source_document_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let source_file_path = props
        .get("source_file_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    RelationshipSourceTracking {
        source_chunk_ids,
        source_chunk_id,
        source_document_id,
        source_file_path,
    }
}

/// Build a RetrievedChunk from a vector search result.
///
/// # WHY: Consistent Chunk Construction
///
/// When building chunks from vector results, we need to:
/// 1. Extract content from metadata
/// 2. Set the score
/// 3. Extract document ID from chunk ID
/// 4. Extract line numbers if available
/// 5. Extract chunk index if available
///
/// This function ensures all chunks are built consistently.
pub fn build_chunk_from_result(result: &VectorSearchResult) -> RetrievedChunk {
    let content = result
        .metadata
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut chunk = RetrievedChunk::new(&result.id, content, result.score);

    // Extract document_id from chunk_id (format: "uuid-chunk-N")
    if let Some(doc_id) = extract_document_id(&result.id) {
        chunk = chunk.with_document_id(doc_id);
    }

    // Extract line number information if available
    if let Some(start) = result.metadata.get("start_line").and_then(|v| v.as_u64()) {
        if let Some(end) = result.metadata.get("end_line").and_then(|v| v.as_u64()) {
            chunk = chunk.with_lines(start as usize, end as usize);
        }
    }

    if let Some(idx) = result.metadata.get("chunk_index").and_then(|v| v.as_u64()) {
        chunk = chunk.with_chunk_index(idx as usize);
    }

    // SPEC-032 W-09 / SPEC-135: PDF page attribution — deep-link uses start.
    if let Some(page) = result.metadata.get("page_start").and_then(|v| v.as_u64()) {
        let end = result
            .metadata
            .get("page_end")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        chunk = chunk.with_page_span(page as u32, end);
    }

    if let Some(modality) = result
        .metadata
        .get("modality")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    {
        chunk = chunk.with_modality(modality);
    }

    // SPEC-180: an image chunk carries no text (`content` is the file name). Downstream
    // steps that score text — notably the cross-encoder reranker — must be able to tell it
    // apart, or they drop the only evidence the user asked for.
    if result
        .metadata
        .get("source_modality")
        .and_then(|v| v.as_str())
        .is_some_and(|m| m.eq_ignore_ascii_case("image"))
    {
        chunk = chunk.with_source_modality("image");
    }

    chunk
}

/// Build a RetrievedEntity from a graph node.
///
/// # Arguments
/// * `node_id` - The entity identifier
/// * `props` - Node properties from the graph
/// * `degree` - Number of connections (for ranking)
/// * `score` - Similarity score from vector search
pub fn build_entity_from_node(
    node_id: &str,
    props: &HashMap<String, Value>,
    degree: usize,
    score: f32,
) -> RetrievedEntity {
    let entity_type = props
        .get("entity_type")
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN")
        .to_string();

    let description = props
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // 066/067/072: shared presentation SSOT (mm display_name + opaque soft-label).
    let existing_display = props.get("display_name").and_then(|v| v.as_str());
    let stored_label = props.get("label").and_then(|v| v.as_str());
    let description_ref = props.get("description").and_then(|v| v.as_str());
    let entity_type_ref = props.get("entity_type").and_then(|v| v.as_str());
    let display_name = edgequake_pipeline::resolve_entity_display_label(
        node_id,
        entity_type_ref,
        description_ref,
        existing_display,
        stored_label,
    );

    let source_tracking = extract_entity_source_tracking(props);

    let mut entity = RetrievedEntity::new(display_name, entity_type, description)
        .with_degree(degree)
        .with_score(score);

    if !source_tracking.source_chunk_ids.is_empty() {
        entity = entity.with_source_chunk_ids(source_tracking.source_chunk_ids);
    }
    if let Some(doc_id) = source_tracking.source_document_id {
        entity = entity.with_source_document_id(doc_id);
    }
    // SPEC-031: propagate multi-document lineage array
    if !source_tracking.source_document_ids.is_empty() {
        entity = entity.with_source_document_ids(source_tracking.source_document_ids);
    }
    if let Some(file_path) = source_tracking.source_file_path {
        entity = entity.with_source_file_path(file_path);
    }

    entity
}

/// Build a RetrievedRelationship from a graph edge.
///
/// # Arguments
/// * `source` - Source entity ID
/// * `target` - Target entity ID
/// * `props` - Edge properties from the graph
pub fn build_relationship_from_edge(
    source: &str,
    target: &str,
    props: &HashMap<String, Value>,
) -> RetrievedRelationship {
    let rel_type = props
        .get("relation_type")
        .and_then(|v| v.as_str())
        .unwrap_or("RELATED_TO")
        .to_string();

    let description = props
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let source_tracking = extract_relationship_source_tracking(props);

    let mut rel = RetrievedRelationship::new(source, target, rel_type);

    if let Some(desc) = description {
        rel = rel.with_description(desc);
    }
    if !source_tracking.source_chunk_ids.is_empty() {
        rel = rel.with_source_chunk_ids(source_tracking.source_chunk_ids);
    } else if let Some(chunk_id) = source_tracking.source_chunk_id {
        rel = rel.with_source_chunk_id(chunk_id);
    }
    if let Some(doc_id) = source_tracking.source_document_id {
        rel = rel.with_source_document_id(doc_id);
    }
    if let Some(file_path) = source_tracking.source_file_path {
        rel = rel.with_source_file_path(file_path);
    }

    rel
}

/// Presentation label for a relationship endpoint (073).
pub fn endpoint_display_label(node_id: &str, props: Option<&HashMap<String, Value>>) -> String {
    if let Some(props) = props {
        return edgequake_pipeline::resolve_entity_display_label(
            node_id,
            props.get("entity_type").and_then(|v| v.as_str()),
            props.get("description").and_then(|v| v.as_str()),
            props.get("display_name").and_then(|v| v.as_str()),
            props.get("label").and_then(|v| v.as_str()),
        );
    }
    let bare = edgequake_storage::EntityId::bare_name_from_graph_node_id(node_id);
    if edgequake_storage::is_opaque_identifier(bare) {
        edgequake_pipeline::soft_label_opaque(None, None)
    } else {
        bare.to_string()
    }
}

/// Soft-label relationship endpoints from a node-id → properties map (073).
pub fn apply_relationship_endpoint_labels(
    relationships: &mut [RetrievedRelationship],
    nodes_by_id: &HashMap<String, HashMap<String, Value>>,
) {
    for rel in relationships.iter_mut() {
        let src_props = nodes_by_id.get(&rel.source);
        let tgt_props = nodes_by_id.get(&rel.target);
        rel.source_label = endpoint_display_label(&rel.source, src_props);
        rel.target_label = endpoint_display_label(&rel.target, tgt_props);
    }
}

/// Collect unique endpoint ids, batch-fetch nodes, apply presentation labels (073).
pub async fn resolve_relationship_endpoint_labels(
    graph: &edgequake_storage::traits::GraphReadView<'_>,
    relationships: &mut [RetrievedRelationship],
) -> Result<(), edgequake_storage::StorageError> {
    if relationships.is_empty() {
        return Ok(());
    }
    let mut ids: Vec<String> = Vec::new();
    for rel in relationships.iter() {
        if !ids.iter().any(|id| id == &rel.source) {
            ids.push(rel.source.clone());
        }
        if !ids.iter().any(|id| id == &rel.target) {
            ids.push(rel.target.clone());
        }
    }
    let nodes_map = graph.get_nodes_batch(&ids).await?;
    let props_map: HashMap<String, HashMap<String, Value>> = nodes_map
        .into_iter()
        .map(|(id, node)| (id, node.properties))
        .collect();
    apply_relationship_endpoint_labels(relationships, &props_map);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_document_id() {
        // Standard format
        assert_eq!(
            extract_document_id("f0291a69-8b63-46d5-b44b-24095b3a8283-chunk-0"),
            Some("f0291a69-8b63-46d5-b44b-24095b3a8283".to_string())
        );

        // Multi-digit chunk index
        assert_eq!(
            extract_document_id("abc123-chunk-42"),
            Some("abc123".to_string())
        );

        // No chunk suffix
        assert_eq!(extract_document_id("just-a-uuid"), None);

        // Empty string
        assert_eq!(extract_document_id(""), None);

        // Malformed (chunk at start)
        assert_eq!(extract_document_id("-chunk-0"), None);
    }

    #[test]
    fn build_chunk_from_result_extracts_modality_metadata() {
        let result = VectorSearchResult {
            id: "chart-0".into(),
            score: 0.9,
            metadata: serde_json::json!({
                "type": "chunk",
                "modality": "chart",
                "content": "Q4: 42M"
            }),
        };
        let chunk = build_chunk_from_result(&result);
        assert_eq!(chunk.modality.as_deref(), Some("chart"));
    }

    #[test]
    fn an_image_chunk_is_marked_so_text_scoring_can_skip_it() {
        // SPEC-180: an image chunk stores its file name as `content`. Without the marker a
        // cross-encoder would read that as an irrelevant empty chunk and drop the picture.
        let result = VectorSearchResult {
            id: "img-0".into(),
            score: 0.9,
            metadata: serde_json::json!({
                "type": "chunk",
                "source_modality": "image",
                "content": "storefront.png"
            }),
        };
        let chunk = build_chunk_from_result(&result);
        assert!(chunk.is_image_chunk(), "image chunk must be identifiable");
        assert_eq!(chunk.source_modality.as_deref(), Some("image"));
    }

    #[test]
    fn a_text_chunk_is_not_marked_as_an_image() {
        let result = VectorSearchResult {
            id: "t-0".into(),
            score: 0.9,
            metadata: serde_json::json!({"type": "chunk", "content": "refunds within 30 days"}),
        };
        let chunk = build_chunk_from_result(&result);
        assert!(!chunk.is_image_chunk());
        assert!(chunk.source_modality.is_none());
    }

    #[test]
    fn test_extract_entity_source_tracking() {
        let mut props = HashMap::new();
        props.insert(
            "source_chunk_ids".to_string(),
            serde_json::json!(["chunk-1", "chunk-2"]),
        );
        props.insert(
            "source_document_id".to_string(),
            serde_json::json!("doc-123"),
        );
        props.insert(
            "source_file_path".to_string(),
            serde_json::json!("/path/to/file.pdf"),
        );

        let tracking = extract_entity_source_tracking(&props);

        assert_eq!(tracking.source_chunk_ids, vec!["chunk-1", "chunk-2"]);
        assert_eq!(tracking.source_document_id, Some("doc-123".to_string()));
        assert_eq!(
            tracking.source_file_path,
            Some("/path/to/file.pdf".to_string())
        );
    }

    #[test]
    fn test_extract_entity_source_tracking_empty() {
        let props = HashMap::new();
        let tracking = extract_entity_source_tracking(&props);

        assert!(tracking.source_chunk_ids.is_empty());
        assert!(tracking.source_document_id.is_none());
        assert!(tracking.source_file_path.is_none());
    }

    #[test]
    fn test_extract_relationship_source_tracking() {
        let mut props = HashMap::new();
        props.insert("source_chunk_id".to_string(), serde_json::json!("chunk-1"));
        props.insert(
            "source_document_id".to_string(),
            serde_json::json!("doc-123"),
        );

        let tracking = extract_relationship_source_tracking(&props);

        assert_eq!(tracking.source_chunk_id, Some("chunk-1".to_string()));
        assert_eq!(tracking.source_chunk_ids, vec!["chunk-1".to_string()]);
        assert_eq!(tracking.source_document_id, Some("doc-123".to_string()));
    }

    #[test]
    fn test_extract_relationship_source_tracking_plural_union() {
        let mut props = HashMap::new();
        props.insert(
            "source_chunk_ids".to_string(),
            serde_json::json!(["chunk-0", "chunk-1"]),
        );
        props.insert("source_chunk_id".to_string(), serde_json::json!("chunk-0"));

        let tracking = extract_relationship_source_tracking(&props);
        assert_eq!(
            tracking.source_chunk_ids,
            vec!["chunk-0".to_string(), "chunk-1".to_string()]
        );
        assert_eq!(tracking.source_chunk_id, Some("chunk-0".to_string()));
    }

    #[test]
    fn test_build_entity_from_node() {
        let mut props = HashMap::new();
        props.insert("entity_type".to_string(), serde_json::json!("PERSON"));
        props.insert(
            "description".to_string(),
            serde_json::json!("A famous person"),
        );
        props.insert(
            "source_chunk_ids".to_string(),
            serde_json::json!(["chunk-1"]),
        );

        let entity = build_entity_from_node("JOHN_DOE", &props, 5, 0.85);

        assert_eq!(entity.name, "JOHN_DOE");
        assert_eq!(entity.entity_type, "PERSON");
        assert_eq!(entity.description, "A famous person");
        assert_eq!(entity.degree, 5);
        assert_eq!(entity.score, 0.85);
        assert_eq!(entity.source_chunk_ids, vec!["chunk-1"]);
    }

    #[test]
    fn test_build_entity_from_node_soft_labels_opaque_uuid() {
        let mut props = HashMap::new();
        props.insert("entity_type".to_string(), serde_json::json!("CONCEPT"));
        props.insert(
            "description".to_string(),
            serde_json::json!("Future of work theme from the agenda"),
        );
        props.insert(
            "label".to_string(),
            serde_json::json!("84B69E27-E38B-444A-83DD-5E6A537C6F12"),
        );

        let entity = build_entity_from_node("84B69E27-E38B-444A-83DD-5E6A537C6F12", &props, 2, 0.5);
        assert!(
            entity.name.contains("Future of work"),
            "got {}",
            entity.name
        );
        assert!(!entity.name.contains("84B69E27"));
    }

    #[test]
    fn test_build_relationship_from_edge() {
        let mut props = HashMap::new();
        props.insert("relation_type".to_string(), serde_json::json!("WORKS_FOR"));
        props.insert(
            "description".to_string(),
            serde_json::json!("Employment relationship"),
        );
        props.insert(
            "source_chunk_ids".to_string(),
            serde_json::json!(["chunk-1", "chunk-7"]),
        );
        props.insert("source_chunk_id".to_string(), serde_json::json!("chunk-1"));

        let rel = build_relationship_from_edge("JOHN_DOE", "ACME_CORP", &props);

        assert_eq!(rel.source, "JOHN_DOE");
        assert_eq!(rel.target, "ACME_CORP");
        assert_eq!(rel.relation_type, "WORKS_FOR");
        assert_eq!(rel.source_chunk_id, Some("chunk-1".to_string()));
        assert_eq!(
            rel.all_source_chunk_ids(),
            vec!["chunk-1".to_string(), "chunk-7".to_string()]
        );
    }

    #[test]
    fn apply_relationship_endpoint_labels_soft_labels_opaque_uuids() {
        let opaque = "84B69E27-E38B-444A-83DD-5E6A537C6F12";
        let mut rel = RetrievedRelationship::new(opaque, "AI_NEXT_CONFERENCE", "HAS_THEME");
        let mut nodes = HashMap::new();
        let mut opaque_props = HashMap::new();
        opaque_props.insert("entity_type".to_string(), serde_json::json!("CONCEPT"));
        opaque_props.insert(
            "description".to_string(),
            serde_json::json!("Future of work theme from the agenda"),
        );
        opaque_props.insert("label".to_string(), serde_json::json!(opaque));
        nodes.insert(opaque.to_string(), opaque_props);
        let mut conf_props = HashMap::new();
        conf_props.insert("entity_type".to_string(), serde_json::json!("EVENT"));
        conf_props.insert("label".to_string(), serde_json::json!("AI_NEXT_CONFERENCE"));
        nodes.insert("AI_NEXT_CONFERENCE".to_string(), conf_props);

        apply_relationship_endpoint_labels(std::slice::from_mut(&mut rel), &nodes);
        assert_eq!(rel.source, opaque);
        assert!(
            rel.source_label.contains("Future of work"),
            "got {}",
            rel.source_label
        );
        assert!(!rel.source_label.contains("84B69E27"));
        assert_eq!(rel.target_label, "AI_NEXT_CONFERENCE");
        assert_eq!(rel.display_source(), rel.source_label.as_str());
    }

    #[test]
    fn endpoint_display_label_opaque_without_node_uses_badge() {
        let opaque = "84b69e27-e38b-444a-83dd-5e6a537c6f12";
        let label = endpoint_display_label(opaque, None);
        assert_eq!(label, "Opaque ID · Entity");
    }
}
