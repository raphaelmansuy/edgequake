//! Shared vector metadata builders for entity and relationship upserts.

use serde_json::{json, Value};

use crate::extractor::{ExtractedEntity, ExtractedRelationship};

/// Optional tenant/workspace scope applied to all vector metadata.
pub struct TenantScope<'a> {
    pub tenant_id: &'a Option<String>,
    pub workspace_id: &'a Option<String>,
}

impl<'a> TenantScope<'a> {
    fn apply(&self, metadata: &mut Value) {
        if let Some(tenant_id) = self.tenant_id {
            metadata["tenant_id"] = json!(tenant_id);
        }
        if let Some(workspace_id) = self.workspace_id {
            metadata["workspace_id"] = json!(workspace_id);
        }
    }
}

/// Build entity vector metadata for Local query mode retrieval.
pub fn entity_vector_metadata(entity: &ExtractedEntity, scope: TenantScope<'_>) -> Value {
    let mut metadata = json!({
        "type": "entity",
        "entity_name": entity.name,
        "entity_type": entity.entity_type,
        "description": entity.description,
        "source_chunk_ids": entity.source_chunk_ids,
        "source_document_id": entity.source_document_id,
        "source_file_path": entity.source_file_path
    });
    scope.apply(&mut metadata);
    metadata
}

/// Build relationship vector metadata for Global query mode retrieval.
pub fn relationship_vector_metadata(
    rel: &ExtractedRelationship,
    source_key: &str,
    target_key: &str,
    scope: TenantScope<'_>,
) -> Value {
    let mut metadata = json!({
        "type": "relationship",
        "src_id": source_key,
        "tgt_id": target_key,
        "keywords": rel.keywords.join(", "),
        "relation_type": rel.relation_type,
        "description": rel.description,
        "source_chunk_id": rel.source_chunk_id,
        "source_document_id": rel.source_document_id,
        "source_file_path": rel.source_file_path
    });
    scope.apply(&mut metadata);
    metadata
}
