# SPEC-044 — First Principles

**Question:** After upgrading to v0.14.x, which storage paths can fail ingest, and which are safe?

---

## 1. AGE `cypher()` has three invocation modes

| Mode | SQL shape | Third argument | Used by EdgeQuake | Status on AGE 1.6/1.7 |
| ---- | --------- | -------------- | ----------------- | --------------------- |
| **A — No params** | `cypher(graph, $$ … $$)` | *(none)* | `cypher_execute`, batch MERGE/UNWIND | ✅ Stable |
| **B — Prepared bind** | `cypher(graph, $$ … $name … $$, $1)` | Bare `$1` (PG param) | **Intended** SPEC-022 hot path | ✅ Required contract |
| **C — Inline literal** | `cypher(graph, $$ … $$, '{"k":"v"}'::agtype)` | Constant expression | **Current** `cypher_*_bound` | ❌ Rejected by AGE |

**Invariant (SPEC-044):** Mode C must never ship. Mode B must work on PG16 (AGE 1.6.0) and PG17/PG18 (AGE 1.7.0).

---

## 2. Ingestion persist ordering (SPEC-021)

```
chunk vectors → entity vectors → entity graph batch → rel vectors → rel graph batch
                     │                                      │
                     └─ on any merge errors > 0 ────────────┘
                                    │
                                    ▼
                         compensate_merge_failure
                         (vectors + new nodes + new edges)
```

**Principle:** Compensation is **best-effort** — must not mask the original error, but should succeed when invoked so operators do not inherit orphan graph residue.

---

## 3. Merge hot path vs compensation cold path

| Operation | Implementation | Third-arg Cypher? | Upgrade impact |
| --------- | -------------- | ----------------- | -------------- |
| `get_nodes_batch` | Native SQL on `{graph}."Node"` | No | Safe (SPEC-032) |
| `upsert_nodes_batch` | UNWIND MERGE or native INSERT | No (inline) | Safe |
| `upsert_edges_batch` | Native or Cypher batch | No (inline) | Safe |
| `pg_has_node` / `pg_get_node` | `cypher_query_bound` | **Yes — broken** | Latent until called |
| `pg_delete_node` | `cypher_execute_bound` | **Yes — broken** | Fails on compensation |
| `pg_delete_edge` | `cypher_execute_bound` | **Yes — broken** | Fails on compensation |

**First principle:** Production ingest **usually succeeds** because the hot path avoids Mode B. Failures expose Mode C through compensation.

---

## 4. Merge error counting semantics

`stats.errors` is a **counter**, not a binary:

- Per-entity build failure: +1, other entities still upserted
- Whole batch `Err`: +1 at merger wrapper
- Final: `Ok(stats)` with `errors > 0` still triggers **full** artifact rollback

**Gap (SPEC-044 P2):** Compensation may delete successfully persisted nodes when an unrelated entity failed build — overly aggressive saga scope.

---

## 5. Upgrade-specific pressures (SPEC-042)

On first boot after image bump to v0.14.x:

| Event | Effect on ingest |
| ----- | ---------------- |
| M042 `ALTER EXTENSION vector UPDATE` | Brief catalog lock; `/ready` may 503 |
| M043 `ALTER EXTENSION age UPDATE` | Can be slow on large graphs; idempotent skip on mismatch |
| `bootstrap_concurrent_indexes` | Background; non-blocking for writes |
| `ensure_graph_labels` | No-op on existing graphs (SPEC-039) |

**Principle:** Transient merge failures during extension upgrade window are expected; compensation must still work when they occur.

---

## 6. Schema dump scope

[`edgequakeSchema.sql`](./edgequakeSchema.sql) captures **`public`** relational DDL only.

AGE graph state lives in:

- `ag_catalog.ag_graph`
- `{workspace_graph}."Node"`, `{workspace_graph}."EDGE"`
- `_ag_label_vertex`, `_ag_label_edge` inheritance parents

**Operator gate:** Run [`e2e/sql/post_upgrade_health.sql`](./e2e/sql/post_upgrade_health.sql) — not inferable from `edgequakeSchema.sql` alone.

---

## 7. Correct bind pattern (target state)

```rust
// Serialize map to JSON string; bind as TEXT; third arg bare $1 (no cast).
let params_json = serde_json::to_string(params)?;
let sql = format!(
    "{} SELECT * FROM cypher('{}', {} {} {}, $1) AS (a agtype);",
    setup, graph, tag, cypher, tag
);
sqlx::query(&sql).bind(params_json).execute(pool).await?;
```

**Rejected patterns (v0.14.0 iteration history):**

| Pattern | AGE response |
| ------- | ------------ |
| `$1::agtype` | Cast expression rejected as third arg |
| `.bind(serde_json::Value)` | sqlx sends jsonb; no jsonb→agtype cast |
| `'…'::agtype` inline | `third argument must be a parameter` |
| `sqlx::raw_sql` without prepare | May not establish `$1` param slot |

---

## 8. Fallback pattern (P1 workaround)

Align `pg_delete_node` with `pg_delete_node_scoped`: escaped inline Cypher via `cypher_execute` (Mode A). Injection-safe via `escape_cypher_string`. Use when Mode B fix is not yet deployed.
