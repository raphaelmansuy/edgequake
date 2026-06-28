# Performance Guarantee Model — SPEC-011

How EdgeQuake **guarantees** (or explicitly does not guarantee) storage performance.

---

## Guarantee tiers

| Tier | Meaning | Enforcement |
| ---- | ------- | ----------- |
| **G1 — Contracted** | Complexity class + CI latency test | Must pass in `e2e_storage_performance_spec011` |
| **G2 — Best effort** | Optimized but no CI SLO | Code review + QUERY_CATALOG |
| **G3 — Unbounded** | O(N); must not be on hot path | Documented ban list |

---

## G1 — Contracted operations (guaranteed)

| Operation | Complexity | SLO (memory test, 5k+ KV rows) | PostgreSQL mechanism |
| --------- | ---------- | ------------------------------ | -------------------- |
| `KVStorage::ping()` | O(1) | < 50 ms | `SELECT 1 … LIMIT 1` |
| `VectorStorage::ping()` | O(1) | < 50 ms | same |
| `GraphStorage::ping()` | O(1) | < 50 ms | `SELECT 1 FROM graph."Node" LIMIT 1` |
| `KVStorage::is_empty()` | O(1) | < 50 ms | `NOT EXISTS (… LIMIT 1)` |
| `GET /health` | 3× ping | < 200 ms | no COUNT on storage |
| `keys_with_prefix(p)` | O(k) k=matches | < 100 ms for single-doc prefix | `LIKE 'p%'` uses PK B-tree |
| `count()` accuracy | O(N) exact | semantics only, **O(1) on PostgreSQL via stats table** | full scan — admin/tests only |

### CI enforcement

```bash
cargo test -p edgequake-api --test e2e_storage_performance_spec011
cargo test -p edgequake-storage --test performance_storage --features postgres
# postgres test runs when POSTGRES_PASSWORD set
```

Failure = regression. Fix or update SLO with documented justification.

---

## G2 — Best effort (optimized, not SLO-bound)

| Operation | Notes |
| --------- | ----- |
| `keys_like('%-metadata')` | Reduces payload; PG may still seq-scan for leading `%` |
| `keys_like('%-chunk-%')` | Same |
| Batch KV upsert | 1000-row chunks; fewer RTTs |
| Shared pool | Prevents connection exhaustion |

---

## G3 — Banned on hot paths (never in probes/polling)

| Operation | Why |
| --------- | --- |
| `count()` for connectivity | O(N) — caused 13s incident |
| `keys()` without filter | O(N) transfer + scan |
| `get_all_nodes()` for dashboards | O(N) graph fetch |
| `node_count()` in health | O(N) |

Static grep gate (manual today, CI candidate):

```bash
rg '\.count\(\)\.await' edgequake/crates/edgequake-api/src/handlers/health.rs  # must be empty
rg 'kv_storage\.keys\(\)' edgequake/crates/edgequake-api/src/handlers/documents/query/  # must be empty after migration
```

---

## Prefix vs suffix — first principle

```
key LIKE 'abc%'   → B-tree range scan     → O(log N + k)  ✅ G1
key LIKE '%-meta' → no index (leading %)  → O(N)          ⚠ G2
```

**Rule**: Document-scoped reads use `keys_with_prefix("{doc_id}-chunk-")`.
Global metadata enumeration uses `keys_like('%-metadata')` until normalized schema exists.

---

## Non-guarantees (explicit)

We do **not** guarantee:

- Sub-second workspace stats on 1M+ chunk tables (graph Cypher + metadata scan)
- PDF list with blob column performance
- Exact `count()` latency
- Vector upsert throughput (still row-by-row)

---

## Rollback of guarantee

If an SLO test flakes in CI:

1. Check seed size vs threshold (debug builds are slower — thresholds use generous multiples)
2. Never disable test without updating this document
3. Never revert ping→count to make test pass

---

## Cross-references

- [BRUTAL_ASSESSMENT.md](./BRUTAL_ASSESSMENT.md) — what phase 1 did not solve
- [QUERY_CATALOG.md](./QUERY_CATALOG.md) — query inventory
- [IMPLEMENTATION_PROOF.md](./IMPLEMENTATION_PROOF.md) — test evidence
