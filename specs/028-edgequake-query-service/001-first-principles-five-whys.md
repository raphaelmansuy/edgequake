# 001 — First Principles & Five Whys

**Spec:** 028-edgequake-query-service  
**Date:** 2026-06-28  
**Cross-ref:** [000-index.md](./000-index.md) | [012-code-is-law-verdict.md](./012-code-is-law-verdict.md)  
**Crates:** `edgequake-query`, `edgequake-api`, `edgequake-core`

---

## Five Whys

### Why 1 — Why do we need a separate Query Context Service?

**Because** external AI agents conducting Agentic Search must retrieve structured knowledge **without** being forced through LLM answer generation on every step.

**Evidence:** Today agents call `POST /api/v1/query` with `context_only: true`. That works as a hack but returns a **generation-oriented** DTO (`QueryResponse` with empty `answer`, flat `sources[]`, 200-char snippets) rather than an **agent-oriented** context bundle.

---

### Why 2 — Why is `context_only` on `/query` insufficient?

**Because** retrieval and generation have **different contracts, SLAs, and consumers**.

| Dimension | Retrieval (Agent) | Generation (User Q&A) |
|-----------|-------------------|------------------------|
| Primary output | Structured subgraph + full chunks | Natural language answer |
| Latency budget | Low; may run 3–10× per agent loop | Higher; includes LLM tokens |
| Token payload | Full chunk text for reasoning | Snippets + citations for UI |
| Structure | Graph-native (entities, edges, provenance) | Flat citations for display |
| Caching | Safe to cache by query+mode+filter | Must not cache LLM output blindly |
| Auth/rate limit | Tool-tier (higher volume) | User-tier |

Coupling both into one endpoint violates **Single Responsibility** (SOLID-S) and forces agents to parse a response shape designed for chat UI.

---

### Why 3 — Why must the DTO be redesigned, not just aliased?

**Because** the current `SourceReference[]` **destroys information** the engine already has:

1. **Graph structure lost** — entities and relationships are flattened into one array; agent cannot traverse edges without reconstruction.
2. **Content truncated** — `snippet` is ~200 chars; agents need full chunk bodies for multi-hop reasoning.
3. **Lineage incomplete** — `source_chunk_ids` on entities exists in engine (`RetrievedEntity`) but is optional/inconsistent in API mapping.
4. **No agent signals** — no coverage estimate, truncation reason, mode arm breakdown, or suggested follow-up queries.
5. **Dead field** — `include_references` on `QueryRequest` is defined but **never read** in handlers.

The engine `QueryContext` (`edgequake-query/src/context.rs`) is the rich truth; HTTP strips it.

---

### Why 4 — Why a service layer instead of only a new handler?

**Because** three consumers already duplicate retrieval orchestration:

```
  query_execute.rs ──────┐
  query_stream.rs ───────┼──► execute_sota_query ──► run_query_pipeline
  chat/mod.rs ───────────┘
         │
         └── each re-implements: workspace resolve, document_filter,
             context → SourceReference mapping (DRY violation)
```

A **`QueryContextService`** becomes the **only** place that:
- Resolves workspace embedding/vector/LLM-for-keywords
- Runs prepare → retrieve → enrich
- Maps `QueryContext` → `ContextBundle` DTO
- Applies document filter SSOT (SPEC-005, SPEC-027 phase 18)

Generation handlers then call: `context = service.retrieve(...)` → `generation = service.generate(context, ...)`.

---

### Why 5 — Why now (June 2026)?

**Because** the industry converged on **Agentic RAG** as the orchestration layer over specialized retrievers:

- **LightRAG** (EdgeQuake's model) handles dual-level graph+vector retrieval.
- **GraphRAG** handles corpus-wide thematic queries via community summaries (EdgeQuake partial).
- **Agentic RAG** treats retrievers as **tools** in a reason-act-observe loop ([Medium 2026](https://medium.com/@Micheal-Lanham/pipeline-rag-vs-agentic-rag-vs-knowledge-graph-rag-what-actually-works-and-when-47a26649a457)).
- **MCP 2026-07-28 RC** standardizes stateless `search`/`fetch` tools for exactly this pattern ([MCP Blog](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/)).

EdgeQuake has production-grade retrieval (SPEC-024 verified). The missing piece is **exposing it as an agent-native product** — not rebuilding retrieval.

---

## First-Principles Invariants (FP-028)

| ID | Invariant | Rationale |
|----|-----------|-----------|
| **FP-028-01** | **Retrieval ≠ Generation** | Separate services, separate endpoints, separate DTOs |
| **FP-028-02** | **Code is law** | `run_query_pipeline` phases are SSOT; service wraps, never reimplements |
| **FP-028-03** | **Structure preserved** | Agents receive graph + chunks + documents as distinct typed sections |
| **FP-028-04** | **Provenance is mandatory** | Every item links to document_id, chunk_id, or graph source |
| **FP-028-05** | **Fail closed on scope** | Workspace + document_filter + RLS identical to `/query` (SPEC-027) |
| **FP-028-06** | **Ascending compatibility** | `/query` + `context_only` deprecated, not removed, for 2 releases |
| **FP-028-07** | **Stateless at MCP boundary** | No session state; explicit handles in tool arguments (MCP 2026-07-28) |
| **FP-028-08** | **Truncation is transparent** | Agent must know what was cut and why (`is_truncated`, `dropped_counts`) |
| **FP-028-09** | **Graph exposed on query results** | `subgraph` on `/query`, stream, chat — see [014-graph-exposure-first-principles.md](./014-graph-exposure-first-principles.md) |
| **FP-028-10** | **Graph mapping SSOT** | `context_bundle_mapper` only — handlers never re-parse flat sources |

---

## Design Tension Resolution

```
                    ┌─────────────────────────────────────┐
                    │         Design Tensions             │
                    ├─────────────────────────────────────┤
                    │ Rich DTO  vs  Payload size          │
                    │ Full text vs  Token budget          │
                    │ Graph structure vs  Flat citations  │
                    │ Agent metadata vs  Simplicity       │
                    └─────────────────────────────────────┘
                                    │
                    Resolution: tiered response via
                    `content_granularity` request field
                    (see 005-dto-model-contract.md)
```

| Tier | Use case | Chunk content | Graph |
|------|----------|---------------|-------|
| `citation` | UI, logging | snippet (200 chars) | **structured `subgraph`** (descriptions truncated) |
| `agent` | external agent default | full text | structured subgraph (full descriptions) |
| `debug` | prompt inspection | full + `context_string` | full + raw metadata |

---

## Success Criteria

1. External agent can conduct 3-step Agentic Search using only `/query/context` — no `/query` with `context_only`.
2. `/query` answer path calls same service — zero duplicate retrieval logic in handlers.
3. MCP `edgequake_search` returns IDs; `edgequake_fetch` returns full `ContextBundle` for one ID.
4. Contract tests prove parity: same retrieval output for `/query/context` and `/query?context_only=true` (citation tier).
5. OpenAPI documents new endpoint with JSON Schema 2020-12 (MCP SEP-2106 aligned).
