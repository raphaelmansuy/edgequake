---
title: 'Integration: Custom Clients'
description: "Build a custom HTTP client against the EdgeQuake REST API."
---

# Integration: Custom Clients

> **Product: v0.23.0** · Contract: [`openapi.snapshot.json`](../../edgequake_webui/openapi/openapi.snapshot.json) · Spec ops: [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md)

Thin HTTP cookbook for custom integrations. **Use official SDKs when possible** — they track OpenAPI and handle auth, retries, and DTOs.

| Language | Package | Docs |
| -------- | ------- | ---- |
| Python | `pip install edgequake-sdk` | [Python SDK](/docs/sdks/python/) |
| TypeScript | `@edgequake/sdk` (in-repo) | [TypeScript SDK](/docs/sdks/typescript/) |
| Rust | `edgequake-sdk` crate | [Rust SDK](/docs/sdks/rust/) |
| Kotlin | Maven `io.edgequake:edgequake-sdk-kotlin` | [Kotlin SDK](/docs/sdks/kotlin/) |
| Go, Java, C#, Ruby, Swift | in-repo | [SDK index](/docs/sdks/) |

**Base URL**: `http://localhost:8080` (API) · WebUI `:3000`

---

## Headers

| Header | When |
| ------ | ---- |
| `X-Workspace-ID` | Scope documents/query (default workspace if omitted in dev) |
| `X-Tenant-ID` | Multi-tenant deployments |
| `Authorization: Bearer <jwt>` | When `EDGEQUAKE_AUTH_ENABLED=true` |
| `X-API-Key` | API key auth (if configured) |

Quickstart/dev often runs with auth disabled (`EDGEQUAKE_DEV_MODE=true`).

---

## Health

```bash
curl -s http://localhost:8080/health | jq .
# {"status":"healthy","storage_mode":"postgresql", ...}
```

---

## Upload

### PDF / files (multipart)

```bash
# PDF-specific (preferred for PDFs)
curl -X POST http://localhost:8080/api/v1/documents/pdf \
  -H "X-Workspace-ID: default" \
  -F "file=@report.pdf" \
  -F "title=Report" \
  -F "pdf_parser_backend=vision"

# Generic file upload (PDF, txt, md, …)
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -H "X-Workspace-ID: default" \
  -F "file=@report.pdf"
```

**Not** `POST /api/v1/documents` with multipart — that route is **JSON text only**:

```bash
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: default" \
  -d '{"content":"Plain text body","title":"Note"}'
```

Upload response includes **`task_id`** (progress/cancel SSOT). Optional **`track_id`** is client correlation only.

---

## Status & presentation (SPEC-057)

Document JSON includes:

| Field | Values (examples) |
| ----- | ----------------- |
| `display_status` | `converting`, `extracting`, `embedding`, **`completed`**, `failed`, **`cancelled`** |
| `ui_phase` | `running`, **`stopping`**, `terminal` |

There is **no** `indexed` status. Treat **`display_status: completed`** as ready for query.

```bash
curl -s "http://localhost:8080/api/v1/documents/{id}" \
  -H "X-Workspace-ID: default" | jq '{display_status, ui_phase, status}'
```

PDF convert completes before KG ingest finishes — see [PDF Ingestion](/docs/tutorials/pdf-ingestion/#two-phases-convert--ingest-spec-057).

---

## Progress & cancel

```bash
TASK_ID="pdf-…"   # from upload response task_id

# Poll
curl -s "http://localhost:8080/api/v1/documents/pdf/progress/${TASK_ID}" \
  -H "X-Workspace-ID: default"

# Cancel (canonical)
curl -X POST "http://localhost:8080/api/v1/tasks/${TASK_ID}/cancel" \
  -H "X-Workspace-ID: default"
```

Also: `DELETE /api/v1/documents/pdf/{pdf_id}/cancel`, WebSocket cancel message. Details: [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md).

---

## Query

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: default" \
  -d '{"query":"What is EdgeQuake?","mode":"hybrid","top_k":10}'
```

**Response shape**:

```json
{
  "answer": "…",
  "sources": [{ "document_id": "…", "snippet": "…", "score": 0.9 }],
  "stats": { "total_time_ms": 500 },
  "mode": "hybrid"
}
```

No top-level `chunks` / `entities`. Use `sources[].snippet` as citation text.

---

## Chat (SSE)

```bash
curl -N -X POST http://localhost:8080/api/v1/chat/completions/stream \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: default" \
  -d '{"message":"Summarize the docs","mode":"hybrid"}'
```

Non-streaming: `POST /api/v1/chat/completions`.

---

## Ollama-compatible API

For Open WebUI and other Ollama clients: `POST /api/chat`, `GET /api/tags`. See [Open WebUI integration](/docs/integrations/open-webui/).

---

## Python one-liner (SDK)

```python
from edgequake import EdgeQuake

with EdgeQuake(base_url="http://localhost:8080", workspace_id="default") as c:
    print(c.query.execute(query="Hello").answer)
```

---

## HTTP status codes

| Code | Meaning |
| ---- | ------- |
| 200 | OK |
| 400 | Bad request |
| 401 | Unauthorized |
| 404 | Not found |
| 409 | Duplicate PDF |
| 413 | Payload too large |
| 429 | Rate limited |
| 503 | Not ready (e.g. store contention critical) |

Error body: `{"error":{"code":"…","message":"…"}}` (see OpenAPI).

---

## See also

- [Document Upload Quick Reference](/docs/api-reference/document-upload-quick-reference/)
- [REST API Reference](/docs/api-reference/rest-api/)
- [LangChain Integration](/docs/integrations/langchain/)
- [SDK index](/docs/sdks/)
