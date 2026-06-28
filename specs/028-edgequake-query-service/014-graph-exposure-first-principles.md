# 014 — Graph Exposure First Principles

**Spec:** 028-edgequake-query-service  
**Date:** 2026-06-28  
**Last verified:** 2026-06-28 (REST + MCP + WebUI graph exposure; code-is-law)  
**Authority:** Lens — defers to [012-code-is-law-verdict.md](./012-code-is-law-verdict.md)  
**Cross-ref:** [001-first-principles-five-whys.md](./001-first-principles-five-whys.md) | [005-dto-model-contract.md](./005-dto-model-contract.md) | [006-api-surface-lens.md](./006-api-surface-lens.md) | [007-mcp-exposure-lens.md](./007-mcp-exposure-lens.md)

---

## Question

When a query returns results, should the API expose the **knowledge graph** (entities + relationships) that matched the query — the same structure the WebUI surfaces in Source Citations and the graph panel?

---

## Verdict: **YES**

Graph exposure is **mandatory** for agent-grade retrieval and **required for WebUI/MCP parity** without lossy client-side reconstruction.

---

## First-Principles Argument

### 1. LightRAG's product IS the graph

EdgeQuake retrieval is not "vector top-K chunks." Local mode retrieves **entity neighborhoods**; Global mode retrieves **relationship themes**; Mix combines both.

Hiding graph structure in a flat `sources[]` array **destroys the retrieval signal** the engine produced. That violates **FP-028-03** (structure preserved).

### 2. Flat citations are a lossy compression, not a feature

The WebUI historically parsed `SourceReference[]` back into entities and relationships (`source-mapper.ts`). The server now exposes structured graph directly; clients prefer `subgraph` and only fall back to flat parsing when absent.

### 3. Agents need edges for multi-hop reasoning

Agentic Search loops plan → retrieve → evaluate. Agents that only see flat citations cannot traverse typed edges, rank by degree, or decide neighborhood expansion vs document artifact fetch.

### 4. Separation of concerns (SOLID)

| Layer | Responsibility |
|-------|----------------|
| `run_query_pipeline` | Retrieve `QueryContext` (engine SSOT) |
| `context_bundle_mapper` | Map engine → `SubgraphBundle` / `ContextBundle` (DTO SSOT) |
| `/query/context`, MCP fetch/retrieve | Return full agent bundle |
| `/query`, `/query/stream`, `/chat/stream` | Citation `sources[]` + structured `subgraph` |
| MCP search | Graph **preview** in `metadata` before fetch |
| WebUI `source-mapper` | Map API subgraph → UI `QueryContext` (no re-parse) |

---

## Design Decision — Tiered Exposure

| Surface | `sources[]` | `subgraph` | `bundle` | Graph preview |
|---------|-------------|------------|----------|---------------|
| `POST /query/context` | — | via `bundle.subgraph` | full | — |
| `POST /query` | citation snippets | **yes** (default) | — | — |
| `POST /query/stream` v2 | citation snippets | **yes** (default) | — | — |
| `POST /query/stream` v3 | citation snippets | via `bundle.subgraph` | full | — |
| `POST /chat/completions/stream` | citation snippets | **yes** (default) | — | — |
| WebUI Source Citations | chunks from sources | entities/relationships from **subgraph** | — | — |
| MCP `edgequake_search` | — | — | — | **`metadata.top_entities`** |
| MCP `edgequake_fetch` | — | via `bundle.subgraph` | full | — |
| MCP `edgequake_retrieve` | — | via `bundle.subgraph` | full | — |

### Request control

| Field | Default | Effect |
|-------|---------|--------|
| `include_subgraph` | `true` | Omit graph when `false` (REST query, context fetch, MCP fetch) |

---

## Consumer Workflows

### MCP agent (graph-aware)

```
edgequake_search  → metadata: entity_count, top_entities[], top_relationships[]
edgequake_fetch   → bundle.subgraph (full multi-hop graph)
artifacts API     → document/chunk/markdown/pdf by lineage IDs
```

### External REST agent

```
POST /query/context              → full ContextBundle
POST /query?context_only=true    → sources[] + subgraph
GET  /query/context/artifacts/…  → deep artifact fetch
```

### WebUI (Source Citations + graph panel)

```
Chat stream context event  →  sources[] + subgraph
buildQueryContextFromRetrieval(sources, subgraph)
  → chunks from sources (citation snippets)
  → entities/relationships from subgraph (typed edges, degree, lineage)
SourceCitations component  →  existing UI, richer graph data
```

**DRY:** WebUI does not parse `"SOURCE->TARGET"` when subgraph is present.

---

## Invariants (FP-028-09)

| ID | Invariant |
|----|-----------|
| **FP-028-09** | Query responses with retrieval MUST expose `subgraph` when `include_subgraph=true` |
| **FP-028-10** | Graph mapping uses `context_bundle_mapper` SSOT only (backend) |
| **FP-028-11** | Chat persistence uses `message_context_mapper` (no lossy re-parse) |
| **FP-028-12** | MCP search includes graph preview; MCP fetch returns `bundle.subgraph` |
| **FP-028-13** | WebUI prefers API `subgraph` over flat source re-inference |

---

## Implementation Evidence (Code-Is-Law)

### Backend (`edgequake-api`)

| Module | Role |
|--------|------|
| `context_bundle_mapper.rs` | `map_query_context_to_subgraph`, `build_search_graph_metadata` |
| `message_context_mapper.rs` | Engine subgraph → chat `MessageContext` |
| `query_context.rs` | `build_query_response_subgraph`, `FetchContextOptions` |
| `query_types.rs` | `QueryResponse.subgraph`, stream `Context.subgraph` |
| `chat_types.rs` | `ChatStreamEvent::Context.subgraph` |
| `mcp/gateway/dispatch.rs` | MCP fetch with `FetchContextOptions` |

### WebUI (`edgequake_webui`)

| Module | Role |
|--------|------|
| `lib/utils/subgraph-types.ts` | API subgraph DTO types |
| `lib/utils/source-mapper.ts` | `buildQueryContextFromRetrieval` — prefers subgraph |
| `hooks/use-query-streaming.ts` | Passes `subgraph` from chat stream to UI context |

### Verification

```bash
# Backend
cargo test -p edgequake-api --features postgres \
  --test spec028_api_contract \
  --test spec028_context_e2e \
  --test spec028_mcp_e2e

# WebUI
cd edgequake_webui && bun test src/lib/utils/__tests__/source-mapper.test.ts
```

| Test | Scope |
|------|-------|
| `ec_query_response_includes_subgraph` | REST `/query` |
| `ec_stream_v2_context_event_includes_subgraph` | Query stream v2 |
| `ec_mcp_search_metadata_includes_graph_preview` | MCP search |
| `ec_mcp_fetch_omits_subgraph_when_disabled` | MCP fetch toggle |
| `spec028_subgraph_mapper_ssot` | Backend contract |
| `buildQueryContextFromRetrieval with subgraph` | WebUI unit |

**Result (2026-06-28):** REST + MCP + WebUI graph paths verified.

---

## Success Criteria — **COMPLETE**

- [x] `POST /query` includes structured subgraph
- [x] Query stream v2 + chat stream include `subgraph`
- [x] Chat persistence uses engine mapper (no lossy re-parse)
- [x] Single backend mapper SSOT shared with `/query/context`
- [x] MCP search preview + MCP fetch full subgraph
- [x] WebUI consumes `subgraph` via `buildQueryContextFromRetrieval`
- [x] Contract + E2E tests (backend) + unit tests (WebUI)

---

## Rejected Alternatives

| Alternative | Why rejected |
|-------------|--------------|
| WebUI-only graph; agents use flat sources | Violates FP-028-03/12 |
| Client-side `"SOURCE->TARGET"` parsing as primary | Lossy; server already has typed edges |
| Separate MCP graph endpoint | Duplicates retrieval orchestration |
| Full workspace graph on every query | Token budget + isolation |
