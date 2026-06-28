# 012 — Code Is Law Verdict (Authority)

**Spec:** 028-edgequake-query-service  
**Date:** 2026-06-28  
**Status:** IMPLEMENTATION FIXED (Phases 1–5) — Phase 6 deferred  
**Supersedes:** All other SPEC-028 lens documents on conflict

---

## Verdict

EdgeQuake **exposes** retrieval as **`QueryContextService`** with agent-grade **`ContextBundle`** DTO via **`POST /api/v1/query/context`**, with generation paths consuming the same source-mapping and engine-request SSOT (DRY). Phases 1–5 are **implemented and verified** (see [013-implementation-assessment.md](./013-implementation-assessment.md)).

Pipeline RAG (`POST /query` with LLM) remains valid for human Q&A. Agentic Search consumers use the context service as a **tool**, not a flag.

---

## Authoritative Truths (Code-Verified 2026-06-28)

| # | Truth | Source |
|---|-------|--------|
| 1 | Query pipeline SSOT is `run_query_pipeline` | `query_pipeline.rs:54` |
| 2 | Rich context is `QueryContext` with chunks/entities/relationships | `context.rs:24` |
| 3 | HTTP strips context to flat `SourceReference[]` only on legacy field; **`subgraph` now structured** | `query_types.rs` |
| 4 | `context_only` skips LLM but uses wrong DTO | `query_pipeline.rs:78,428` |
| 5 | `include_references` is dead | ~~`query_types.rs:103`~~ **FIXED** — wired in query_execute |
| 6 | Source mapping duplicated | ~~query_execute + chat~~ **FIXED** — source_reference_builder SSOT |
| 7 | EdgeQuake is LightRAG-class (Local+Global+Mix) | `modes.rs`, SPEC-024 |
| 8 | Document filter SSOT is scoped metadata scan | SPEC-027 phase 18 |
| 9 | Auth/workspace isolation is mandatory | SPEC-027 phase 35+ |
| 10 | MCP 2026-07-28 requires stateless search/fetch | MCP RC spec |

---

## Target State Architecture (Authoritative)

```
                         ┌─────────────────────────────────────┐
                         │           API Consumers             │
                         ├─────────────┬───────────┬───────────┤
                         │ External    │ WebUI     │ MCP Host  │
                         │ Agent       │ /query    │ Cursor    │
                         └──────┬──────┴─────┬─────┴─────┬─────┘
                                │            │           │
                    POST        │    POST    │   tools/  │
              /query/context    │   /query   │   call    │
                                │            │           │
                                v            v           v
                         ┌──────────────────────────────────────┐
                         │         QueryContextService          │
                         │  validate → filter → retrieve → map  │
                         └──────────────────┬───────────────────┘
                                            │
                              ContextBundle │
                                            v
              ┌─────────────────────────────┴────────────────────────────┐
              │                                                          │
              v                                                          v
   Return bundle to agent                          QueryGenerationService
   (no LLM)                                       (build_prompt + LLM)
```

---

## Approved DTO SSOT

**Request:** `ContextRetrievalRequest` — see [005-dto-model-contract.md](./005-dto-model-contract.md)  
**Response:** `ContextRetrievalResponse` wrapping `ContextBundle`  
**Default granularity:** `agent` (full chunk text + structured subgraph)  
**Legacy compat:** `QueryResponse.sources[]` via `to_legacy_sources(citation)`

No alternative DTO shapes approved without amending this verdict.

---

## Approved API Endpoints

| Endpoint | Phase | Status |
|----------|-------|--------|
| `POST /api/v1/query/context` | 2 | **FIXED** |
| `POST /api/v1/query/context/search` | 2 | **FIXED** |
| `GET /api/v1/query/context/{retrieval_id}` | 2 | **FIXED** |
| `GET /api/v1/query/context/artifacts/{type}/{id}` | 2 | **FIXED** |
| `POST /api/v1/query` (refactored) | 3 | **FIXED** — behavior preserved |
| `POST /api/v1/mcp` | 5 | **FIXED** |
| MCP `edgequake_search` / `edgequake_fetch` | 5 | **FIXED** |

---

## Rejected Alternatives

| Alternative | Reason rejected |
|-------------|-----------------|
| Extend `context_only` only — no new endpoint | Violates FP-028-01; wrong DTO forever |
| Return raw `QueryContext` JSON from engine | Leaks internal types; no agent metadata |
| New `/retrieve` top-level resource | Breaks query family grouping; REST lens |
| Server-side agent loop in phase 2 | Scope creep — phase 6 only |
| GraphRAG community summaries in this spec | Out of scope — see SPEC-024-005 |
| Replace LightRAG modes with single vector | Regresses multi-hop capability |

---

## Finding Dispositions

| ID | Disposition | Phase | Status |
|----|-------------|-------|--------|
| QRY-001 | FIX — unify default to Mix | 3 | FIXED |
| QRY-002 | FIX — QueryContextService | 3 | FIXED |
| QRY-003 | FIX — wire include_references | 3 | FIXED |
| QRY-004..009 | FIX — context service + endpoint | 2–3 | FIXED |
| QRY-010 | FIX — MCP tools | 5 | FIXED |
| QRY-011..013 | FIX — per matrix | 2–4 | FIXED |
| QRY-014 | FIX — bypass rejected | 2 | FIXED |
| QRY-015 | FIX — truncation metadata | 2–4 | PARTIAL |

Full matrix: [010-cross-reference-matrix.md](./010-cross-reference-matrix.md)

---

## Implementation Authority Chain

```
  012-code-is-law-verdict.md  ◄── YOU ARE HERE (wins all conflicts)
           │
           ├── 011-implementation-plan-phases.md (execution order)
           ├── 005-dto-model-contract.md (DTO SSOT)
           ├── 004-context-service-architecture.md (service SSOT)
           └── 003-code-is-law-current-pipeline.md (baseline truth)
```

Lens documents (002, 006, 007, 008, 009) are **advisory** unless echoed in 012.

---

## Verification Gate (Implementation Complete)

```bash
# Verified 2026-06-28 — SPEC-028 Phases 1–5 FIXED
cargo test -p edgequake-api --features postgres \
  --test spec028_api_contract --test spec028_context_e2e --test spec028_mcp_e2e
cargo test -p edgequake-api --features postgres --test spec027_e2e  # no regression
cargo clippy -p edgequake-api --features postgres -- -D warnings
```

---

## Document Map (Quick Links)

| Need | Read |
|------|------|
| Why we're doing this | [001-first-principles-five-whys.md](./001-first-principles-five-whys.md) |
| What is Agentic Search | [002-agentic-search-paradigm-lens.md](./002-agentic-search-paradigm-lens.md) |
| Current code truth | [003-code-is-law-current-pipeline.md](./003-code-is-law-current-pipeline.md) |
| Service design | [004-context-service-architecture.md](./004-context-service-architecture.md) |
| JSON shapes | [005-dto-model-contract.md](./005-dto-model-contract.md) |
| REST routes | [006-api-surface-lens.md](./006-api-surface-lens.md) |
| MCP tools | [007-mcp-exposure-lens.md](./007-mcp-exposure-lens.md) |
| DRY refactor | [008-dry-refactor-generation-lens.md](./008-dry-refactor-generation-lens.md) |
| Edge cases | [009-edge-cases-invariants.md](./009-edge-cases-invariants.md) |
| How to build | [011-implementation-plan-phases.md](./011-implementation-plan-phases.md) |
| Implementation status | [013-implementation-assessment.md](./013-implementation-assessment.md) |
