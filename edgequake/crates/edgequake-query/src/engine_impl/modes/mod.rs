//! Per-mode vector query implementations (SPEC-024 2.8).

mod chunk_retrieval;
mod global;
mod hybrid;
mod local;
mod mix;
mod naive;

use edgequake_storage::traits::MetadataFilter;

/// Build a [`MetadataFilter`] for a mode query, optionally scoped to specific
/// document IDs (SPEC-031: pre-filter at SQL layer before results return).
///
/// Passing `allowed_document_ids` here pushes scope filtering to the SQL layer
/// (Tier 1) instead of relying solely on the post-retrieval `context_filter`.
/// This reduces data transferred from the DB and eliminates leniency gaps.
///
/// @implements SPEC-031: Vector pre-filter for entity/relationship scope
pub(super) fn make_scope_metadata_filter(
    tenant_id: Option<String>,
    workspace_id: Option<String>,
    allowed_document_ids: Option<&[String]>,
    vector_type: Option<&str>,
) -> Option<MetadataFilter> {
    let doc_ids = allowed_document_ids
        .filter(|ids| !ids.is_empty())
        .map(|ids| ids.iter().map(String::from).collect::<Vec<_>>());

    let has_any =
        tenant_id.is_some() || workspace_id.is_some() || doc_ids.is_some() || vector_type.is_some();

    if !has_any {
        return None;
    }

    Some(MetadataFilter {
        tenant_id,
        workspace_id,
        document_ids: doc_ids,
        vector_type: vector_type.map(str::to_string),
    })
}
