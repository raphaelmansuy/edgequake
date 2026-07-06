# SPEC-044 — Code is Law

Every claim maps to live source (baseline: **v0.14.1** + production Graylog 2026-07-06).

---

## Failure surface (compensation)

| Claim | File | Symbol / lines |
| ----- | ---- | -------------- |
| Merge `errors > 0` triggers compensation | `ingestion_persister.rs` | `persist_processing_result` ~343–357 |
| Compensation calls `delete_node` per artifact | `compensation.rs` | `compensate_orphan_graph_writes` ~97–106 |
| Quarantine log message matches Graylog | `compensation.rs` | `"quarantine: failed to roll back orphan node"` |
| `delete_node` → `pg_delete_node` | `graph_storage_impl.rs` | `delete_node` |
| Parameterized Cypher delete | `nodes_ops.rs` | `pg_delete_node` ~230–233 |

---

## Broken Cypher binding (regression)

| Claim | File | Evidence |
| ----- | ---- | -------- |
| Third arg inlined as `'…'::agtype` | `cypher_exec.rs` | `cypher_execute_bound` ~78–80 |
| Read path same pattern | `cypher_exec.rs` | `cypher_query_bound` ~53–55 |
| Comment incorrectly claims inline is required | `cypher_exec.rs` | module doc ~5–10 |
| Uses `sqlx::raw_sql` (no `$1` bind) | `cypher_exec.rs` | `cypher_execute_bound` ~85 |
| Pre-v0.14.0 used `$1::agtype` + `.bind(params)` | git `c2dfe7ae^` | `cypher_bound_sql` |

**Git evidence (v0.14.0 #278 commit chain):**

1. `fix(storage): bind Cypher params as text::ag_catalog.agtype`
2. `fix(storage): AGE Cypher params as bare $1 — no cast expression`
3. `fix(storage): bind Cypher params as text with $1::agtype cast`
4. `fix(storage): inline agtype literal for AGE Cypher parameter binding` ← **shipped**
5. `ci: allow AGE batch contract test failure (pre-existing AGE compat)` — acknowledges inline literals rejected

---

## Merge error source

| Claim | File | Symbol |
| ----- | ---- | ------ |
| Global merge increments errors on batch `Err` | `merger/mod.rs` | ~403–411, ~461–465 |
| Per-entity build failure increments errors | `merger/entity.rs` | ~200–208 |
| New nodes tracked in artifacts before upsert | `merger/entity.rs` | `build_entity_node_batch_entry` ~238 |
| Relationship placeholders also in artifacts | `merger/relationship.rs` | ~114–116 |
| Batch upsert uses native SQL / inline Cypher | `nodes_ops.rs` | `pg_upsert_nodes_batch`, `pg_upsert_nodes_batch_native` |

---

## Safe paths (not implicated in Graylog error)

| Claim | File | Symbol |
| ----- | ---- | ------ |
| `get_nodes_batch` native SQL | `nodes_ops.rs` | `pg_get_nodes_batch` |
| Label bootstrap on upgrade (no-op if exists) | `graph_lifecycle.rs` | `ensure_age_label` EXISTS check |
| SPEC-039 fix present | `graph_lifecycle.rs` | `ensure_graph_labels` in `create_graph` |

---

## Other consumers of `cypher_*_bound` (same bug class)

| Operation | File |
| --------- | ---- |
| `pg_has_node` | `nodes_ops.rs` |
| `pg_get_node` | `nodes_ops.rs` |
| `pg_has_edge` | `edges_ops.rs` |
| `pg_get_edge` | `edges_ops.rs` |
| `pg_delete_edge` | `edges_ops.rs` |

---

## Production log correlation fields

| Field | Interpretation |
| ----- | -------------- |
| `document_id=f78341cd-…` | Failed ingest document UUID |
| `node_id=C1236` | Normalized entity graph key (`EntityId::as_graph_node_id`) |
| `merge_cause=1 knowledge-graph merge error(s)` | `stats.errors == 1` in persister |
| `cleanup_error=… third argument…` | AGE Mode C rejection |

**Correlate primary:** search same `document_id` for `merge_entities_batch_global` or `merge_relationships_batch_global` WARN lines.

---

## External AGE contract (SSOT)

| Source | Rule |
| ------ | ---- |
| [AGE Prepared Statements](https://age.apache.org/age-manual/master/advanced/prepared_statements.html) | Third arg must be `$1` inside PREPARE |
| [apache/age#315](https://github.com/apache/age/issues/315) | Literal map string → `must be a parameter` |
| `zz-reference/002-apache-age/` | Local mental model copy |

---

## Relational schema artifact

| Claim | File |
| ----- | ---- |
| Post-migration `public` DDL dump | `edgequakeSchema.sql` (2392 lines) |
| Does **not** include AGE graph namespace | No `ag_catalog` / `eq_*_graph` tables in dump |

---

## Downstream callers of broken bound ops (C-6)

Full matrix: [006-similar-issues-audit.md §4](./006-similar-issues-audit.md#4-downstream-production-paths-c-6).

| Caller | File | Broken calls |
| ------ | ---- | ------------ |
| Saga compensation | `compensation.rs` | `delete_node`, `delete_edge` |
| Document deletion | `orchestrator/deletion.rs` | `delete_node`, `delete_edge` |
| Entity reconcile | `entity_reconcile.rs` | `get_node`, `get_edge`, `delete_node`, `delete_edge` |
| Entity API lookup | `entity_graph_lookup.rs` | `get_node` |
| Entity handlers | `handlers/entities/entity_ops.rs` | `get_node` (via resolve); `delete_node_scoped` ✅ |
| Query enrichment | `orchestrator/query_ops.rs` | `get_node` |
| Isolation | `handlers/isolation.rs` | `get_node` |

---

## CI masks known failure (C-7)

| Claim | File | Line |
| ----- | ---- | ---- |
| `continue-on-error: true` on graph batch contracts | `.github/workflows/postgres-integration.yml` | ~268 |
| Comment: "AGE rejects inline agtype" | same | ~268 |
| `continue-on-error: true` on AGE graph tests | same | ~286 |

---

## Static test enforces broken pattern (C-8)

| Claim | File |
| ----- | ---- |
| `assert!(src.contains("::agtype"))` | `spec022_cypher_prepared_postgres.rs` L85 |
| spec022 integration skips without `POSTGRES_PASSWORD` | `spec022_cypher_prepared_postgres.rs` L17-19 |
| Skip message incorrectly says `DATABASE_URL` | same L18 |

---

## Safe paths (explicit non-regression)

| Claim | File |
| ----- | ---- |
| Merge `get_nodes_batch` native SQL | `nodes_ops.rs` `pg_get_nodes_batch` |
| Ingest upsert batch UNWIND / native | `nodes_ops.rs`, `edges_ops.rs` |
| Scoped deletes inline Cypher | `pg_delete_node_scoped`, `pg_delete_edge_scoped` |
| Workspace clear inline 2-arg cypher | `analytics_ops.rs` |
