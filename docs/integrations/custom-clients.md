---
title: "Integration: Custom clients"
description: Thin HTTP cookbook for calling EdgeQuake without an SDK. Covers headers, upload, progress, query and the v0.33.0 Connections routes.
---

# Integration: Custom clients

Use this page when no SDK fits, or when you need routes the SDKs do not wrap yet (Connections, `providers/test`). Prefer an [official SDK](../sdks/README.md) for day-to-day work. Base URL: `http://localhost:8080`.

## Headers

| Header | When |
|--------|------|
| `Authorization: Bearer <jwt-or-key>` | Auth enabled |
| `X-API-Key: <key>` | Alternative to Bearer for API keys |
| `X-Workspace-ID` | Scope documents and queries |
| `X-Tenant-ID` | Multi-tenant deployments |
| `Content-Type: application/json` | JSON bodies |
| `X-EdgeQuake-Confirm: delete-all-documents` | Bulk wipe when confirmation is required |

Full conventions: [REST API](../api-reference/rest-api.md#conventions).

## Health

```bash
curl -s http://localhost:8080/health
curl -s http://localhost:8080/ready
```

## Upload and wait

```bash
# Text
curl -s -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: $WORKSPACE_ID" \
  -d '{"content":"Marie Curie won two Nobel Prizes.","title":"Notes"}'
# → 202 { "document_id", "task_id", "track_id", "status":"pending" }

# PDF (preferred path)
curl -s -X POST http://localhost:8080/api/v1/documents/pdf \
  -H "X-Workspace-ID: $WORKSPACE_ID" \
  -F "file=@paper.pdf" -F "title=Paper"
# → 200 { "pdf_id", "task_id":"pdf-...", "status":"queued" }

# Poll until terminal
curl -s http://localhost:8080/api/v1/tasks/$TASK_ID
# status: pending | processing | indexed | failed | cancelled
```

Details: [Document upload](../api-reference/document-upload-quick-reference.md).

## Query

```bash
curl -s -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: $WORKSPACE_ID" \
  -d '{"query":"How many Nobel Prizes?","mode":"mix","max_results":10}'
```

Real fields include `query`, `mode`, `max_results`, `enable_rerank`, `llm_provider`, `llm_model`, `document_filter`. There is no `top_k` on this endpoint. Streaming: `POST /api/v1/query/stream` (SSE JSON events with `type`: `context`, `token`, `thinking`, `done`, `error`).

## Connections (v0.33.0)

No SDK covers these yet. Admin only.

```bash
curl -s -X POST http://localhost:8080/api/v1/providers/test \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"shape":"ollama","base_url":"http://127.0.0.1:11434","model":"gemma3:latest"}'

curl -s -X POST http://localhost:8080/api/v1/connections \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"slug":"my-ollama","display_name":"Ollama","api_shape":"ollama","base_url":"http://127.0.0.1:11434","locality":"local"}'
```

Full reference: [Connections](../api-reference/connections.md).

## Status fields

On documents, prefer `display_status` and `ui_phase` over raw `status` for UI badges. Task terminal success is **`indexed`**, not `completed`. Document success is often `completed` (also reported as `indexed`). See [lifecycle](../api-reference/extended-api.md#lifecycle).

## OpenAPI

Interactive docs: `http://localhost:8080/swagger-ui/`. Snapshot: [`openapi.snapshot.json`](../../edgequake_webui/openapi/openapi.snapshot.json) (v0.32.2; Connections are only in HEAD until the next snapshot).
