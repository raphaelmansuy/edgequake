# 004 — QueryContextService Architecture

**Spec:** 028-edgequake-query-service  
**Date:** 2026-06-28  
**Cross-ref:** [003-code-is-law-current-pipeline.md](./003-code-is-law-current-pipeline.md) | [005-dto-model-contract.md](./005-dto-model-contract.md)  
**Crates:** `edgequake-api` (service), `edgequake-query` (engine)

---

## Service Boundary

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         edgequake-api                                    │
│                                                                          │
│  ┌────────────────┐   ┌─────────────────────┐   ┌──────────────────┐ │
│  │ HTTP Handlers  │   │ QueryContextService │   │ QueryGeneration  │ │
│  │ /query/context │──►│ (retrieval SSOT)    │◄──│ Service          │ │
│  │ /query         │   │                     │   │ (LLM answer)     │ │
│  │ /chat/*        │   └──────────┬──────────┘   └────────▲─────────┘ │
│  │ MCP adapter    │              │                        │           │
│  └────────────────┘              │                        │           │
│                                  │ ContextBundle          │           │
│                                  └────────────────────────┘           │
│                                             │                          │
└─────────────────────────────────────────────┼──────────────────────────┘
                                              │
                                              v
                              ┌───────────────────────────┐
                              │ edgequake-query           │
                              │ QueryEngine               │
                              │ run_query_pipeline        │
                              │ (context_only=true)       │
                              └───────────────────────────┘
```

---

## Module Layout

```
edgequake-api/src/services/
├── query_execution.rs          # existing — becomes thin wrapper
├── query_context.rs            # NEW — QueryContextService
├── query_generation.rs         # NEW — prompt + LLM (phase 3)
├── context_bundle_mapper.rs    # NEW — QueryContext → ContextBundle
└── mod.rs                      # re-exports

edgequake-api/src/handlers/
├── query/
│   ├── context_retrieve.rs     # NEW — POST /query/context
│   ├── query_execute.rs        # refactored — calls services
│   └── ...
└── mcp/                        # NEW (phase 5)
    └── tools.rs
```

---

## QueryContextService API (Rust)

```rust
/// Retrieval-only SSOT. Never calls answer LLM (keyword LLM optional).
pub struct QueryContextService {
    state: AppState,
}

impl QueryContextService {
    /// Primary retrieval entry — returns agent-grade ContextBundle.
    pub async fn retrieve(
        &self,
        request: ContextRetrievalRequest,
        auth: &AuthenticatedContext,
    ) -> ApiResult<ContextRetrievalResponse>;

    /// Lightweight search — returns summaries + stable IDs (MCP search).
    pub async fn search(
        &self,
        request: ContextSearchRequest,
        auth: &AuthenticatedContext,
    ) -> ApiResult<ContextSearchResponse>;

    /// Fetch full bundle by retrieval_id (MCP fetch).
    pub async fn fetch(
        &self,
        retrieval_id: &str,
        auth: &AuthenticatedContext,
    ) -> ApiResult<ContextRetrievalResponse>;

    /// Map engine context — internal, also used by generation service.
    pub fn map_bundle(
        &self,
        engine_response: &QueryResponse,
        options: &MappingOptions,
    ) -> ContextBundle;
}
```

---

## retrieve() Flow

```
  ContextRetrievalRequest
           │
           ▼
  ┌────────────────────────────┐
  │ 1. validate + auth scope   │  workspace_id, tenant RLS
  └─────────────┬──────────────┘
                ▼
  ┌────────────────────────────┐
  │ 2. resolve document_filter │  document_filter_resolver (SSOT)
  └─────────────┬──────────────┘
                ▼
  ┌────────────────────────────┐
  │ 3. resolve workspace       │  resolve_workspace_query_resources
  │    embedding + vector      │
  └─────────────┬──────────────┘
                ▼
  ┌────────────────────────────┐
  │ 4. build EngineQueryRequest│  context_only=true ALWAYS
  │    + allowed_document_ids  │
  └─────────────┬──────────────┘
                ▼
  ┌────────────────────────────┐
  │ 5. execute_sota_query      │  existing query_execution.rs
  └─────────────┬──────────────┘
                ▼
  ┌────────────────────────────┐
  │ 6. enrich documents        │  KV: titles, mime, created_at
  │    (lineage block)         │  pdf_lineage metadata where avail
  └─────────────┬──────────────┘
                ▼
  ┌────────────────────────────┐
  │ 7. map ContextBundle       │  granularity tier
  │    + agent metadata        │  coverage, fingerprint
  └─────────────┬──────────────┘
                ▼
  ┌────────────────────────────┐
  │ 8. optional cache store    │  retrieval_id → bundle (MCP fetch)
  └─────────────┬──────────────┘
                ▼
  ContextRetrievalResponse
```

---

## QueryGenerationService (Phase 3)

Thin wrapper — **does not retrieve**:

```rust
pub struct QueryGenerationService;

impl QueryGenerationService {
    pub async fn generate(
        &self,
        bundle: &ContextBundle,
        request: &GenerationRequest,
        llm: Arc<dyn LLMProvider>,
    ) -> ApiResult<GeneratedAnswer>;
}
```

Uses `ContextBundle::to_context_string()` (ported from `QueryContext::to_context_string`) for prompt building.

---

## Dependency Rules (SOLID)

| Rule | Enforcement |
|------|-------------|
| **S** — Single responsibility | `QueryContextService` never calls answer LLM |
| **O** — Open for extension | `MappingOptions` for granularity tiers |
| **L** — Liskov | All handlers get same bundle shape |
| **I** — Interface segregation | `search` vs `fetch` vs `retrieve` methods |
| **D** — Dependency inversion | Service depends on `QueryEngine` trait boundary |

---

## Caching Strategy

| Cache | Key | TTL | Scope |
|-------|-----|-----|-------|
| Engine result cache | query+mode+filter+workspace | config | existing `query_result_cache.rs` |
| Retrieval ID cache | `retrieval_id` | 15 min default | new — for MCP fetch |
| Search index | query hash | 5 min | optional — search summaries only |

**MCP statelessness:** `retrieval_id` is an **explicit handle** returned in search results; client passes it to fetch. No `Mcp-Session-Id` ([MCP 2026-07-28](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/)).

---

## Observability

Extend existing query spans (SPEC-018):

```
  query_context.retrieve
    ├── workspace_id
    ├── mode
    ├── granularity
    ├── chunks_count
    ├── entities_count
    ├── relationships_count
    ├── is_truncated
    ├── coverage_score
    └── retrieval_fingerprint
```

Metric: `edgequake_query_context_retrieval_duration_ms` histogram.

---

## Error Mapping

Reuse `ApiError` from query handlers:

| Engine error | HTTP | Agent hint |
|--------------|------|------------|
| Empty workspace | 404 | `"workspace_not_found"` |
| No vector storage | 503 | `"retrieval_unavailable"` |
| Embedding failure | 502 | `"embedding_failed"` |
| Empty context (valid) | 200 | `coverage_score: 0.0` — not an error |
| Auth failure | 401/403 | unchanged (SPEC-027) |

Empty retrieval is **success with low coverage** — agents decide to broaden query.
