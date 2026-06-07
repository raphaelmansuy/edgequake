# edgequake-storage — Observability Audit

**Path:** `edgequake/crates/edgequake-storage`  
**Tracing macros (src):** ~37  
**Role:** KV, vector, graph adapters (Postgres primary)

---

## Executive Summary

Postgres adapters log **graph lifecycle and migration** at info/warn — valuable for ops. **No sqlx/OpenTelemetry integration** — DB latency invisible in traces.

Memory adapters nearly silent (1 debug in `workspace_vector.rs`).

---

## Findings

| ID | P | Finding | Evidence | Remediation |
|----|---|---------|----------|-------------|
| STORE-OBS-001 | P1 | No DB span instrumentation | Postgres modules | `tracing-sqlx` or manual spans on slow queries |
| STORE-OBS-002 | P2 | Graph ops log without timing | `graph/query_ops.rs` | `debug!(duration_ms)` |
| STORE-OBS-003 | P2 | RLS warnings | `postgres/rls.rs` | Good — keep WARN |
| STORE-OBS-004 | P2 | Vector migration info | `vector/migration.rs` | Good for upgrades |
| STORE-OBS-005 | P3 | Test println noise | Multiple `tests/*` | OK |

---

## Adapter Coverage

| Adapter | Logging | OTEL target |
|---------|---------|-------------|
| postgres/kv | minimal | span per batch upsert |
| postgres/vector | search_tuning debug | span per search |
| postgres/graph | query_ops debug | span per cypher |
| memory/* | almost none | debug parity optional |

---

## Verify

```bash
rg 'tracing::' edgequake/crates/edgequake-storage/src -c
```
