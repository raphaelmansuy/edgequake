# SPEC-031 / 005 — MCP Integration

> **Lens**: Platform Engineer / MCP Consumer  
> **Cross-refs**: [003-api-backend-spec.md](003-api-backend-spec.md), MCP tool definitions

---

## 1. Context

EdgeQuake exposes a Model Context Protocol (MCP) server that enables AI assistants and agents to query the knowledge graph programmatically. The `query` tool currently accepts `document_filter` with `date_from`, `date_to`, and `document_pattern`. SPEC-031 adds `document_ids` to this tool.

---

## 2. Current MCP `query` Tool Schema

```json
{
  "name": "query",
  "description": "Query the EdgeQuake knowledge graph with optional document filters.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": { "type": "string", "description": "Natural language query" },
      "mode": { "type": "string", "enum": ["naive","local","global","hybrid","mix"] },
      "document_filter": {
        "type": "object",
        "properties": {
          "date_from": { "type": "string", "format": "date-time" },
          "date_to": { "type": "string", "format": "date-time" },
          "document_pattern": { "type": "string" }
        }
      }
    },
    "required": ["query"]
  }
}
```

---

## 3. Updated MCP `query` Tool Schema

```json
{
  "name": "query",
  "description": "Query the EdgeQuake knowledge graph. Optionally restrict to specific documents via document_filter.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "Natural language question to answer from the knowledge graph."
      },
      "mode": {
        "type": "string",
        "enum": ["naive", "local", "global", "hybrid", "mix"],
        "description": "Retrieval mode. Default: hybrid."
      },
      "document_filter": {
        "type": "object",
        "description": "Optional filter to restrict RAG context to a subset of documents.",
        "properties": {
          "date_from": {
            "type": "string",
            "format": "date-time",
            "description": "Include only documents created on or after this date (ISO 8601)."
          },
          "date_to": {
            "type": "string",
            "format": "date-time",
            "description": "Include only documents created on or before this date (ISO 8601)."
          },
          "document_pattern": {
            "type": "string",
            "description": "Case-insensitive title substring. Comma-separated values are OR conditions."
          },
          "document_ids": {
            "type": "array",
            "items": { "type": "string", "format": "uuid" },
            "description": "Explicit document UUIDs to restrict query scope. Takes priority: skips KV scan when only IDs are specified. Empty array is treated as null (no filtering).",
            "maxItems": 100
          }
        }
      }
    },
    "required": ["query"]
  }
}
```

---

## 4. New `search_documents` MCP Tool

To help MCP clients discover document IDs before querying, expose the search endpoint as a tool:

```json
{
  "name": "search_documents",
  "description": "Search for documents in the current workspace by title. Returns minimal projections (id, title, status) suitable for use as document_ids in the query tool.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "q": {
        "type": "string",
        "description": "Title search query (case-insensitive substring). Omit or leave empty to list recent documents."
      },
      "page_size": {
        "type": "integer",
        "minimum": 1,
        "maximum": 50,
        "default": 20,
        "description": "Maximum number of results."
      },
      "status": {
        "type": "string",
        "enum": ["completed", "all"],
        "default": "completed",
        "description": "Filter by document status. Use 'completed' to only show queryable documents."
      }
    }
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "items": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "id": { "type": "string", "format": "uuid" },
            "title": { "type": "string" },
            "status": { "type": "string" },
            "created_at": { "type": "string", "format": "date-time" }
          }
        }
      },
      "total": { "type": "integer" },
      "has_more": { "type": "boolean" }
    }
  }
}
```

---

## 5. Typical MCP Agent Workflow

### 5.1 Two-Step: Search Then Query

```
1. Agent calls: search_documents({ q: "quarterly report" })
   -> returns: [
        { id: "doc-abc", title: "Q1 2025 Financial Report", status: "completed" },
        { id: "doc-def", title: "Q2 2025 Financial Report", status: "completed" }
      ]

2. Agent calls: query({
     query: "What were the revenue trends across quarters?",
     mode: "hybrid",
     document_filter: { document_ids: ["doc-abc", "doc-def"] }
   })
   -> returns: { answer: "...", context: {...} }
```

This pattern enables MCP agents to reason about which documents to query before executing, rather than blindly querying all documents.

### 5.2 One-Step: Pattern-Based (SPEC-005 legacy, unchanged)

```
Agent calls: query({
  query: "What are the Q1 revenue figures?",
  document_filter: { document_pattern: "Q1,2025" }
})
```

Both patterns remain valid. `document_ids` is more deterministic; `document_pattern` is more convenient for automated agents.

---

## 6. MCP Server Registration

The MCP server tool registry must be updated to add `search_documents`:

```rust
// In mcp/src/tools.rs (or equivalent):

Tool {
    name: "search_documents",
    description: "Search for documents in the workspace by title...",
    input_schema: json!({
        "type": "object",
        "properties": {
            "q": { "type": "string" },
            "page_size": { "type": "integer", "default": 20, "maximum": 50 },
            "status": { "type": "string", "enum": ["completed", "all"], "default": "completed" }
        }
    }),
    handler: |args, ctx| {
        // Route to GET /api/v1/documents/search with tenant context
        search_documents_handler(args, ctx).await
    }
}
```

---

## 7. Backward Compatibility

- Existing MCP `query` calls without `document_filter` work unchanged
- Existing MCP `query` calls with `document_filter` (date/pattern only) work unchanged
- `document_ids: []` (empty array) is a no-op — same behavior as not setting the field
- `search_documents` is additive — no existing tools are modified
