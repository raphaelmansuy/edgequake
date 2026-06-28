# 006 — API Surface Lens

**Spec:** 028-edgequake-query-service  
**Date:** 2026-06-28  
**Cross-ref:** [005-dto-model-contract.md](./005-dto-model-contract.md) | [027-003](../027-api-edgequake-audit/003-rest-design-lens.md)  
**Crates:** `edgequake-api`

---

## New Endpoints

| Method | Path | Purpose | Auth |
|--------|------|---------|------|
| **POST** | `/api/v1/query/context` | Full context retrieval (agent default) | Same as `/query` |
| **POST** | `/api/v1/query/context/search` | Lightweight search → retrieval_ids | Same as `/query` |
| **GET** | `/api/v1/query/context/{retrieval_id}` | Fetch cached bundle by ID | Same as `/query` |
| **GET** | `/api/v1/query/context/artifacts/{type}/{id}` | Fetch document, chunk, or figure by ID | Same as `/query` |

All nested under existing `/api/v1` + workspace auth envelope (SPEC-027).

---

## Route Registration

**File:** `edgequake-api/src/routes.rs`

```rust
// SPEC-028: Query Context Service (Agentic Search foundation)
.route("/query/context", post(context_retrieve::retrieve_context))
.route("/query/context/search", post(context_search::search_context))
.route("/query/context/:retrieval_id", get(context_fetch::fetch_context))
```

OpenAPI tags: `Query Context` (separate from `Query` generation).

---

## POST /api/v1/query/context

### Request

`Content-Type: application/json`

Body: `ContextRetrievalRequest` ([005-dto-model-contract.md](./005-dto-model-contract.md))

### Response `200 OK`

Body: `ContextRetrievalResponse`

### Response Headers

| Header | Value | Notes |
|--------|-------|-------|
| `X-Retrieval-Id` | `ret_...` | Duplicate of body field for convenience |
| `X-Retrieval-Fingerprint` | sha256 | Cache/replay key |
| `Cache-Control` | `private, max-age=300` | When `cached=true` |

### Errors

| Status | Code | When |
|--------|------|------|
| 400 | `INVALID_QUERY` | Empty query |
| 400 | `INVALID_MODE` | Unknown mode string |
| 401 | `UNAUTHORIZED` | No auth (when required) |
| 403 | `FORBIDDEN` | Workspace access denied |
| 404 | `WORKSPACE_NOT_FOUND` | Invalid workspace |
| 503 | `RETRIEVAL_UNAVAILABLE` | Vector/embedding down |

**Note:** Empty context returns **200** with `retrieval_quality.empty_context: true` — not 404.

---

## POST /api/v1/query/context/search

MCP-aligned search — returns summaries only.

### Request

`ContextSearchRequest`

### Response `200 OK`

`ContextSearchResponse`

Designed for OpenAI/ChatGPT deep research `search` tool compatibility ([OpenAI MCP docs](https://developers.openai.com/api/docs/mcp)).

---

## GET /api/v1/query/context/{retrieval_id}

MCP-aligned fetch — returns full bundle.

### Path params

- `retrieval_id` — must start with `ret_`

### Query params

| Param | Default | Notes |
|-------|---------|-------|
| `content_granularity` | `agent` | citation\|agent\|debug |

### Response `200 OK`

`ContextRetrievalResponse`

### Errors

| Status | When |
|--------|------|
| 404 | Unknown or expired retrieval_id |
| 410 | Expired (TTL exceeded) — agent should re-search |

---

## GET /api/v1/query/context/artifacts/{artifact_type}/{artifact_id}

Agent artifact fetch — resolve stable IDs from `ContextBundle` lineage after retrieve.

### Path params

| Param | Values |
|-------|--------|
| `artifact_type` | `document`, `chunk`, `figure`, `markdown`, `pdf` (aliases: `doc`, `drawing`, `image`, `md`) |
| `artifact_id` | Document ID, chunk ID, manifest item ID, or PDF UUID |

### Query params

| Param | Required | Notes |
|-------|----------|-------|
| `document_id` | For `figure` only | Parent document containing the figure |
| `include_content` | No | When `artifact_type=document` or `pdf`, include full markdown body |

### Response `200 OK`

`ContextArtifactResponse` — exactly one of `document`, `chunk`, `figure`, `markdown`, or `pdf` populated.

| Artifact type | Response field | Key fields |
|---------------|----------------|------------|
| `document` | `document` | metadata; with `include_content`: `content`, `markdown`, `pdf_download_path`, `pdf_content_path` |
| `markdown` | `markdown` | `markdown`, `content_source` (`kv` \| `pdf_storage`) |
| `pdf` | `pdf` | `pdf_id`, `file_name`, `download_path`, `content_path`; optional `markdown_content` |
| `chunk` | `chunk` | full text + lineage |
| `figure` | `figure` | VLM text + status |

### Agent workflow

```
POST /query/context  →  bundle with lineage IDs
GET  /query/context/artifacts/chunk/{chunk_id}
GET  /query/context/artifacts/document/{doc_id}?include_content=true
GET  /query/context/artifacts/markdown/{doc_id}
GET  /query/context/artifacts/pdf/{pdf_id}?include_content=true
GET  /query/context/artifacts/figure/{item_id}?document_id={doc_id}
```

**DRY:** Reuses KV manifest/mm-chunk loaders, `document_body_loader` (KV + PDF pipeline markdown), and `verify_document_access` — same isolation as `/documents` and `/chunks`.

**Legacy:** WebUI may continue using `GET /documents/{id}`, `GET /documents/pdf/{id}/download`, `GET /documents/pdf/{id}/content`, `GET /chunks/{id}`, `GET /documents/{id}/lineage`.

---

## Existing Endpoint Changes (Ascending)

### POST /api/v1/query

| Change | Phase | Notes |
|--------|-------|-------|
| Internal refactor | 3 | Calls `QueryContextService` + `QueryGenerationService` |
| **`subgraph` field** | 3+ | Structured entities + relationships (default `include_subgraph=true`) — [014](./014-graph-exposure-first-principles.md) |
| Deprecation header | 3 | `context_only` → `Deprecation: true`, `Link: </api/v1/query/context>` |
| Behavior preserved | 3 | Legacy `sources[]` unchanged |

### POST /api/v1/query/stream

| Change | Phase | Notes |
|--------|-------|-------|
| Context event enriched | 4 | Optional `bundle` field in v3 stream format |
| **`subgraph` on v2** | 4+ | Structured graph on default stream context event when `include_subgraph=true` |
| v1/v2 compat | 4 | v2 adds `subgraph`; v3 opt-in via `stream_format: "v3"` (full bundle) |

---

## OpenAPI Integration

Follow SPEC-027 OpenAPI SSOT pattern:

```
  handlers/context_types.rs     #[derive(ToSchema)]
  handlers/context_retrieve.rs  #[utoipa::path]
  openapi_path_ssot.rs          route registration
  openapi_examples.rs           request/response examples
  build.rs                      SSOT scan — compile error on drift
```

### Example OpenAPI snippet

```yaml
/api/v1/query/context:
  post:
    operationId: retrieveQueryContext
    tags: [Query Context]
    summary: Retrieve structured context for Agentic Search
    description: |
      Returns graph subgraph, full chunks, document lineage, and agent hints.
      Does NOT call the answer LLM. Use POST /query for answer generation.
    requestBody:
      required: true
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ContextRetrievalRequest'
    responses:
      '200':
        description: Structured context bundle
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/ContextRetrievalResponse'
```

---

## REST Design Decisions

| Decision | Rationale | Alternative rejected |
|----------|-----------|---------------------|
| `/query/context` not `/retrieve` | Groups with query family; clear sub-resource | `/retrieve` — orphan from query semantics |
| POST not GET for retrieve | Complex body (filters, history) | GET with long query string |
| Separate search/fetch | MCP + OpenAI pattern | Single endpoint with `?summary=true` |
| retrieval_id in path for GET | Cacheable, bookmarkable | Header-only — worse for MCP url field |

---

## Auth & Isolation (Inherited — No Changes)

```
  Request
     │
     ▼
  ┌─────────────────────┐
  │ require_authenticated│  (when auth enabled — SPEC-027)
  └──────────┬──────────┘
             ▼
  ┌─────────────────────┐
  │ resolve_workspace    │  fail-closed
  └──────────┬──────────┘
             ▼
  ┌─────────────────────┐
  │ tenant RLS context   │  migration 050+
  └──────────┬──────────┘
             ▼
  QueryContextService.retrieve()
```

---

## Rate Limiting (Recommended)

| Endpoint | Limit | Rationale |
|----------|-------|-----------|
| `/query/context` | Same as `/query` | Baseline |
| `/query/context/search` | 2× `/query` | Agents may search frequently |
| `/query/context/{id}` GET | 5× `/query` | Fetch is cheap (cache hit) |

Future: dedicated `agent` tier API key scope (SPEC-027 extension).

---

## Contract Tests (spec028_api_contract.rs)

| Test ID | Assertion |
|---------|-----------|
| QRY-CT-001 | `/query/context` registered in routes + OpenAPI |
| QRY-CT-002 | `content_granularity=agent` returns full chunk content |
| QRY-CT-003 | `subgraph.entities` non-empty for local mode fixture |
| QRY-CT-004 | document_filter reduces bundle documents |
| QRY-CT-005 | search → fetch returns equivalent bundle |
| QRY-CT-006 | legacy `context_only` == `to_legacy_sources(bundle)` |
| QRY-CT-007 | workspace isolation — cross-tenant 403 |
| QRY-CT-008 | empty query → 400 |
| QRY-CT-009 | retrieval_id expires → 410 |
| QRY-CT-010 | `retrieval_fingerprint` stable for same inputs |
