# SPEC-028 — EdgeQuake Query Context Service (Agentic Search Foundation)

**Spec:** `028-edgequake-query-service`  
**Date:** 2026-06-28  
**Method:** Code is law — [012-code-is-law-verdict.md](./012-code-is-law-verdict.md) is authoritative.  
**Scope:** Retrieval-only service layer + API + future MCP + DRY refactor of generation pipeline  

**Cross-ref:** [SPEC-024](../024-egdequake-audit/003-query-retrieval-audit.md) · [SPEC-017](../017-dry-and-solid-audit/006-edgequake-query/001-audit.md) · [SPEC-027](../027-api-edgequake-audit/000-index.md) · [SPEC-021](../021-storage-study/03-pipelines/02-query-pipeline.md)

---

## Executive Verdict

EdgeQuake already **retrieves** world-class LightRAG-style context (chunks + graph entities/relationships + lineage). It **generates** answers in the same HTTP call. External agents and future Agentic Search need the **retrieval product** as a first-class, structured service — not a boolean flag on a generation endpoint.

| Claim | Reality (code-verified) |
|-------|-------------------------|
| "Retrieval exists" | **True** — `run_query_pipeline` prepare → retrieve → finalize |
| "Retrieval is exposed for agents" | **False** — `context_only` returns flat `sources[]` with 200-char snippets; engine `QueryContext` is stripped |
| "One SSOT for query paths" | **Partial** — engine pipeline is unified; API mapping duplicated in `query_execute.rs` + `chat/mod.rs` |
| "Agentic Search ready" | **False** — no structured subgraph DTO, no MCP tools, no agent metadata |
| "MCP compatible" | **Not yet** — needs stateless `search`/`fetch` tools per 2026-07-28 MCP RC |

**Bottom line:** Introduce **`QueryContextService`** — retrieval SSOT consumed by `/query/context`, existing `/query` (refactored), `/chat/completions`, future MCP, and native Agentic Search mode.

---

## Problem Statement (One Sentence)

Pipeline RAG answers questions in one shot; **Agentic Search** requires a **machine-readable retrieval surface** that agents can call repeatedly, inspect, and compose — EdgeQuake must expose its LightRAG retrieval engine as that surface without coupling it to LLM generation.

---

## Document Map

| Doc | Purpose | Authority |
|-----|---------|-----------|
| **[012-code-is-law-verdict.md](./012-code-is-law-verdict.md)** | **Authority** — supersedes all 028 lenses | ✅ |
| [001-first-principles-five-whys.md](./001-first-principles-five-whys.md) | 5 Whys + FP invariants | Lens |
| [002-agentic-search-paradigm-lens.md](./002-agentic-search-paradigm-lens.md) | Agentic Search vs LightRAG/GraphRAG/RAG | Lens |
| [003-code-is-law-current-pipeline.md](./003-code-is-law-current-pipeline.md) | Current query pipeline (code truth) | Lens |
| [004-context-service-architecture.md](./004-context-service-architecture.md) | `QueryContextService` design | Lens |
| [005-dto-model-contract.md](./005-dto-model-contract.md) | Agent-grade DTO SSOT | Lens |
| [006-api-surface-lens.md](./006-api-surface-lens.md) | REST endpoints + OpenAPI | Lens |
| [007-mcp-exposure-lens.md](./007-mcp-exposure-lens.md) | MCP 2026-07-28 tool design | Lens |
| [008-dry-refactor-generation-lens.md](./008-dry-refactor-generation-lens.md) | Unify LLM answer path via service | Lens |
| [009-edge-cases-invariants.md](./009-edge-cases-invariants.md) | Edge cases + failure modes | Lens |
| [010-cross-reference-matrix.md](./010-cross-reference-matrix.md) | QRY-xxx finding matrix | Tracking |
| [011-implementation-plan-phases.md](./011-implementation-plan-phases.md) | Phased delivery + tests | Execution |
| [014-graph-exposure-first-principles.md](./014-graph-exposure-first-principles.md) | Graph exposure on query results (YES) | Lens |

---

## Target Architecture (Summary)

```
  Natural Language Query
           │
           ▼
  ┌────────────────────────────────────────────────────────────┐
  │              QueryContextService (NEW SSOT)                  │
  │  prepare → retrieve → enrich → map → ContextBundle DTO     │
  └───────────────┬──────────────────────┬─────────────────────┘
                  │                      │
       ┌──────────▼──────────┐  ┌────────▼────────────┐
       │ POST /query/context │  │ QueryGenerationSvc  │
       │ MCP search/fetch    │  │ (uses same bundle)  │
       │ Agentic Search loop │  │ POST /query         │
       └─────────────────────┘  │ POST /chat/*        │
                                  └─────────────────────┘
```

---

## Implementation Phases (Planned)

| Phase | Theme | Outcome |
|-------|-------|---------|
| 0 | Spec + DTO freeze | SPEC-028 approved; `ContextBundle` JSON schema locked |
| 1 | `QueryContextService` crate module | `edgequake-api/src/services/query_context.rs` |
| 2 | `POST /api/v1/query/context` | OpenAPI + contract tests `spec028_*` |
| 3 | DRY refactor | `/query` + `/chat` call service; remove duplicate source mappers |
| 4 | Agent metadata | coverage score, suggested_followups, truncation transparency |
| 5 | MCP adapter | stateless `edgequake_search` + `edgequake_fetch` tools |
| 6 | Native Agentic Search mode | in-process agent loop (future) |

---

## Verification Commands (Target)

```bash
cargo test -p edgequake-api --features postgres \
  --test spec028_api_contract --test spec028_context_e2e
cargo test -p edgequake-query --lib
cargo clippy -p edgequake-api --features postgres -- -D warnings
```

---

## Related Specs

| Spec | Relationship |
|------|--------------|
| [024-003](../024-egdequake-audit/003-query-retrieval-audit.md) | Retrieval mode semantics — **do not re-audit** |
| [024-004](../024-egdequake-audit/004-lightrag-expert-lens.md) | LightRAG alignment — EdgeQuake **is** LightRAG-class |
| [024-005](../024-egdequake-audit/005-graphrag-expert-lens.md) | GraphRAG gaps — community summaries N/A |
| [017-006](../017-dry-and-solid-audit/006-edgequake-query/001-audit.md) | Pipeline DRY — extends with service layer |
| [004-system-prompt](../004-system-prompt-query.md) | Generation-only — stays on `/query` |
| [005-filter](../005-filter.md) | Document filter — shared by context service |
| [027](../027-api-edgequake-audit/000-index.md) | Auth/workspace isolation — context service inherits |
