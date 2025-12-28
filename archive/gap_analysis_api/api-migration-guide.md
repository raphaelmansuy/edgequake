# API Migration Guide: LightRAG Python → EdgeQuake Rust

**Generated:** 2024-12-24  
**Source API:** LightRAG Python FastAPI  
**Target API:** EdgeQuake Rust Axum  
**Migration Impact:** Breaking changes requiring adapter layer

---

## Executive Summary

The EdgeQuake Rust API represents a complete rewrite with:

- RESTful URL structure with `/api/v1` prefix
- Consistent response schemas with pagination
- Task-based asynchronous processing
- Enhanced error handling with detailed error types

### Key Changes

1. All endpoints prefixed with `/api/v1/` (except health and Ollama emulation)
2. Knowledge Bases renamed to Workspaces
3. Pipeline status replaced by Task API
4. Document operations use RESTful resource paths
5. Entity/Relationship editing uses PUT method

---

## Endpoint Migration Reference

### Health & Monitoring

| Old Endpoint | New Endpoint | Method | Changes                    |
| ------------ | ------------ | ------ | -------------------------- |
| `/health`    | `/health`    | GET    | Same                       |
| N/A          | `/ready`     | GET    | New - Kubernetes readiness |
| N/A          | `/live`      | GET    | New - Kubernetes liveness  |
| N/A          | `/metrics`   | GET    | New - Prometheus metrics   |

**Migration:** No changes needed for health check.

---

### Authentication

| Old Endpoint       | New Endpoint                | Method | Changes                    |
| ------------------ | --------------------------- | ------ | -------------------------- |
| `POST /login`      | `POST /api/v1/auth/login`   | POST   | URL prefix                 |
| `GET /auth-status` | `GET /api/v1/auth/me`       | GET    | Renamed, returns user info |
| N/A                | `POST /api/v1/auth/refresh` | POST   | New - Token refresh        |
| N/A                | `POST /api/v1/auth/logout`  | POST   | New - Explicit logout      |

#### Login Request

**Old Format:**

```typescript
// multipart/form-data
FormData {
  username: string;
  password: string;
}
```

**New Format:**

```typescript
// application/json
interface LoginRequest {
  username: string;
  password: string;
}
```

#### Login Response

**Old Format:**

```typescript
interface LoginResponse {
  access_token: string;
  token_type: string;
  auth_mode?: "enabled" | "disabled";
  message?: string;
  core_version?: string;
  api_version?: string;
}
```

**New Format:**

```typescript
interface LoginResponse {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
  user: {
    id: string;
    username: string;
    email?: string;
    roles: string[];
  };
}
```

#### Adapter Pattern

```typescript
// lib/api/adapters/auth.adapter.ts

interface LegacyLoginResponse {
  access_token: string;
  token_type: string;
  auth_mode?: string;
}

export function adaptLoginResponse(
  newResponse: LoginResponse
): LegacyLoginResponse {
  return {
    access_token: newResponse.access_token,
    token_type: newResponse.token_type,
    auth_mode: "enabled",
  };
}
```

---

### Tenants & Workspaces

| Old Endpoint                   | New Endpoint                           | Method | Changes     |
| ------------------------------ | -------------------------------------- | ------ | ----------- |
| `GET /api/v1/tenants`          | `GET /api/v1/tenants`                  | GET    | Same        |
| `POST /api/v1/tenants`         | `POST /api/v1/tenants`                 | POST   | Same        |
| `GET /api/v1/tenants/me`       | `GET /api/v1/tenants/{id}`             | GET    | Path-based  |
| `GET /api/v1/knowledge-bases`  | `GET /api/v1/tenants/{id}/workspaces`  | GET    | **Renamed** |
| `POST /api/v1/knowledge-bases` | `POST /api/v1/tenants/{id}/workspaces` | POST   | **Renamed** |

#### Knowledge Base → Workspace Mapping

**Old Type:**

```typescript
interface KnowledgeBase {
  kb_id: string;
  tenant_id: string;
  kb_name: string;
  description?: string;
  num_documents: number;
  num_entities: number;
  num_relations: number;
}
```

**New Type:**

```typescript
interface Workspace {
  id: string; // Was kb_id
  tenant_id: string;
  name: string; // Was kb_name
  description?: string;
  document_count: number; // Was num_documents
  entity_count: number; // Was num_entities
  created_at: string;
}
```

#### Adapter Pattern

```typescript
// lib/api/adapters/workspace.adapter.ts

export function adaptWorkspaceToKB(workspace: Workspace): KnowledgeBase {
  return {
    kb_id: workspace.id,
    tenant_id: workspace.tenant_id,
    kb_name: workspace.name,
    description: workspace.description,
    num_documents: workspace.document_count,
    num_entities: workspace.entity_count,
    num_relations: 0, // Not available in new API
  };
}
```

---

### Documents

| Old Endpoint                        | New Endpoint                       | Method | Changes               |
| ----------------------------------- | ---------------------------------- | ------ | --------------------- |
| `GET /documents`                    | `GET /api/v1/documents`            | GET    | Pagination changed    |
| `POST /documents/text`              | `POST /api/v1/documents`           | POST   | Body format           |
| `POST /documents/texts`             | `POST /api/v1/documents`           | POST   | Loop or batch         |
| `POST /documents/upload`            | `POST /api/v1/documents/upload`    | POST   | Response changed      |
| `POST /documents/paginated`         | `GET /api/v1/documents?page=X`     | GET    | Query params          |
| `GET /documents/status_counts`      | `GET /api/v1/documents`            | GET    | Included in list      |
| `DELETE /documents`                 | `DELETE /api/v1/documents`         | DELETE | Same                  |
| `DELETE /documents/delete_document` | `DELETE /api/v1/documents/{id}`    | DELETE | RESTful path          |
| `POST /documents/scan`              | `POST /api/v1/documents/scan`      | POST   | Same                  |
| `POST /documents/reprocess_failed`  | `POST /api/v1/documents/reprocess` | POST   | Renamed               |
| `GET /documents/track_status/{id}`  | `GET /api/v1/documents/track/{id}` | GET    | Path changed          |
| `GET /documents/pipeline_status`    | `GET /api/v1/tasks`                | GET    | **Replaced by Tasks** |
| `POST /documents/cancel_pipeline`   | `POST /api/v1/pipeline/cancel`     | POST   | Moved                 |
| `POST /documents/reset_status`      | N/A                                | -      | **Not implemented**   |
| `GET /documents/scan-progress`      | N/A                                | -      | **Use tasks instead** |
| `POST /documents/clear_cache`       | N/A                                | -      | **Not implemented**   |

#### Documents List Response

**Old Format:**

```typescript
interface DocsStatusesResponse {
  statuses: Record<DocStatus, DocStatusResponse[]>;
}
```

**New Format:**

```typescript
interface ListDocumentsResponse {
  documents: Document[];
  total: number;
  page: number;
  page_size: number;
  status_counts: {
    pending: number;
    processing: number;
    completed: number;
    failed: number;
  };
}
```

#### Document Status Mapping

| Old Status     | New Status         |
| -------------- | ------------------ |
| `pending`      | `pending`          |
| `processing`   | `processing`       |
| `preprocessed` | N/A (intermediate) |
| `processed`    | `completed`        |
| `failed`       | `failed`           |
| N/A            | `indexed` (new)    |

#### Upload Document Request

**Old Format:**

```typescript
// POST /documents/text
interface InsertTextRequest {
  text: string;
}
```

**New Format:**

```typescript
// POST /api/v1/documents
interface UploadDocumentRequest {
  content: string;
  title?: string;
  source_type?: "text" | "file" | "url";
  metadata?: Record<string, unknown>;
  async_processing?: boolean;
  track_id?: string;
}
```

#### Upload Document Response

**Old Format:**

```typescript
interface DocActionResponse {
  status: "success" | "partial_success" | "failure" | "duplicated";
  message: string;
  track_id?: string;
}
```

**New Format:**

```typescript
interface UploadDocumentResponse {
  document_id: string;
  status: string;
  task_id?: string;
  track_id: string;
  duplicate_of?: string;
  chunk_count?: number;
  entity_count?: number;
  relationship_count?: number;
}
```

---

### Query

| Old Endpoint         | New Endpoint                | Method | Changes    |
| -------------------- | --------------------------- | ------ | ---------- |
| `POST /query`        | `POST /api/v1/query`        | POST   | URL prefix |
| `POST /query/stream` | `POST /api/v1/query/stream` | POST   | URL prefix |

#### Query Request

**Old Format:**

```typescript
interface QueryRequest {
  query: string;
  mode: "naive" | "local" | "global" | "hybrid" | "mix" | "bypass";
  only_need_context?: boolean;
  only_need_prompt?: boolean;
  response_type?: string;
  stream?: boolean;
  top_k?: number;
  chunk_top_k?: number;
  max_entity_tokens?: number;
  max_relation_tokens?: number;
  max_total_tokens?: number;
  conversation_history?: Message[];
  history_turns?: number;
  user_prompt?: string;
  enable_rerank?: boolean;
}
```

**New Format:**

```typescript
interface QueryRequest {
  query: string;
  mode: "local" | "global" | "hybrid" | "naive";
  top_k?: number;
  max_tokens?: number;
  temperature?: number;
  stream?: boolean;
  only_context?: boolean; // Was only_need_context
}
```

**Changes:**

- `mix` mode → use `hybrid`
- `bypass` mode → not exposed (direct LLM call)
- `only_need_context` → `only_context`
- `only_need_prompt` → removed
- `conversation_history` → handled client-side
- `response_type` → removed (default formatting)
- Token budget params → simplified to `max_tokens`

#### Query Response

**Old Format:**

```typescript
interface QueryResponse {
  response: string;
}
```

**New Format:**

```typescript
interface QueryResponse {
  answer: string; // Was response
  context: {
    chunks: Array<{ content: string; document_id: string; score: number }>;
    entities: Array<{ id: string; label: string; relevance: number }>;
    relationships: Array<{
      source: string;
      target: string;
      type: string;
      relevance: number;
    }>;
  };
  mode: QueryMode;
  tokens_used: number;
  duration_ms: number;
}
```

#### Stream Response

**Old Format:**

```json
{"response": "token chunk"}
{"response": "more tokens"}
{"error": "optional error"}
```

**New Format:**

```json
{"type": "token", "content": "token chunk"}
{"type": "context", "context": {...}}
{"type": "done", "tokens_used": 123, "duration_ms": 456}
{"type": "error", "error": "error message"}
```

---

### Knowledge Graph

| Old Endpoint                | New Endpoint                              | Method | Changes                   |
| --------------------------- | ----------------------------------------- | ------ | ------------------------- |
| `GET /graphs?label=X`       | `GET /api/v1/graph`                       | GET    | Query params              |
| `GET /graph/label/list`     | `GET /api/v1/graph/labels`                | GET    | Path changed              |
| `GET /graph/label/popular`  | `GET /api/v1/graph/labels/popular`        | GET    | Same                      |
| `GET /graph/label/search`   | `GET /api/v1/graph/labels/search`         | GET    | Same                      |
| `GET /graph/entity/exists`  | `GET /api/v1/graph/entities/exists`       | GET    | Path changed              |
| `POST /graph/entity/edit`   | `PUT /api/v1/graph/entities/{name}`       | PUT    | RESTful                   |
| `POST /graph/relation/edit` | `PUT /api/v1/graph/relationships/{id}`    | PUT    | RESTful                   |
| N/A                         | `POST /api/v1/graph/entities`             | POST   | New - Create entity       |
| N/A                         | `DELETE /api/v1/graph/entities/{name}`    | DELETE | New - Delete entity       |
| N/A                         | `POST /api/v1/graph/relationships`        | POST   | New - Create relationship |
| N/A                         | `DELETE /api/v1/graph/relationships/{id}` | DELETE | New - Delete relationship |

#### Graph Response

**Old Format:**

```typescript
interface LightragGraphType {
  nodes: Array<{
    id: string;
    labels: string[];
    properties: Record<string, any>;
  }>;
  edges: Array<{
    id: string;
    source: string;
    target: string;
    type: string;
    properties: Record<string, any>;
  }>;
}
```

**New Format:**

```typescript
interface KnowledgeGraph {
  nodes: Array<{
    id: string;
    label: string; // Was labels array
    node_type: string; // First label
    description?: string;
    degree?: number;
    properties?: Record<string, unknown>;
    created_at?: string;
    updated_at?: string;
  }>;
  edges: Array<{
    id: string;
    source: string;
    target: string;
    relationship_type: string; // Was type
    weight: number;
    description?: string;
    source_ids: string[];
    properties?: Record<string, unknown>;
    created_at: string;
  }>;
  metadata: {
    node_count: number;
    edge_count: number;
    entity_types: string[];
    relationship_types: string[];
  };
}
```

#### Entity Edit Request

**Old Format:**

```typescript
// POST /graph/entity/edit
interface EntityEditRequest {
  entity_name: string;
  updated_data: Record<string, any>;
  allow_rename?: boolean;
  allow_merge?: boolean;
}
```

**New Format:**

```typescript
// PUT /api/v1/graph/entities/{entity_name}
interface EntityUpdateRequest {
  label?: string;
  entity_type?: string;
  description?: string;
  properties?: Record<string, unknown>;
}
```

**Notes:** Rename and merge are now separate operations:

- Rename: Update `label` field
- Merge: Use `POST /api/v1/graph/entities/merge`

---

### Tasks (New - Replaces Pipeline Status)

| Endpoint                               | Method | Description       |
| -------------------------------------- | ------ | ----------------- |
| `GET /api/v1/tasks`                    | GET    | List all tasks    |
| `GET /api/v1/tasks/{track_id}`         | GET    | Get task details  |
| `POST /api/v1/tasks/{track_id}/cancel` | POST   | Cancel task       |
| `POST /api/v1/tasks/{track_id}/retry`  | POST   | Retry failed task |

#### Task Response

```typescript
interface TaskResponse {
  track_id: string;
  task_type: string;
  status: "pending" | "processing" | "indexed" | "failed" | "cancelled";
  created_at: string;
  updated_at: string;
  started_at?: string;
  completed_at?: string;
  error_message?: string;
  error?: {
    message: string;
    step: string;
    reason: string;
    suggestion: string;
    retryable: boolean;
  };
  retry_count: number;
  max_retries: number;
  progress?: Record<string, unknown>;
  result?: Record<string, unknown>;
}
```

#### Task List Response

```typescript
interface TaskListResponse {
  tasks: TaskResponse[];
  pagination: {
    total: number;
    page: number;
    page_size: number;
    total_pages: number;
  };
  statistics: {
    pending: number;
    processing: number;
    indexed: number;
    failed: number;
    cancelled: number;
  };
}
```

#### Migrating Pipeline Status

**Old Pattern:**

```typescript
// Polling pipeline status
const status = await getPipelineStatus();
console.log(status.busy, status.job_name, status.latest_message);
```

**New Pattern:**

```typescript
// Using task API
const tasks = await getTasksList({ status: "processing" });
const isBusy = tasks.statistics.processing > 0;
const runningTasks = tasks.tasks.filter((t) => t.status === "processing");
```

---

### Ollama Emulation API

**No changes** - Full backward compatibility maintained.

| Endpoint             | Method | Status  |
| -------------------- | ------ | ------- |
| `GET /api/version`   | GET    | ✅ Same |
| `GET /api/tags`      | GET    | ✅ Same |
| `GET /api/ps`        | GET    | ✅ Same |
| `POST /api/generate` | POST   | ✅ Same |
| `POST /api/chat`     | POST   | ✅ Same |

---

## Header Changes

### Tenant Context

**Old:**

```
X-Tenant-ID: tenant_id
X-KB-ID: kb_id
```

**New:**

```
X-Tenant-ID: tenant_id
X-Workspace-ID: workspace_id  (replaces X-KB-ID)
```

### Authorization

No changes - both use:

```
Authorization: Bearer <token>
X-API-Key: <api_key>
```

---

## Error Response Format

**Old Format:**

```typescript
// Inconsistent, often just strings
{
  "detail": "Error message"
}
// or
{
  "status": "error",
  "message": "Error message"
}
```

**New Format:**

```typescript
interface ApiError {
  message: string;
  code?: string;
  details?: Record<string, unknown>;
  status: number;
}
```

---

## Migration Checklist

### Phase 1: Non-Breaking Changes

- [ ] Add `/api/v1/` prefix to API client base URL
- [ ] Update health check (no changes needed)
- [ ] Update Ollama emulation calls (no changes needed)

### Phase 2: Authentication

- [ ] Update login to use JSON body instead of FormData
- [ ] Add refresh token handling
- [ ] Update auth state to store refresh token
- [ ] Handle new user object in response

### Phase 3: Tenant/Workspace

- [ ] Rename Knowledge Base → Workspace in UI
- [ ] Update header from X-KB-ID → X-Workspace-ID
- [ ] Update tenant API calls to new paths
- [ ] Create adapter for backward compatibility

### Phase 4: Documents

- [ ] Update document list to handle new pagination
- [ ] Update upload to use new request format
- [ ] Update delete to use RESTful path
- [ ] Migrate pipeline status to task API
- [ ] Update track status path

### Phase 5: Query

- [ ] Update mode handling (mix → hybrid)
- [ ] Update response handling for context
- [ ] Update stream parsing for new format
- [ ] Handle only_context instead of only_need_context

### Phase 6: Graph

- [ ] Update graph fetch to new path
- [ ] Update labels API paths
- [ ] Update entity edit to use PUT
- [ ] Update relationship edit to use PUT
- [ ] Implement merge as separate operation

### Phase 7: Testing

- [ ] Test all CRUD operations
- [ ] Test streaming queries
- [ ] Test file uploads
- [ ] Test tenant switching
- [ ] Test error handling

---

## Adapter Layer Implementation

For gradual migration, implement an adapter layer:

```typescript
// lib/api/adapter.ts

import { api } from "./client";

// Adapt old-style calls to new API
export const legacyApi = {
  // Documents
  async getDocuments() {
    const response = await api.get<ListDocumentsResponse>("/documents");
    // Convert to old format
    return {
      statuses: groupByStatus(response.documents),
    };
  },

  // Query
  async query(request: LegacyQueryRequest) {
    const newRequest = adaptQueryRequest(request);
    const response = await api.post<QueryResponse>("/query", newRequest);
    return { response: response.answer };
  },

  // ... more adapters
};
```

This allows incremental migration while maintaining backward compatibility.
