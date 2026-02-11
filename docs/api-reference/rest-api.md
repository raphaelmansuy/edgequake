# EdgeQuake REST API Reference

> **Version**: 0.1.0  
> **Base URL**: `http://localhost:8080/api/v1`  
> **OpenAPI**: Available at `/api-docs/openapi.json`

This reference documents all EdgeQuake REST API endpoints for document ingestion, knowledge graph queries, and chat interactions.

---

## Table of Contents

- [Authentication](#authentication)
- [Health & Diagnostics](#health--diagnostics)
- [Documents API](#documents-api)
- [Query API](#query-api)
- [Chat API](#chat-api)
- [Graph API](#graph-api)
- [Workspaces API](#workspaces-api)
- [Conversations API](#conversations-api)
- [Models & Settings](#models--settings)
- [Error Handling](#error-handling)
- [Rate Limiting](#rate-limiting)

---

## Authentication

EdgeQuake supports two authentication methods:

### API Key Authentication

Include your API key in the `X-API-Key` header:

```bash
curl -H "X-API-Key: your-api-key" \
     http://localhost:8080/api/v1/documents
```

### Bearer Token Authentication

Use `Authorization: Bearer` header:

```bash
curl -H "Authorization: Bearer your-api-key" \
     http://localhost:8080/api/v1/documents
```

### Multi-Tenant Headers

For multi-tenant deployments, include workspace context:

| Header           | Description                 | Required                         |
| ---------------- | --------------------------- | -------------------------------- |
| `X-Tenant-ID`    | Tenant identifier (UUID)    | Required for multi-tenant        |
| `X-Workspace-ID` | Workspace identifier (UUID) | Required for workspace isolation |

```bash
curl -H "X-API-Key: your-key" \
     -H "X-Tenant-ID: tenant-uuid" \
     -H "X-Workspace-ID: workspace-uuid" \
     http://localhost:8080/api/v1/documents
```

### Public Endpoints (No Auth Required)

- `GET /health`
- `GET /ready`
- `GET /live`
- `GET /swagger-ui/*`
- `GET /api-docs/*`

---

## Health & Diagnostics

### GET /health

Deep health check with component status for monitoring dashboards.

**Response**:

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "storage_mode": "postgres",
  "workspace_id": "default",
  "components": {
    "kv_storage": true,
    "vector_storage": true,
    "graph_storage": true,
    "llm_provider": true
  },
  "llm_provider_name": "ollama",
  "schema": {
    "latest_version": 20240115001,
    "migrations_applied": 12,
    "last_applied_at": "2024-01-15T10:30:00Z"
  }
}
```

### GET /ready

Kubernetes readiness probe. Returns 200 if service can accept traffic.

```bash
curl http://localhost:8080/ready
# Response: 200 OK
```

### GET /live

Kubernetes liveness probe. Returns 200 if process is alive.

```bash
curl http://localhost:8080/live
# Response: 200 OK
```

---

## Documents API

Document ingestion with automatic entity extraction and knowledge graph construction.

### POST /api/v1/documents

Upload document content as JSON text.

**Text Upload (JSON)**:

```bash
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: workspace-uuid" \
  -d '{
    "content": "Your document text here...",
    "title": "Document Title",
    "source": "manual_entry"
  }'
```

### POST /api/v1/documents/upload

Upload a file (PDF, TXT, MD, JSON) via multipart form data.

**File Upload (Multipart)**:

```bash
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -H "X-Workspace-ID: workspace-uuid" \
  -F "file=@document.pdf" \
  -F "title=My PDF Document"
```

**Supported File Types**:

| Extension | MIME Type        | Max Size |
| --------- | ---------------- | -------- |
| `.pdf`    | application/pdf  | 50 MB    |
| `.txt`    | text/plain       | 10 MB    |
| `.md`     | text/markdown    | 10 MB    |
| `.json`   | application/json | 10 MB    |

**Response** (Sync processing):

```json
{
  "id": "doc-uuid",
  "title": "Document Title",
  "status": "completed",
  "content_hash": "sha256:...",
  "chunk_count": 15,
  "entity_count": 23,
  "relationship_count": 18,
  "created_at": "2024-01-15T10:30:00Z",
  "processing_time_ms": 2340
}
```

**Response** (Async processing for large files):

```json
{
  "id": "doc-uuid",
  "title": "Large Document",
  "status": "processing",
  "task_id": "task-uuid",
  "message": "Document queued for processing"
}
```

### GET /api/v1/documents

List all documents in the workspace.

**Query Parameters**:

| Parameter | Type    | Default | Description                                      |
| --------- | ------- | ------- | ------------------------------------------------ |
| `limit`   | integer | 50      | Max documents to return                          |
| `offset`  | integer | 0       | Pagination offset                                |
| `status`  | string  | all     | Filter by status (processing, completed, failed) |

```bash
curl http://localhost:8080/api/v1/documents?limit=10&status=completed \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response**:

```json
{
  "documents": [
    {
      "id": "doc-uuid-1",
      "title": "Document 1",
      "status": "completed",
      "chunk_count": 15,
      "created_at": "2024-01-15T10:30:00Z"
    }
  ],
  "total": 42,
  "limit": 10,
  "offset": 0
}
```

### GET /api/v1/documents/:id

Get document details by ID.

```bash
curl http://localhost:8080/api/v1/documents/doc-uuid \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response**:

```json
{
  "id": "doc-uuid",
  "title": "Document Title",
  "status": "completed",
  "content_hash": "sha256:...",
  "chunk_count": 15,
  "entity_count": 23,
  "relationship_count": 18,
  "file_path": "/uploads/document.pdf",
  "file_size": 1024000,
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:32:40Z"
}
```

### DELETE /api/v1/documents/:id

Delete a document and all associated data (chunks, entities, relationships).

```bash
curl -X DELETE http://localhost:8080/api/v1/documents/doc-uuid \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response**: `204 No Content`

---

## Query API

Execute RAG queries with multi-mode retrieval.

### POST /api/v1/query

Execute a query with configurable retrieval mode.

**Request**:

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: workspace-uuid" \
  -d '{
    "query": "What are the main themes discussed?",
    "mode": "hybrid",
    "enable_rerank": true,
    "rerank_top_k": 5
  }'
```

**Request Body**:

| Field                  | Type    | Default  | Description                                  |
| ---------------------- | ------- | -------- | -------------------------------------------- |
| `query`                | string  | required | The question to answer                       |
| `mode`                 | string  | "hybrid" | Query mode (see below)                       |
| `context_only`         | boolean | false    | Return only retrieved context, no LLM answer |
| `prompt_only`          | boolean | false    | Return formatted prompt for debugging        |
| `enable_rerank`        | boolean | true     | Apply reranking to improve relevance         |
| `rerank_top_k`         | integer | 5        | Number of top chunks after reranking         |
| `conversation_history` | array   | null     | Previous messages for multi-turn context     |

**Query Modes**:

| Mode     | Description              | Use Case                          |
| -------- | ------------------------ | --------------------------------- |
| `naive`  | Vector search only       | Fast, simple queries              |
| `local`  | Entity-centric retrieval | Questions about specific entities |
| `global` | Community summaries      | Theme/overview questions          |
| `hybrid` | Local + Global (default) | General queries                   |
| `mix`    | Adaptive blending        | Complex queries                   |
| `bypass` | Direct LLM, no RAG       | When context not needed           |

**Response**:

```json
{
  "answer": "The main themes discussed include...",
  "mode": "hybrid",
  "sources": [
    {
      "source_type": "chunk",
      "id": "chunk-uuid",
      "score": 0.89,
      "rerank_score": 0.95,
      "snippet": "The first theme relates to...",
      "reference_id": 1,
      "document_id": "doc-uuid",
      "file_path": "document.pdf",
      "start_line": 45,
      "end_line": 52,
      "chunk_index": 3
    },
    {
      "source_type": "entity",
      "id": "CLIMATE_CHANGE",
      "score": 0.85,
      "snippet": "A global phenomenon affecting...",
      "reference_id": 2,
      "document_id": "doc-uuid"
    }
  ],
  "stats": {
    "embedding_time_ms": 45,
    "retrieval_time_ms": 123,
    "generation_time_ms": 890,
    "total_time_ms": 1058,
    "sources_retrieved": 8,
    "rerank_time_ms": 67,
    "tokens_used": 256,
    "tokens_per_second": 287.6,
    "llm_provider": "ollama",
    "llm_model": "gemma3:12b"
  },
  "reranked": true
}
```

### POST /api/v1/query/stream

Stream query response using Server-Sent Events (SSE).

**Request**:

```bash
curl -X POST http://localhost:8080/api/v1/query/stream \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -d '{"query": "Explain the key findings", "mode": "hybrid"}'
```

**SSE Events**:

```
event: start
data: {"mode":"hybrid","sources_count":5}

event: token
data: {"content":"The"}

event: token
data: {"content":" key"}

event: token
data: {"content":" findings"}

event: sources
data: [{"source_type":"chunk","id":"...","score":0.89}]

event: done
data: {"total_tokens":256,"total_time_ms":1200}
```

---

## Chat API

Unified chat completions API with OpenAI-compatible format.

### POST /api/v1/chat/completions

Execute a chat completion with automatic conversation management.

**Request**:

```bash
curl -X POST http://localhost:8080/api/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: workspace-uuid" \
  -d '{
    "message": "What is the relationship between X and Y?",
    "conversation_id": "conv-uuid",
    "mode": "hybrid",
    "stream": false
  }'
```

**Request Body**:

| Field             | Type    | Default  | Description                                    |
| ----------------- | ------- | -------- | ---------------------------------------------- |
| `message`         | string  | required | User message                                   |
| `conversation_id` | string  | null     | Existing conversation ID (creates new if null) |
| `mode`            | string  | "hybrid" | Query mode                                     |
| `stream`          | boolean | false    | Enable SSE streaming                           |

**Response** (Non-streaming):

```json
{
  "id": "msg-uuid",
  "conversation_id": "conv-uuid",
  "role": "assistant",
  "content": "The relationship between X and Y is...",
  "sources": [...],
  "stats": {...},
  "created_at": "2024-01-15T10:30:00Z"
}
```

**Streaming Response**:

```bash
curl -X POST http://localhost:8080/api/v1/chat/completions \
  -H "Accept: text/event-stream" \
  -d '{"message": "...", "stream": true}'
```

```
event: message_start
data: {"conversation_id":"conv-uuid","message_id":"msg-uuid"}

event: content_delta
data: {"delta":"The"}

event: content_delta
data: {"delta":" relationship"}

event: sources
data: [{"source_type":"entity","id":"X",...}]

event: message_end
data: {"finish_reason":"stop","tokens_used":128}
```

---

## Graph API

Knowledge graph exploration and visualization endpoints.

### GET /api/v1/graph

Get the knowledge graph with optional traversal.

**Query Parameters**:

| Parameter    | Type    | Default | Description                     |
| ------------ | ------- | ------- | ------------------------------- |
| `start_node` | string  | null    | Entity ID to center traversal   |
| `depth`      | integer | 2       | Max traversal hops              |
| `max_nodes`  | integer | 100     | Max nodes to return (max: 1000) |

```bash
curl "http://localhost:8080/api/v1/graph?start_node=ENTITY_NAME&depth=2&max_nodes=50" \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response**:

```json
{
  "nodes": [
    {
      "id": "ENTITY_NAME",
      "label": "Entity Name",
      "node_type": "PERSON",
      "description": "Description of the entity...",
      "degree": 5,
      "properties": {}
    }
  ],
  "edges": [
    {
      "source": "ENTITY_A",
      "target": "ENTITY_B",
      "edge_type": "WORKS_WITH",
      "weight": 1.0,
      "properties": {}
    }
  ],
  "total_nodes": 150,
  "total_edges": 200,
  "is_truncated": true
}
```

### GET /api/v1/graph/stats

Get graph statistics.

```bash
curl http://localhost:8080/api/v1/graph/stats \
  -H "X-Workspace-ID: workspace-uuid"
```

**Response**:

```json
{
  "total_nodes": 1500,
  "total_edges": 4200,
  "node_types": {
    "PERSON": 250,
    "ORGANIZATION": 180,
    "CONCEPT": 820,
    "LOCATION": 150,
    "EVENT": 100
  },
  "edge_types": {
    "RELATED_TO": 2100,
    "WORKS_WITH": 450,
    "LOCATED_IN": 320,
    "PART_OF": 580
  },
  "avg_degree": 2.8,
  "density": 0.0019
}
```

### GET /api/v1/graph/entities

List entities with pagination.

```bash
curl "http://localhost:8080/api/v1/graph/entities?limit=20&type=PERSON" \
  -H "X-Workspace-ID: workspace-uuid"
```

### GET /api/v1/graph/entities/:id

Get entity details by ID.

```bash
curl http://localhost:8080/api/v1/graph/entities/ENTITY_NAME \
  -H "X-Workspace-ID: workspace-uuid"
```

### GET /api/v1/graph/relationships

List relationships with pagination.

```bash
curl "http://localhost:8080/api/v1/graph/relationships?limit=20&type=WORKS_WITH" \
  -H "X-Workspace-ID: workspace-uuid"
```

### GET /api/v1/graph/stream

Stream graph updates via SSE (for real-time visualization).

```bash
curl http://localhost:8080/api/v1/graph/stream \
  -H "Accept: text/event-stream" \
  -H "X-Workspace-ID: workspace-uuid"
```

---

## Workspaces API

Manage workspaces for multi-tenant isolation.

### POST /api/v1/workspaces

Create a new workspace.

```bash
curl -X POST http://localhost:8080/api/v1/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Research Project",
    "description": "Workspace for research documents",
    "embedding_model": "text-embedding-3-small",
    "embedding_dimension": 1536,
    "llm_model": "gpt-4o-mini"
  }'
```

### GET /api/v1/workspaces

List all workspaces.

### GET /api/v1/workspaces/:id

Get workspace details.

### PATCH /api/v1/workspaces/:id

Update workspace settings.

### DELETE /api/v1/workspaces/:id

Delete a workspace and all its data.

---

## Conversations API

Manage chat conversations.

### GET /api/v1/conversations

List conversations.

### POST /api/v1/conversations

Create a new conversation.

### GET /api/v1/conversations/:id

Get conversation with messages.

### DELETE /api/v1/conversations/:id

Delete a conversation.

### GET /api/v1/conversations/:id/messages

Get messages in a conversation.

---

## Models & Settings

### GET /api/v1/models

List available LLM models.

```bash
curl http://localhost:8080/api/v1/models
```

**Response**:

```json
{
  "models": [
    {
      "id": "gpt-4o-mini",
      "name": "GPT-4o Mini",
      "provider": "openai",
      "context_length": 128000,
      "capabilities": ["chat", "embeddings"]
    },
    {
      "id": "gemma3:12b",
      "name": "Gemma 3 12B",
      "provider": "ollama",
      "context_length": 8192,
      "capabilities": ["chat"]
    }
  ]
}
```

### GET /api/v1/settings

Get current settings.

### PATCH /api/v1/settings

Update settings.

---

## Error Handling

EdgeQuake uses RFC 7807 Problem Details for error responses.

**Error Response Format**:

```json
{
  "type": "https://edgequake.dev/errors/not-found",
  "title": "Resource Not Found",
  "status": 404,
  "detail": "Document with ID 'doc-uuid' not found in workspace",
  "instance": "/api/v1/documents/doc-uuid"
}
```

**Common Error Codes**:

| Status | Type                  | Description                         |
| ------ | --------------------- | ----------------------------------- |
| 400    | `bad-request`         | Invalid request parameters          |
| 401    | `unauthorized`        | Missing or invalid authentication   |
| 403    | `forbidden`           | Access denied to resource           |
| 404    | `not-found`           | Resource not found                  |
| 409    | `conflict`            | Resource already exists (duplicate) |
| 413    | `payload-too-large`   | File exceeds size limit             |
| 422    | `validation-error`    | Request validation failed           |
| 429    | `rate-limited`        | Too many requests                   |
| 500    | `internal-error`      | Server error                        |
| 503    | `service-unavailable` | Dependency unavailable              |

---

## Rate Limiting

Rate limiting is applied per API key or IP address.

**Headers in Response**:

| Header                  | Description                         |
| ----------------------- | ----------------------------------- |
| `X-RateLimit-Limit`     | Max requests per window             |
| `X-RateLimit-Remaining` | Requests remaining                  |
| `X-RateLimit-Reset`     | Epoch timestamp when limit resets   |
| `Retry-After`           | Seconds to wait (when rate limited) |

**Default Limits**:

| Endpoint Category | Requests  | Window   |
| ----------------- | --------- | -------- |
| Document upload   | 10        | 1 minute |
| Query execution   | 60        | 1 minute |
| Graph traversal   | 100       | 1 minute |
| Health checks     | Unlimited | -        |

---

## Ollama Compatibility Layer

EdgeQuake provides Ollama-compatible endpoints for tool integration.

### POST /v1/embeddings

Generate embeddings (Ollama format).

```bash
curl -X POST http://localhost:8080/v1/embeddings \
  -H "Content-Type: application/json" \
  -d '{"model": "nomic-embed-text", "input": "Hello world"}'
```

### POST /v1/chat/completions

Chat completions (OpenAI format, Ollama compatible).

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemma3:12b",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ],
    "stream": false
  }'
```

---

## Request Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    API REQUEST PROCESSING                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  Client Request                                                  │
│       ↓                                                          │
│  ┌─────────────────┐                                            │
│  │  Rate Limiter   │ ─ 429 if exceeded                          │
│  └────────┬────────┘                                            │
│           ↓                                                      │
│  ┌─────────────────┐                                            │
│  │ Authentication  │ ─ 401 if invalid                           │
│  └────────┬────────┘                                            │
│           ↓                                                      │
│  ┌─────────────────┐                                            │
│  │ Tenant Context  │ ─ Extract X-Tenant-ID, X-Workspace-ID      │
│  └────────┬────────┘                                            │
│           ↓                                                      │
│  ┌─────────────────┐                                            │
│  │ Request Handler │ ─ Business logic                           │
│  └────────┬────────┘                                            │
│           ↓                                                      │
│  ┌─────────────────┐                                            │
│  │  Response       │ ─ JSON or SSE stream                       │
│  └─────────────────┘                                            │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## See Also

- [Quick Start Guide](../getting-started/quick-start.md) - Get running in 5 minutes
- [Query Modes](../deep-dives/lightrag-algorithm.md#query-modes) - Detailed mode comparison
- [Architecture Overview](../architecture/overview.md) - System design
