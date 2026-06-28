//! MCP tool catalog SSOT (mirrors specs/028/.../mcp/tool-schemas.json).

use serde_json::{json, Value};

pub const TOOLS_LIST_TTL_MS: u64 = 3_600_000;

/// Build tools/list result with caching metadata (SEP-2549).
pub fn tools_list_result() -> Value {
    json!({
        "tools": [
            edgequake_search_tool(),
            edgequake_fetch_tool(),
            edgequake_retrieve_tool(),
        ],
        "ttlMs": TOOLS_LIST_TTL_MS,
        "cacheScope": "public"
    })
}

fn edgequake_search_tool() -> Value {
    json!({
        "name": "edgequake_search",
        "description": "Search EdgeQuake knowledge graph and documents. Returns retrieval_id for fetch.",
        "inputSchema": {
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": { "type": "string", "description": "Natural language search query" },
                "mode": {
                    "type": "string",
                    "enum": ["naive", "local", "global", "hybrid", "mix"],
                    "default": "mix"
                },
                "max_results": { "type": "integer", "minimum": 1, "maximum": 50, "default": 5 },
                "workspace_id": {
                    "type": "string",
                    "description": "Target workspace (multi-tenant)",
                    "x-mcp-header": "Workspace-Id"
                },
                "document_filter": {
                    "type": "object",
                    "properties": {
                        "date_from": { "type": "string", "format": "date-time" },
                        "date_to": { "type": "string", "format": "date-time" },
                        "document_pattern": { "type": "string" }
                    }
                }
            }
        },
        "outputSchema": {
            "type": "object",
            "required": ["results"],
            "properties": {
                "results": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["retrieval_id", "title", "snippet", "url", "score"],
                        "properties": {
                            "retrieval_id": { "type": "string", "pattern": "^ret_" },
                            "title": { "type": "string" },
                            "snippet": { "type": "string" },
                            "url": { "type": "string" },
                            "score": { "type": "number" },
                            "metadata": {
                                "type": "object",
                                "description": "Graph preview: entity/relationship counts and top matches",
                                "properties": {
                                    "entity_count": { "type": "integer" },
                                    "relationship_count": { "type": "integer" },
                                    "top_entities": { "type": "array" },
                                    "top_relationships": { "type": "array" }
                                }
                            }
                        }
                    }
                }
            }
        },
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false
        }
    })
}

fn edgequake_fetch_tool() -> Value {
    json!({
        "name": "edgequake_fetch",
        "description": "Fetch full ContextBundle (chunks + subgraph entities/relationships + documents) for a retrieval_id from edgequake_search.",
        "inputSchema": {
            "type": "object",
            "required": ["retrieval_id"],
            "properties": {
                "retrieval_id": { "type": "string", "pattern": "^ret_" },
                "content_granularity": {
                    "type": "string",
                    "enum": ["citation", "agent", "debug"],
                    "default": "agent"
                },
                "include_subgraph": {
                    "type": "boolean",
                    "default": true,
                    "description": "Include bundle.subgraph (entities + relationships). Set false for chunks-only payload."
                }
            }
        },
        "outputSchema": {
            "type": "object",
            "required": ["retrieval_id", "bundle"],
            "properties": {
                "retrieval_id": { "type": "string" },
                "bundle": {
                    "type": "object",
                    "properties": {
                        "subgraph": {
                            "type": "object",
                            "properties": {
                                "entities": { "type": "array" },
                                "relationships": { "type": "array" }
                            }
                        },
                        "chunks": { "type": "array" },
                        "documents": { "type": "array" }
                    }
                }
            }
        },
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false
        }
    })
}

fn edgequake_retrieve_tool() -> Value {
    json!({
        "name": "edgequake_retrieve",
        "description": "One-shot full context retrieval without search/fetch split.",
        "inputSchema": {
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": { "type": "string" },
                "mode": { "type": "string", "enum": ["naive", "local", "global", "hybrid", "mix"] },
                "content_granularity": { "type": "string", "enum": ["citation", "agent", "debug"], "default": "agent" },
                "include_subgraph": { "type": "boolean", "default": true },
                "max_results": { "type": "integer" },
                "workspace_id": { "type": "string", "x-mcp-header": "Workspace-Id" },
                "enable_rerank": { "type": "boolean", "default": true }
            }
        },
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_list_has_cache_metadata() {
        let list = tools_list_result();
        assert_eq!(list["ttlMs"], TOOLS_LIST_TTL_MS);
        assert_eq!(list["cacheScope"], "public");
        assert!(list["tools"].as_array().unwrap().len() >= 3);
    }
}
