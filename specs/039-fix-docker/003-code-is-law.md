# SPEC-039 — Code is Law

Every claim maps to a live source file (commit baseline: v0.13.0 + SPEC-039 fix).

---

## Graph bootstrap (before fix)

| Claim | File | Lines |
| ----- | ---- | ----- |
| `create_graph()` only calls `ag_catalog.create_graph` | `graph_lifecycle.rs` | `create_graph()` |
| `ensure_indexes()` skips missing `Node` silently | `graph_lifecycle.rs` | `ensure_indexes()` err branch |
| `pg_initialize()` never creates labels | `lifecycle_ops.rs` | `pg_initialize()` |

---

## Failing read path (merge)

| Claim | File | Lines |
| ----- | ---- | ----- |
| Merge calls `get_nodes_batch` before upsert | `merger/entity.rs` | `merge_entities_batch` |
| `pg_get_nodes_batch` queries `"Node"` table | `nodes_ops.rs` | `pg_get_nodes_batch` |

---

## Write paths

| Claim | File | Lines |
| ----- | ---- | ----- |
| Native writes gated by `EDGEQUAKE_NATIVE_GRAPH_WRITES` | `graph/mod.rs` | `native_graph_writes_enabled()` |
| Native upsert INSERTs into `"Node"` | `nodes_ops.rs` | `pg_upsert_nodes_batch_native` |
| Cypher upsert MERGE creates label lazily | `nodes_ops.rs` | `pg_upsert_nodes_batch` |

---

## Fix (v0.13.1)

| Claim | File | Lines |
| ----- | ---- | ----- |
| `ensure_graph_labels()` after `create_graph` | `graph_lifecycle.rs` | `ensure_graph_labels`, `ensure_age_label` |
| Idempotent `pg_class` EXISTS check | `graph_lifecycle.rs` | `ensure_age_label` |
| Calls `create_vlabel` / `create_elabel` | `graph_lifecycle.rs` | `ensure_age_label` |

---

## Docker runtime evidence (v0.13.0 pre-fix)

```text
edgequake-api logs:
  merge_entities_batch_global → relation "eq_eq_default_graph.Node" does not exist
  merge_relationships_batch_global → same

postgres:
  ag_graph row exists; only _ag_label_vertex/_ag_label_edge tables (no Node/EDGE)
```

---

## AGE reference (external SSOT)

Apache AGE manual — labels are explicit DDL:

```sql
SELECT create_vlabel('graph', 'Node');
SELECT create_elabel('graph', 'EDGE');
```

Source: `zz-reference/002-apache-age/002-fundamentals/002-graphs-and-labels.md`
