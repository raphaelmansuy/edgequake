# edgequake-query — Observability Audit

**Path:** `edgequake/crates/edgequake-query`  
**Tracing macros (src):** ~30  
**Role:** SOTA query engine, retrieval, reranking, truncation

---

## Executive Summary

Query engine logs **retrieval internals at DEBUG** (`vector_queries.rs`, `reranking.rs`, `truncation.rs`) — correct level for tuning, dangerous at default debug in `main.rs`.

No latency breakdown exported to metrics (mode-level histogram is stubbed at API).

---

## Findings

| ID | P | Finding | Evidence | Remediation |
|----|---|---------|----------|-------------|
| QUERY-OBS-001 | P1 | No query-level INFO summary | `sota_engine/` | One `info!` per query: mode, chunks, ms |
| QUERY-OBS-002 | P2 | Rerank failures at warn | `reranking.rs` | Include `query_id` / parent span |
| QUERY-OBS-003 | P2 | Stream path minimal | `query_stream.rs:1` warn | Mirror execute path fields |
| QUERY-OBS-004 | P3 | Test `println!` heavy | `tests/search_quality_tests.rs` | OK in tests |

---

## SOTA Engine Span Model (target)

```
query_execute (API span)
  ├── retrieve_local
  ├── retrieve_global
  ├── rerank
  ├── truncate
  └── llm_generate (child via edgequake-llm)
```

---

## Verify

```bash
rg 'tracing::debug!' edgequake/crates/edgequake-query/src -c
rg 'tracing::warn!' edgequake/crates/edgequake-query/src
```
