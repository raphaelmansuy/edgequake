//! SPEC-180: turn the kind predicates of a `DocumentFilter` into a normal-RAG plan.
//!
//! One validation path for query, context, chat and MCP (DRY). Words are checked
//! (`invalid_doc_kind`, `invalid_source_modality` → 400), and a graph workspace that
//! receives them gets 422 `kind_filter_not_supported_by_profile` instead of silently
//! ignoring a filter the caller thinks is applied.

use edgequake_core::rag_profile::{parse_source_modalities, DocKind};
use edgequake_core::Workspace;
use edgequake_query::normal::NormalQueryPlan;
use edgequake_storage::traits::ChunkKinds;

use crate::error::ApiResult;
use crate::handlers::query_types::DocumentFilter;

fn lowered(list: &[String]) -> Vec<String> {
    list.iter().map(|w| w.trim().to_ascii_lowercase()).collect()
}

/// Validated kinds of a filter, `None` when it sets none.
pub fn kinds_from_filter(filter: &DocumentFilter) -> ApiResult<Option<ChunkKinds>> {
    let doc_kinds = filter
        .doc_kinds
        .as_deref()
        .map(DocKind::parse_all)
        .transpose()?
        .map(|kinds| kinds.iter().map(|k| k.as_str().to_string()).collect());
    let source_modalities = filter
        .source_modalities
        .as_deref()
        .map(parse_source_modalities)
        .transpose()?;
    let mime_types = filter.mime_types.as_deref().map(lowered);
    let kinds = ChunkKinds {
        doc_kinds,
        mime_types,
        source_modalities,
    };
    Ok(Some(kinds).filter(|k| !k.is_empty()))
}

/// A plan carrying the filter's kinds, or `None` when the filter sets none.
/// Errors when the workspace profile cannot honour them.
pub fn plan_for_filter(
    workspace: Option<&Workspace>,
    filter: Option<&DocumentFilter>,
) -> ApiResult<Option<NormalQueryPlan>> {
    let Some(filter) = filter.filter(|f| f.has_kind_filters()) else {
        return Ok(None);
    };
    let caps = workspace.map(Workspace::capabilities).unwrap_or_default();
    caps.check_kind_filters(true)?;
    Ok(kinds_from_filter(filter)?.map(|kinds| NormalQueryPlan::text().with_kinds(Some(kinds))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(doc_kinds: &[&str], modalities: &[&str]) -> DocumentFilter {
        DocumentFilter {
            doc_kinds: Some(doc_kinds.iter().map(|s| s.to_string()).collect()),
            source_modalities: Some(modalities.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        }
    }

    #[test]
    fn words_are_normalised() {
        let kinds = kinds_from_filter(&filter(&["PDF", " Image "], &["TEXT"]))
            .unwrap()
            .unwrap();
        assert_eq!(kinds.doc_kinds, Some(vec!["pdf".into(), "image".into()]));
        assert_eq!(kinds.source_modalities, Some(vec!["text".into()]));
    }

    #[test]
    fn unknown_words_are_rejected_with_their_codes() {
        let err = kinds_from_filter(&filter(&["spreadsheet"], &[])).unwrap_err();
        assert!(err.to_string().contains("doc_kinds"));
        let err = kinds_from_filter(&filter(&[], &["audio"])).unwrap_err();
        assert!(err.to_string().contains("source_modalities"));
    }

    #[test]
    fn a_filter_without_kinds_yields_no_plan() {
        let plan = plan_for_filter(None, Some(&DocumentFilter::default())).unwrap();
        assert!(plan.is_none());
    }

    #[test]
    fn graph_workspace_refuses_kind_filters() {
        let err = plan_for_filter(None, Some(&filter(&["pdf"], &[]))).unwrap_err();
        assert!(err.to_string().contains("normal workspaces"));
    }
}
