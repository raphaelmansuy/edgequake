# EdgeQuake MCP Server — Specification

## Purpose

Expose EdgeQuake's Graph-RAG capabilities as an MCP (Model Context Protocol) server so that any MCP-compatible agent can use EdgeQuake as persistent, queryable memory. Agents can create workspaces, ingest documents, query the knowledge graph, and explore entities — all through standard MCP tools.

## Package

- **npm**: `@edgequake/mcp-server`
- **Binary name**: `edgequake-mcp`
- **Transport**: stdio (standard MCP transport)
- **Runtime**: Node.js >= 18
- **Dependency**: `edgequake-sdk` (official TypeScript SDK)

## Configuration

Environment variables consumed by the MCP server:

| Variable                      | Required | Default                 | Description                                      |
| ----------------------------- | -------- | ----------------------- | ------------------------------------------------ |
| `EDGEQUAKE_BASE_URL`          | No       | `http://localhost:8080` | EdgeQuake API base URL                           |
| `EDGEQUAKE_API_KEY`           | No       | —                       | API key for authentication                       |
| `EDGEQUAKE_DEFAULT_TENANT`    | No       | —                       | Default tenant ID (auto-detected if only one)    |
| `EDGEQUAKE_DEFAULT_WORKSPACE` | No       | —                       | Default workspace ID (auto-detected if only one) |

## MCP Tools

### 1. Workspace Management

#### `workspace_list`

List all workspaces in the current tenant.

**Parameters**: none

**Returns**: Array of `{ id, name, slug, description, document_count, entity_count }`

---

#### `workspace_create`

Create a new workspace.

**Parameters**:
| Name | Type | Required | Description |
|---|---|---|---|
| `name` | string | yes | Workspace name |
| `description` | string | no | Description |
| `llm_model` | string | no | LLM model for extraction (e.g. "gemma3:12b") |
| `llm_provider` | string | no | LLM provider ("ollama", "openai") |
| `embedding_model` | string | no | Embedding model |
| `embedding_provider` | string | no | Embedding provider |

**Returns**: `{ id, name, slug }`

---

#### `workspace_get`

Get workspace details including statistics.

**Parameters**:
| Name | Type | Required | Description |
|---|---|---|---|
| `workspace_id` | string | yes | Workspace UUID |

**Returns**: `{ id, name, slug, description, document_count, entity_count, relationship_count }`

---

#### `workspace_delete`

Delete a workspace and all its data.

**Parameters**:
| Name | Type | Required | Description |
|---|---|---|---|
| `workspace_id` | string | yes | Workspace UUID |

**Returns**: `{ success: true }`

---

#### `workspace_stats`

Get statistics for a workspace.

**Parameters**:
| Name | Type | Required | Description |
|---|---|---|---|
| `workspace_id` | string | yes | Workspace UUID |

**Returns**: `{ document_count, entity_count, relationship_count, chunk_count }`

---

### 2. Document Management

#### `document_upload`

Upload a text document to the knowledge graph pipeline.

**Parameters**:
| Name | Type | Required | Description |
|---|---|---|---|
| `content` | string | yes | Document text content |
| `title` | string | no | Document title |
| `metadata` | object | no | Custom metadata key-value pairs |
| `enable_gleaning` | boolean | no | Enable multi-pass extraction (default: true) |

**Returns**: `{ document_id, status, task_id, chunk_count, entity_count, relationship_count }`

---

#### `document_list`

List documents with pagination and filtering.

**Parameters**:
| Name | Type | Required | Description |
|---|---|---|---|
| `page` | number | no | Page number (default: 1) |
| `page_size` | number | no | Items per page (default: 20) |
| `status` | string | no | Filter: "pending", "processing", "completed", "failed" |
| `search` | string | no | Full-text search in title/content |

**Returns**: `{ documents: [...], total, page, page_size, has_more }`

---

#### `document_get`

Get document details including full content.

**Parameters**:
| Name | Type | Required | Description |
|---|---|---|---|
| `document_id` | string | yes | Document UUID |

**Returns**: `{ id, title, content, status, chunk_count, entity_count, metadata }`

---

#### `document_delete`

Delete a document and its extracted knowledge.

**Parameters**:
| Name | Type | Required | Description |
|---|---|---|---|
| `document_id` | string | yes | Document UUID |

**Returns**: `{ success: true }`

---

#### `document_status`

Check the processing status of a document.

**Parameters**:
| Name | Type | Required | Description |
|---|---|---|---|
| `document_id` | string | yes | Document UUID |

**Returns**: `{ id, status, current_stage, stage_progress, error_message }`

---

### 3. Query & Knowledge Retrieval

#### `query`

Execute a RAG query against the knowledge graph. This is the primary tool for agents to retrieve knowledge.

**Parameters**:
| Name | Type | Required | Description |
|---|---|---|---|
| `query` | string | yes | Natural language question |
| `mode` | string | no | "naive", "local", "global", "hybrid" (default), "mix" |
| `max_results` | number | no | Max source references to return |
| `include_references` | boolean | no | Include source snippets (default: true) |
| `conversation_history` | array | no | Prior messages for multi-turn context |

**Returns**: `{ answer, mode, sources: [{ source_type, snippet, score }], stats: { total_time_ms } }`

---

### 4. Knowledge Graph Exploration

#### `graph_search_entities`

Search for entities in the knowledge graph.

**Parameters**:
| Name | Type | Required | Description |
|---|---|---|---|
| `search` | string | no | Search term |
| `label` | string | no | Filter by entity type (PERSON, ORG, TECH, etc.) |
| `limit` | number | no | Max results (default: 20) |

**Returns**: Array of `{ name, label, description }`

---

#### `graph_get_entity`

Get detailed information about a specific entity.

**Parameters**:
| Name | Type | Required | Description |
|---|---|---|---|
| `entity_name` | string | yes | Entity name (e.g. "RUST") |

**Returns**: `{ name, label, description, properties, source_documents }`

---

#### `graph_entity_neighborhood`

Get an entity's neighborhood — connected entities and relationships.

**Parameters**:
| Name | Type | Required | Description |
|---|---|---|---|
| `entity_name` | string | yes | Entity name |

**Returns**: `{ center: {...}, neighbors: [{ entity, relationship, direction }] }`

---

#### `graph_search_relationships`

Search relationships between entities.

**Parameters**:
| Name | Type | Required | Description |
|---|---|---|---|
| `source` | string | no | Source entity name |
| `target` | string | no | Target entity name |
| `label` | string | no | Relationship type |
| `limit` | number | no | Max results (default: 20) |

**Returns**: Array of `{ source, target, label, description, weight }`

---

### 5. Health & Status

#### `health`

Check EdgeQuake server health.

**Parameters**: none

**Returns**: `{ status, version, storage_mode, llm_provider_name, components }`

---

## MCP Resources

The server exposes the following MCP resources for context injection:

### `edgequake://workspace/{workspace_id}/stats`

Workspace statistics as a text summary. Agents can read this to understand the current state of the knowledge base.

### `edgequake://workspace/{workspace_id}/entities`

List of all entities in the workspace. Useful for agents to discover what knowledge is available.

## MCP Prompts

### `rag_query`

A prompt template for constructing effective RAG queries.

**Arguments**:

- `topic`: The subject to query about
- `mode`: Query mode (optional, default "hybrid")

### `document_summary`

A prompt template for summarizing a document after upload.

**Arguments**:

- `document_id`: The document to summarize

## Error Handling

All tools return structured errors with:

- `code`: HTTP status code or MCP error code
- `message`: Human-readable error description

Common error codes:

- `401`: Authentication required — set `EDGEQUAKE_API_KEY`
- `404`: Resource not found
- `422`: Validation error (missing required params)
- `503`: EdgeQuake server unavailable

## Usage in claude_desktop_config.json

```json
{
  "mcpServers": {
    "edgequake": {
      "command": "npx",
      "args": ["-y", "@edgequake/mcp-server"],
      "env": {
        "EDGEQUAKE_BASE_URL": "http://localhost:8080",
        "EDGEQUAKE_API_KEY": "eq-key-xxx"
      }
    }
  }
}
```

## Architecture

```
mcp/
  src/
    index.ts          # Entry point — creates McpServer, registers tools/resources
    server.ts         # McpServer setup, stdio transport binding
    tools/
      workspace.ts    # workspace_* tool handlers
      document.ts     # document_* tool handlers
      query.ts        # query tool handler
      graph.ts        # graph_* tool handlers
      health.ts       # health tool handler
    resources/
      workspace.ts    # edgequake:// resource handlers
    prompts/
      rag.ts          # Prompt templates
    config.ts         # Configuration from env vars
    client.ts         # EdgeQuake SDK client factory
  tests/
    e2e/
      workspace.test.ts
      document.test.ts
      query.test.ts
      graph.test.ts
  package.json
  tsconfig.json
  tsup.config.ts
```
