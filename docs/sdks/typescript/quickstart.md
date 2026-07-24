---
title: "TypeScript SDK — quickstart"
---

# TypeScript SDK — quickstart

> **Product: v0.19.0** · Contract: OpenAPI · Spec ops: [Ingestion cancel & fairness](../../ingestion-cancel-and-fairness.md)

> **SDK package: ~0.4.0** (≠ product version)

## Run tests

```bash
cd sdks/typescript && bun test
```

## Upload + list

```typescript
import { EdgeQuakeClient } from "@edgequake/sdk";

const client = new EdgeQuakeClient({ baseUrl: "http://localhost:8080" });

await client.documents.upload({
  content: "# Hello\n\nEdgeQuake",
  title: "demo.md",
});

const list = await client.documents.list({ page: 1, page_size: 10 });
```

## Conversations bulk delete

The client sends `{ conversation_ids: [...] }` and expects `{ affected: number }` from the API.

## Progress / cancel

```typescript
await client.tasks.cancel(trackId);
// WebSocket: ws://host/ws/progress/{track_id}
```

See [Ingestion cancel & fairness](../../ingestion-cancel-and-fairness.md).

## Costs / pipeline pricing

Pipeline cost endpoints share DRY path constants in the SDK — they map to `/api/v1/pipeline/costs/...` as routed in `routes.rs`.
