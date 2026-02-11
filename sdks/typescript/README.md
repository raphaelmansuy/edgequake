# @edgequake/sdk

TypeScript SDK for the [EdgeQuake](https://github.com/raphaelmansuy/edgequake) RAG API.

## Features

- **Full API Coverage** — All 131+ endpoints across 27 resource categories
- **Type-Safe** — Complete TypeScript definitions for every request/response
- **Zero Dependencies** — Uses native `fetch()` (Node 18+, Deno, Bun, Browser)
- **Streaming** — Server-Sent Events (SSE) and WebSocket support with `AsyncIterable`
- **Auto-Pagination** — `for await...of` over paginated results
- **Retry & Backoff** — Configurable exponential backoff with jitter
- **Multi-Tenant** — Built-in tenant/workspace header management
- **Dual Format** — ESM + CommonJS output with `.d.ts` declarations

## Installation

```bash
npm install @edgequake/sdk
```

## Quick Start

```typescript
import { EdgeQuake } from "@edgequake/sdk";

const client = new EdgeQuake({
  baseUrl: "http://localhost:8080",
  apiKey: "eq-key-xxx",
});

// Health check
const health = await client.health();
console.log(health.status); // "healthy"

// Upload a document
const doc = await client.documents.upload({
  title: "My Document",
  content: "EdgeQuake is a graph-based RAG framework...",
});

// Query the knowledge graph
const result = await client.query.execute({
  query: "What is EdgeQuake?",
  mode: "hybrid",
});
console.log(result.answer);

// Stream chat completions
for await (const event of client.chat.stream({
  messages: [{ role: "user", content: "Summarize the documents" }],
})) {
  process.stdout.write(event.choices?.[0]?.delta?.content ?? "");
}
```

## Configuration

```typescript
const client = new EdgeQuake({
  baseUrl: "http://localhost:8080", // Default
  apiKey: "eq-key-xxx", // API key auth
  accessToken: "jwt-token", // Or JWT auth
  tenantId: "tenant-1", // Multi-tenant
  workspaceId: "workspace-1", // Workspace scope
  timeout: 30000, // 30s default
  maxRetries: 3, // Retry on 429/503
});
```

Environment variables are used as fallbacks:

- `EDGEQUAKE_BASE_URL`
- `EDGEQUAKE_API_KEY`
- `EDGEQUAKE_TENANT_ID`
- `EDGEQUAKE_WORKSPACE_ID`

## Resource Namespaces

| Namespace                       | Description                          |
| ------------------------------- | ------------------------------------ |
| `client.auth`                   | Login, refresh, logout, current user |
| `client.users`                  | User management (admin)              |
| `client.apiKeys`                | API key management                   |
| `client.documents`              | Document ingestion & management      |
| `client.documents.pdf`          | PDF upload, extraction, download     |
| `client.query`                  | RAG query execution & streaming      |
| `client.chat`                   | Chat completions (unified API)       |
| `client.conversations`          | Conversation history                 |
| `client.conversations.messages` | Message CRUD                         |
| `client.folders`                | Conversation folders                 |
| `client.shared`                 | Public shared conversations          |
| `client.graph`                  | Knowledge graph queries              |
| `client.graph.entities`         | Entity CRUD & merge                  |
| `client.graph.relationships`    | Relationship CRUD                    |
| `client.tenants`                | Multi-tenant management              |
| `client.workspaces`             | Workspace management                 |
| `client.tasks`                  | Async task tracking                  |
| `client.pipeline`               | Pipeline status & control            |
| `client.costs`                  | Cost tracking & budgets              |
| `client.lineage`                | Entity & document lineage            |
| `client.chunks`                 | Chunk-level details                  |
| `client.provenance`             | Entity provenance                    |
| `client.settings`               | Provider settings                    |
| `client.models`                 | Model configuration                  |
| `client.ollama`                 | Ollama-compatible API                |

## Pagination

```typescript
// Auto-iterate through all pages
for await (const doc of client.documents.list({ status: "completed" })) {
  console.log(doc.title);
}

// Get a specific page
const page = await client.documents.list().getPage(2);
console.log(page.items, page.hasMore);

// Collect all into array
const all = await client.documents.list().toArray();
```

## Error Handling

```typescript
import {
  NotFoundError,
  RateLimitedError,
  EdgeQuakeError,
} from "@edgequake/sdk";

try {
  await client.documents.get("missing-id");
} catch (error) {
  if (error instanceof NotFoundError) {
    console.log("Document not found");
  } else if (error instanceof RateLimitedError) {
    console.log("Rate limited, retry later");
  } else if (error instanceof EdgeQuakeError) {
    console.log(`API error: ${error.code} (${error.status})`);
  }
}
```

## Examples

See the [`examples/`](./examples/) directory for complete working examples:

| Example                                                     | Description                                |
| ----------------------------------------------------------- | ------------------------------------------ |
| [`basic_usage.ts`](./examples/basic_usage.ts)               | Setup, health check, upload, query         |
| [`document_upload.ts`](./examples/document_upload.ts)       | Text + PDF upload, tracking, pagination    |
| [`query_demo.ts`](./examples/query_demo.ts)                 | Simple, hybrid, and chat queries           |
| [`graph_exploration.ts`](./examples/graph_exploration.ts)   | Entity search, neighborhood, relationships |
| [`streaming_query.ts`](./examples/streaming_query.ts)       | SSE streaming query + chat + abort         |
| [`websocket_progress.ts`](./examples/websocket_progress.ts) | WebSocket pipeline progress                |
| [`multi_tenant.ts`](./examples/multi_tenant.ts)             | Tenant/workspace management                |
| [`batch_operations.ts`](./examples/batch_operations.ts)     | Bulk operations, pagination, cost estimate |

Run any example with:

```bash
npx tsx examples/basic_usage.ts
```

## Documentation

- [**API Reference**](./docs/API.md) — All endpoints, methods, and types
- [**Authentication**](./docs/AUTHENTICATION.md) — API key, JWT, multi-tenant
- [**Streaming**](./docs/STREAMING.md) — SSE + WebSocket patterns

## Development

```bash
npm install          # Install dependencies
npm run build        # Build ESM + CJS + .d.ts
npm test             # Run 243 unit tests
npm run test:coverage # Run tests with coverage report
npm run lint         # TypeScript type check
```

## License

Apache-2.0
