# P1-12 — Postgres graph `helpers` SRP split

**Date:** 2026-06-03  
**Status:** ✅ Proven

## Change

Monolithic `helpers.rs` (678 LOC) split into `helpers/` directory:

| Module | Responsibility | ~LOC |
|--------|----------------|------|
| `session.rs` | AGE bootstrap SQL, dollar-quote tag, dedicated conn setup | 60 |
| `cypher_exec.rs` | cypher_query / execute / count, batch_sql_query | 109 |
| `age_parse.rs` | agtype → GraphNode/GraphEdge parsing | 104 |
| `cypher_format.rs` | escape + property literal formatting | 53 |
| `graph_lifecycle.rs` | create_graph, ensure_indexes | 182 |
| `mod.rs` | module root + unit tests | 58 |

All modules use `pub(in crate::adapters::postgres::graph)` for cross-submodule access (same visibility as former `pub(super)` from `helpers.rs`).

## Proof

```bash
cd edgequake && cargo check --workspace
cd edgequake && cargo test -p edgequake-storage --features postgres --lib helper_tests
./specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/run_storage_e2e.sh
```

## SOLID

- **SRP:** Session, execution, parsing, formatting, and DDL are separate files.
- **No largest file >200 LOC** in helpers (was 678).

## Remaining P1-12 note

Other graph submodules (`nodes_ops.rs` 521, `query_ops.rs` 611) remain large but are operation-focused, not helper bloat.
