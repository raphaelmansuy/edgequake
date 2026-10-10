---
title: TypeScript SDK
description: Install and use the EdgeQuake TypeScript client (npm package edgequake-sdk). Node 18+, class EdgeQuake.
---

# TypeScript SDK

Official TypeScript / JavaScript client. Requires **Node.js 18+**. Package name: **`edgequake-sdk`** (not `@edgequake/sdk`). Source version **0.4.0**; latest on npm is **0.1.0**.

```bash
npm install edgequake-sdk@0.1.0
# Source tree (0.4.0): npm install ./sdks/typescript
```

```ts
import { EdgeQuake } from "edgequake-sdk";

const client = new EdgeQuake({
  baseUrl: "http://localhost:8080",
  apiKey: "eq-...",
  workspaceId: "...",
});

const health = await client.health();
const answer = await client.query.execute({
  query: "What is in my documents?",
  mode: "mix",
  max_results: 10,
});
console.log(answer.answer);
```

## Resources

Same layout as Python: `documents` (with `documents.pdf`), `parse`, `query`, `chat`, `graph`, `entities`, `relationships`, `conversations`, `folders`, `auth`, `users`, `apiKeys`, `tenants`, `workspaces`, `tasks`, `pipeline`, `costs`, `lineage`, `chunks`, `settings`, `models`. Streaming helpers: `parseSSEStream`, `EdgeQuakeWebSocket`.

Query requests use the real field names (`max_results`, `enable_rerank`, `llm_provider`). Connections and `providers/test` are not wrapped.

Quickstart: [quickstart.md](quickstart.md). API: [REST API](../../api-reference/rest-api.md).
