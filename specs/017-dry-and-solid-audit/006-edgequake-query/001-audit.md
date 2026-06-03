# edgequake-query — DRY & SOLID Audit

**Crate path:** `edgequake/crates/edgequake-query`  
**LOC:** ~10,105 (src) | `sota_engine/` ~4,319 LOC  
**Role:** Production query engine (SOTA), legacy `QueryEngine`, orphaned strategies

---

## Executive Summary

Production API uses `SOTAQueryEngine`. The crate contains **three parallel query implementations**: SOTA (~4.3k LOC), legacy `QueryEngine` (769 LOC), and dead `strategies/` (~900 LOC, bench-only). The SOTA **entry pipeline is copy-pasted 3×** across basic/stream/workspace files (~1,900 duplicate LOC). This is the highest LOC duplication in the workspace.

---

## DRY Violations

| ID | P | Violation | Evidence | Remediation |
|----|---|-----------|----------|-------------|
| QUERY-DRY-001 | **P0** | Triple query stack with core | Core `query/*` ~1,145 LOC parallel to this crate | Deprecate core engine; single path |
| QUERY-DRY-002 | **P0** | SOTA entry pipeline 3× copy | `query_entry/query_basic.rs` (801), `query_stream.rs` (570), `query_workspace.rs` (516) | Extract `QueryPipeline::run()` |
| QUERY-DRY-003 | **P1** | Dead `strategies/` module | `create_strategy` only in `strategies/mod.rs` + benches; `engine.rs:491-607` inlines retrieval | Wire strategies OR delete ~900 LOC |
| QUERY-DRY-004 | **P1** | Default vs workspace retrieval duplicated | `query_modes.rs` vs `vector_queries.rs` (~589 LOC) | `RetrievalContext { vector_storage, tenant_filter }` param |
| QUERY-DRY-005 | **P1** | Keyword extraction duplicated with core | `keywords/llm_extractor.rs` (521) vs `core/keyword_extractor.rs` (~362) | Single module in query; core re-exports |
| QUERY-DRY-006 | **P2** | Tenant filter logic 4× | `engine.rs:502`, `sota_engine/prompt.rs:17/47`, core `query/mod.rs:64` | Shared `matches_tenant` in core types |
| QUERY-DRY-007 | **P2** | Dual config structs | `QueryEngineConfig` vs `SOTAQueryConfig` — same defaults | Merge or map once |
| QUERY-DRY-008 | **P2** | Dual type systems | `QueryMode` (no Bypass) vs core (has Bypass); `QueryContext` vs core equivalents | Canonical types in core |
| QUERY-DRY-009 | **P3** | Unused `edgequake-core` dep | `Cargo.toml` lists core; no imports in src | Remove or use for shared types |

---

## SOLID Violations

| ID | P | Principle | Violation | Evidence |
|----|---|-----------|-----------|----------|
| QUERY-SOLID-S-001 | **P1** | SRP | `SOTAQueryEngine` god object | 9 files, 8 impl blocks: retrieval, validation, rerank, prompt, streaming |
| QUERY-SOLID-O-001 | **P1** | OCP | New mode → edit query_modes + vector_queries + each query_entry file | Hybrid in two places |
| QUERY-SOLID-L-001 | **P0** | LSP | API holds both engines; different semantics | `state/mod.rs:152-155`; Ollama uses legacy |
| QUERY-SOLID-L-002 | **P0** | LSP | Core orchestrator can't use SOTA (cycle) | Comment at core `orchestrator/mod.rs:103` |
| QUERY-SOLID-I-001 | **P2** | ISP | `QueryStrategy` trait unused by production paths | `strategies/config.rs:51` |

---

## sota_engine Structure

```text
sota_engine/                    ~4,319 LOC
├── mod.rs              558   config, struct, constructors
├── query_modes.rs      763   local/global/hybrid/naive/mix
├── vector_queries.rs   589   workspace variants (duplicate logic)
├── query_entry/
│   ├── query_basic.rs  801   4 entry methods, duplicated pipeline
│   ├── query_stream.rs 570
│   └── query_workspace.rs 516
├── reranking.rs        228   validate_keywords + rerank (mixed SRP)
└── prompt.rs           284
```

**Root cause:** File splitting reduced individual file size but not algorithm duplication.

---

## Remediation Plan

### P0

1. Route ALL API + orchestrator paths through `SOTAQueryEngine`
2. Extract `QueryPipeline` — one `async fn run(request, overrides) -> Response`
3. Break core↔query cycle via `edgequake-query-types` or moving shared types to core

### P1

4. Delete or wire `strategies/` (~900 LOC savings)
5. Merge `query_modes` + `vector_queries` via shared retrieval context
6. Unify `QueryMode`, `QueryContext`, `QueryParams` with core

### P2/P3

7. Consolidate keyword extraction; remove dead core dep
8. Split `validate_keywords` from `reranking.rs`
9. Migrate Ollama handlers to `sota_engine`

---

## Verification

```bash
cargo test -p edgequake-query --lib
cargo test -p edgequake-query --test keyword_validation_tests
# After QueryPipeline extraction: LOC in query_entry/ should drop ~60%
```

**Acceptance:** One production query path; grep shows no `QueryEngine::retrieve_context` in api handlers except deprecated Ollama routes.
