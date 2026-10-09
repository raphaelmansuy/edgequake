---
title: "TypeScript / Node SDK"
description: "TypeScript SDK for EdgeQuake."
---

# TypeScript / Node SDK

> **Product: v0.23.0** · Package **~0.4.0** (decoupled from server)

**Location:** `sdks/typescript`

## Install

```bash
cd sdks/typescript && npm install   # or bun install
```

## Example

```typescript
import { EdgeQuakeClient } from "@edgequake/sdk";

const client = new EdgeQuakeClient({
  baseUrl: "http://localhost:8080",
  apiKey: process.env.EDGEQUAKE_API_KEY!,
  tenantId: process.env.EDGEQUAKE_TENANT_ID,
  userId: process.env.EDGEQUAKE_USER_ID,
  workspaceId: process.env.EDGEQUAKE_WORKSPACE_ID,
});

const health = await client.health.check();
console.log(health.status);

const docs = await client.documents.list({
  page: 1,
  page_size: 20,
  document_pattern: "quarterly",
});
console.log(docs.documents.length, docs.status_counts);
```

## Lawful document list filters

`ListDocumentsQuery` supports: `page`, `page_size`, `date_from`, `date_to`, `document_pattern` — matching the Rust `ListDocumentsRequest`.

## Progress / cancel (v0.23)

Use `client.tasks.cancel(trackId)` or raw `POST /api/v1/tasks/{track_id}/cancel`. WebSocket progress: `/ws/progress/{track_id}`. See [Ingestion cancel & fairness](../../ingestion-cancel-and-fairness.md).

## Stateless parse (SPEC-094, v0.23)

`client.parse` maps to `POST /api/v1/parse` (multipart `file` + `options`) and converts a PDF to Markdown without ingestion residue. Sync by default (≤ 15 pages / 20 MiB); pass `async: true` for jobs up to 1000 pages and poll `client.parse.job()` (`GET /api/v1/parse/jobs/{id}`).

```typescript
import { readFile } from "node:fs/promises";

const file = new File([await readFile("/tmp/paper.pdf")], "paper.pdf", { type: "application/pdf" });

// Sync parse
const res = await client.parse.parse(
  file,
  { backend: "vision", pages: "1-5" },
);
if ("markdown" in res) {
  console.log(res.markdown.slice(0, 200), res.page_count, res.metrics.total_ms);
}

// Backends + ceilings
const backends = await client.parse.backends();
console.log(backends.default_backend, backends.limits.sync_max_pages);

// Async job
const accepted = await client.parse.parse(file, { async: true });
if ("job_id" in accepted) {
  const status = await client.parse.job(accepted.job_id);
  if (status.result) console.log(status.result.markdown.slice(0, 200));
}
```

## Document display fields (v0.23)

`DocumentSummary` / `DocumentDetailResponse` include `display_status` and `ui_phase` — prefer them over raw `status`/`stage` for progress UI (SPEC-057 P4).

## LLM cache (server-side)

`EDGEQUAKE_LLM_CACHE=1` (default) caches keyword extraction + answers on the **server**; `EDGEQUAKE_KEYWORD_CACHE` / `EDGEQUAKE_QUERY_ANSWER_CACHE` override. No client change — repeated queries transparently return cached answers.

## See also

- [Quickstart](./quickstart.md)
- `sdks/typescript/README.md`
