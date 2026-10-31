//! SPEC-180 W5a: what the MCP surface does on a `normal` workspace.
//!
//! One place answers three questions, so no tool branches on a profile name:
//! which tools need a graph, how a chunk-kind filter is read from tool arguments, and what
//! the agent is told about the workspace.

use edgequake_core::rag_profile::ProfileCapabilities;
use serde_json::{json, Value};

use super::errors::{eq_error, ErrorCode};
use crate::handlers::query_types::DocumentFilter;

/// Tools whose output is a graph. A workspace without one refuses them (409).
pub const GRAPH_TOOLS: [&str; 4] = [
    "eq_entity_search",
    "eq_entity_get",
    "eq_neighborhood",
    "eq_graph_image",
];

/// The refusal envelope when `tool` needs a graph the workspace lacks.
pub fn graph_tool_refusal(tool: &str, caps: &ProfileCapabilities) -> Option<Value> {
    if caps.queries_graph || !GRAPH_TOOLS.contains(&tool) {
        return None;
    }
    Some(eq_error(
        ErrorCode::ProfileHasNoGraph,
        "This workspace is a normal RAG workspace: it has chunks and embeddings but no \
         entities or graph. Use eq_search and eq_fetch(view=chunks).",
        None,
    ))
}

fn strings(source: Option<&Value>, key: &str) -> Option<Vec<String>> {
    let items: Vec<String> = source?
        .get(key)?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    (!items.is_empty()).then_some(items)
}

/// A kind filter field: the top-level argument wins over the nested `document_filter`.
fn kind_field(args: &Value, key: &str) -> Option<Vec<String>> {
    strings(Some(args), key).or_else(|| strings(args.get("document_filter"), key))
}

/// `doc_kinds`, `mime_types` and `source_modalities` from tool arguments, as a filter that
/// carries only those three fields.
pub fn kind_filter_from_args(args: &Value) -> Option<DocumentFilter> {
    let filter = DocumentFilter {
        doc_kinds: kind_field(args, "doc_kinds"),
        mime_types: kind_field(args, "mime_types"),
        source_modalities: kind_field(args, "source_modalities"),
        ..Default::default()
    };
    filter.has_kind_filters().then_some(filter)
}

/// Add the kind fields found in `args` to `filter`, creating it when needed.
pub fn with_kind_filters(filter: Option<DocumentFilter>, args: &Value) -> Option<DocumentFilter> {
    let Some(kinds) = kind_filter_from_args(args) else {
        return filter;
    };
    let mut merged = filter.unwrap_or_default();
    merged.doc_kinds = kinds.doc_kinds;
    merged.mime_types = kinds.mime_types;
    merged.source_modalities = kinds.source_modalities;
    Some(merged)
}

/// Shown to the agent when the workspace is a normal one.
pub const NORMAL_NOTE: &str = "This workspace is a normal RAG workspace: a chunk and embedding \
index with no knowledge graph. Entity tools (eq_entity_search, eq_entity_get, eq_neighborhood, \
eq_graph_image) return profile_has_no_graph; do not invent entities or relationships. Search \
returns chunks only. Narrow by doc_kinds, mime_types or source_modalities (text, image) when \
the user asks for a kind of source.";

/// Instructions for a connection: the profile text, plus the normal-workspace note.
pub fn instructions_with_note(base: &str, caps: &ProfileCapabilities) -> String {
    if caps.queries_graph {
        base.to_string()
    } else {
        format!("{base}\n9. {NORMAL_NOTE}")
    }
}

/// JSON-schema properties for the three kind filters, shared by every search tool.
pub fn kind_filter_schema() -> Value {
    json!({
        "doc_kinds": {
            "type": "array",
            "items": { "type": "string", "enum": ["pdf", "image", "markdown", "text", "office", "other"] },
            "description": "Normal workspaces only: keep chunks of these document kinds."
        },
        "mime_types": {
            "type": "array",
            "items": { "type": "string" },
            "description": "Normal workspaces only: keep chunks of documents with these MIME types."
        },
        "source_modalities": {
            "type": "array",
            "items": { "type": "string", "enum": ["text", "image"] },
            "description": "Normal workspaces only: keep text chunks or image chunks."
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_core::rag_profile::{codes, RagProfile};

    fn normal() -> ProfileCapabilities {
        ProfileCapabilities::new(RagProfile::Normal, Default::default())
    }

    #[test]
    fn graph_tools_are_refused_on_normal_only() {
        for tool in GRAPH_TOOLS {
            let refusal = graph_tool_refusal(tool, &normal()).expect(tool);
            assert_eq!(refusal["error"]["code"], codes::PROFILE_HAS_NO_GRAPH);
            assert!(graph_tool_refusal(tool, &ProfileCapabilities::default()).is_none());
        }
        assert!(graph_tool_refusal("eq_search", &normal()).is_none());
    }

    #[test]
    fn kind_filters_read_top_level_then_nested() {
        let args = json!({
            "doc_kinds": ["pdf", " "],
            "document_filter": {"doc_kinds": ["image"], "source_modalities": ["image"]},
        });
        let filter = kind_filter_from_args(&args).expect("filter");
        assert_eq!(filter.doc_kinds, Some(vec!["pdf".to_string()]));
        assert_eq!(filter.source_modalities, Some(vec!["image".to_string()]));
        assert!(filter.mime_types.is_none());
    }

    #[test]
    fn no_kind_arguments_leave_the_filter_alone() {
        assert!(kind_filter_from_args(&json!({"query": "x"})).is_none());
        assert!(with_kind_filters(None, &json!({"doc_kinds": []})).is_none());
        let ids = DocumentFilter {
            document_ids: Some(vec!["a".into()]),
            ..Default::default()
        };
        let merged = with_kind_filters(Some(ids), &json!({"mime_types": ["image/png"]})).unwrap();
        assert_eq!(merged.document_ids, Some(vec!["a".to_string()]));
        assert_eq!(merged.mime_types, Some(vec!["image/png".to_string()]));
    }

    #[test]
    fn the_note_is_added_only_for_normal() {
        assert_eq!(
            instructions_with_note("base", &ProfileCapabilities::default()),
            "base"
        );
        assert!(instructions_with_note("base", &normal()).contains("profile_has_no_graph"));
    }
}
