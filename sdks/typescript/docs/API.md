# API Reference — @edgequake/sdk

## Client

### `new EdgeQuake(config)`

Create a new SDK client.

```typescript
import { EdgeQuake } from "@edgequake/sdk";

const client = new EdgeQuake({
  baseUrl: "http://localhost:8080",
  apiKey: "your-api-key",
  tenantId: "optional-tenant-id", // multi-tenant header
  workspaceId: "optional-workspace", // workspace isolation
  timeout: 30_000, // request timeout (ms)
  maxRetries: 3, // auto-retry count
});
```

### Health Checks

```typescript
await client.health(); // GET /health
await client.ready(); // GET /ready
await client.live(); // GET /live
```

---

## Resources

### `client.auth`

| Method               | Endpoint                    | Description                |
| -------------------- | --------------------------- | -------------------------- |
| `login(credentials)` | `POST /api/v1/auth/login`   | Authenticate and get JWT   |
| `refresh(token)`     | `POST /api/v1/auth/refresh` | Refresh JWT token          |
| `logout()`           | `POST /api/v1/auth/logout`  | Invalidate current session |
| `me()`               | `GET /api/v1/auth/me`       | Get current user info      |

### `client.documents`

| Method                          | Endpoint                                    | Description                |
| ------------------------------- | ------------------------------------------- | -------------------------- |
| `upload(request)`               | `POST /api/v1/documents`                    | Upload text document       |
| `uploadFile(file, metadata?)`   | `POST /api/v1/documents`                    | Upload file (multipart)    |
| `uploadBatch(files, metadata?)` | `POST /api/v1/documents/batch`              | Upload multiple files      |
| `list(params?)`                 | `GET /api/v1/documents`                     | List documents (paginated) |
| `get(id)`                       | `GET /api/v1/documents/:id`                 | Get document details       |
| `delete(id)`                    | `DELETE /api/v1/documents/:id`              | Delete document            |
| `deleteAll()`                   | `DELETE /api/v1/documents`                  | Delete all documents       |
| `getTrackStatus(trackId)`       | `GET /api/v1/documents/track/:id`           | Track processing status    |
| `analyzeDeletionImpact(id)`     | `GET /api/v1/documents/:id/deletion-impact` | Analyze deletion impact    |
| `scan(request)`                 | `POST /api/v1/documents/scan`               | Trigger document scan      |
| `reprocessFailed()`             | `POST /api/v1/documents/reprocess`          | Reprocess failed docs      |
| `recoverStuck()`                | `POST /api/v1/documents/recover-stuck`      | Recover stuck docs         |
| `retryFailedChunks(id)`         | `POST /api/v1/documents/:id/retry-chunks`   | Retry failed chunks        |
| `listFailedChunks(id)`          | `GET /api/v1/documents/:id/failed-chunks`   | List failed chunks         |

### `client.documents.pdf`

| Method                    | Endpoint                                  | Description             |
| ------------------------- | ----------------------------------------- | ----------------------- |
| `upload(file, metadata?)` | `POST /api/v1/documents/pdf`              | Upload PDF              |
| `list()`                  | `GET /api/v1/documents/pdf`               | List PDFs               |
| `getStatus(id)`           | `GET /api/v1/documents/pdf/:id`           | Get PDF status          |
| `getContent(id)`          | `GET /api/v1/documents/pdf/:id/content`   | Get extracted content   |
| `download(id)`            | `GET /api/v1/documents/pdf/:id/download`  | Download PDF blob       |
| `getProgress(id)`         | `GET /api/v1/documents/pdf/progress/:id`  | Get extraction progress |
| `retry(id)`               | `POST /api/v1/documents/pdf/:id/retry`    | Retry extraction        |
| `cancel(id)`              | `DELETE /api/v1/documents/pdf/:id/cancel` | Cancel extraction       |
| `delete(id)`              | `DELETE /api/v1/documents/pdf/:id`        | Delete PDF              |

### `client.query`

| Method             | Endpoint                    | Description                 |
| ------------------ | --------------------------- | --------------------------- |
| `execute(request)` | `POST /api/v1/query`        | Execute RAG query           |
| `stream(request)`  | `POST /api/v1/query/stream` | Stream query response (SSE) |

### `client.chat`

| Method                 | Endpoint                               | Description       |
| ---------------------- | -------------------------------------- | ----------------- |
| `completions(request)` | `POST /api/v1/chat/completions`        | Chat completion   |
| `stream(request)`      | `POST /api/v1/chat/completions/stream` | Stream chat (SSE) |

### `client.graph`

| Method                     | Endpoint                           | Description             |
| -------------------------- | ---------------------------------- | ----------------------- |
| `get()`                    | `GET /api/v1/graph`                | Get graph overview      |
| `stream()`                 | `GET /api/v1/graph/stream`         | Stream graph data (SSE) |
| `getNode(id)`              | `GET /api/v1/graph/nodes/:id`      | Get node details        |
| `searchNodes(params)`      | `GET /api/v1/graph/nodes/search`   | Search nodes            |
| `searchLabels(params)`     | `GET /api/v1/graph/labels/search`  | Search labels           |
| `getPopularLabels()`       | `GET /api/v1/graph/labels/popular` | Get popular labels      |
| `getDegreesBatch(request)` | `POST /api/v1/graph/degrees/batch` | Batch degree lookup     |

### `client.graph.entities`

| Method                  | Endpoint                                        | Description      |
| ----------------------- | ----------------------------------------------- | ---------------- |
| `list()`                | `GET /api/v1/graph/entities`                    | List entities    |
| `create(request)`       | `POST /api/v1/graph/entities`                   | Create entity    |
| `get(name)`             | `GET /api/v1/graph/entities/:name`              | Get entity       |
| `exists(name)`          | `GET /api/v1/graph/entities/exists`             | Check existence  |
| `update(name, request)` | `PUT /api/v1/graph/entities/:name`              | Update entity    |
| `delete(name)`          | `DELETE /api/v1/graph/entities/:name`           | Delete entity    |
| `merge(request)`        | `POST /api/v1/graph/entities/merge`             | Merge entities   |
| `neighborhood(name)`    | `GET /api/v1/graph/entities/:name/neighborhood` | Get neighborhood |

### `client.graph.relationships`

| Method                | Endpoint                                 | Description         |
| --------------------- | ---------------------------------------- | ------------------- |
| `list()`              | `GET /api/v1/graph/relationships`        | List relationships  |
| `create(request)`     | `POST /api/v1/graph/relationships`       | Create relationship |
| `get(id)`             | `GET /api/v1/graph/relationships/:id`    | Get relationship    |
| `update(id, request)` | `PUT /api/v1/graph/relationships/:id`    | Update relationship |
| `delete(id)`          | `DELETE /api/v1/graph/relationships/:id` | Delete relationship |

### `client.conversations`

| Method                 | Endpoint                                  | Description         |
| ---------------------- | ----------------------------------------- | ------------------- |
| `list(params?)`        | `GET /api/v1/conversations`               | List (paginated)    |
| `get(id)`              | `GET /api/v1/conversations/:id`           | Get conversation    |
| `create(request)`      | `POST /api/v1/conversations`              | Create conversation |
| `update(id, request)`  | `PATCH /api/v1/conversations/:id`         | Update conversation |
| `delete(id)`           | `DELETE /api/v1/conversations/:id`        | Delete conversation |
| `share(id, request)`   | `POST /api/v1/conversations/:id/share`    | Share conversation  |
| `unshare(id)`          | `DELETE /api/v1/conversations/:id/share`  | Unshare             |
| `import(request)`      | `POST /api/v1/conversations/import`       | Import conversation |
| `bulkDelete(request)`  | `POST /api/v1/conversations/bulk/delete`  | Bulk delete         |
| `bulkArchive(request)` | `POST /api/v1/conversations/bulk/archive` | Bulk archive        |
| `bulkMove(request)`    | `POST /api/v1/conversations/bulk/move`    | Bulk move           |

### `client.conversations.messages`

| Method                            | Endpoint                                  | Description    |
| --------------------------------- | ----------------------------------------- | -------------- |
| `list(conversationId)`            | `GET /api/v1/conversations/:id/messages`  | List messages  |
| `create(conversationId, request)` | `POST /api/v1/conversations/:id/messages` | Add message    |
| `update(messageId, request)`      | `PATCH /api/v1/messages/:id`              | Update message |
| `delete(messageId)`               | `DELETE /api/v1/messages/:id`             | Delete message |

### `client.tenants`

| Method                               | Endpoint                                           | Description      |
| ------------------------------------ | -------------------------------------------------- | ---------------- |
| `create(request)`                    | `POST /api/v1/tenants`                             | Create tenant    |
| `list()`                             | `GET /api/v1/tenants`                              | List tenants     |
| `get(id)`                            | `GET /api/v1/tenants/:id`                          | Get tenant       |
| `update(id, request)`                | `PUT /api/v1/tenants/:id`                          | Update tenant    |
| `delete(id)`                         | `DELETE /api/v1/tenants/:id`                       | Delete tenant    |
| `createWorkspace(tenantId, request)` | `POST /api/v1/tenants/:id/workspaces`              | Create workspace |
| `listWorkspaces(tenantId)`           | `GET /api/v1/tenants/:id/workspaces`               | List workspaces  |
| `getWorkspaceBySlug(tenantId, slug)` | `GET /api/v1/tenants/:id/workspaces/by-slug/:slug` | Get by slug      |

### `client.workspaces`

| Method                       | Endpoint                                              | Description      |
| ---------------------------- | ----------------------------------------------------- | ---------------- |
| `get(id)`                    | `GET /api/v1/workspaces/:id`                          | Get workspace    |
| `update(id, request)`        | `PUT /api/v1/workspaces/:id`                          | Update workspace |
| `delete(id)`                 | `DELETE /api/v1/workspaces/:id`                       | Delete workspace |
| `stats(id)`                  | `GET /api/v1/workspaces/:id/stats`                    | Get stats        |
| `metricsHistory(id)`         | `GET /api/v1/workspaces/:id/metrics-history`          | Metrics history  |
| `triggerMetricsSnapshot(id)` | `POST /api/v1/workspaces/:id/metrics-snapshot`        | Trigger snapshot |
| `rebuildEmbeddings(id)`      | `POST /api/v1/workspaces/:id/rebuild-embeddings`      | Rebuild          |
| `rebuildKnowledgeGraph(id)`  | `POST /api/v1/workspaces/:id/rebuild-knowledge-graph` | Rebuild          |
| `reprocessDocuments(id)`     | `POST /api/v1/workspaces/:id/reprocess-documents`     | Reprocess        |

### Other Resources

| Resource            | Key Methods                                                             |
| ------------------- | ----------------------------------------------------------------------- |
| `client.users`      | `create`, `list`, `get`, `delete`                                       |
| `client.apiKeys`    | `create`, `list`, `revoke`                                              |
| `client.folders`    | `list`, `create`, `update`, `delete`                                    |
| `client.shared`     | `get`                                                                   |
| `client.tasks`      | `get`, `list`, `cancel`, `retry`                                        |
| `client.pipeline`   | `status`, `cancel`, `queueMetrics`, `pricing`, `estimateCost`           |
| `client.costs`      | `summary`, `history`, `budget`, `updateBudget`                          |
| `client.lineage`    | `entity`, `document`                                                    |
| `client.chunks`     | `get`                                                                   |
| `client.provenance` | `get`                                                                   |
| `client.settings`   | `providerStatus`, `listProviders`                                       |
| `client.models`     | `list`, `listLlm`, `listEmbedding`, `health`, `getProvider`, `getModel` |
| `client.ollama`     | `version`, `tags`, `ps`, `generate`, `chat`                             |

---

## Error Types

```typescript
import {
  EdgeQuakeError, // Base error class
  NotFoundError, // 404
  UnauthorizedError, // 401
  ForbiddenError, // 403
  ValidationError, // 400/422
  ConflictError, // 409
  RateLimitedError, // 429
  InternalServerError, // 500
  NetworkError, // Connection failures
  TimeoutError, // Request timeout
} from "@edgequake/sdk";
```

## Pagination

```typescript
// Auto-paginating iterator
for await (const doc of client.documents.list({ limit: 50 })) {
  console.log(doc.title);
}

// Manual pagination
const page = await client.documents.list({ limit: 10 }).nextPage();
console.log(page.items, page.total, page.hasMore);
```

## Streaming

```typescript
// SSE streaming
for await (const event of client.query.stream({ query: "..." })) {
  console.log(event.chunk);
}

// WebSocket
import { EdgeQuakeWebSocket } from "@edgequake/sdk";
const ws = new EdgeQuakeWebSocket("ws://localhost:8080/ws/pipeline/progress");
for await (const event of ws) {
  console.log(event.type, event.progress);
}
```
