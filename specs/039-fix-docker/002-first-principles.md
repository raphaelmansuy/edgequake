# SPEC-039 — First Principles

---

## P1 — What is an AGE graph?

An AGE graph is a PostgreSQL schema with:

1. Catalog row in `ag_catalog.ag_graph`
2. Parent tables `_ag_label_vertex`, `_ag_label_edge`
3. **Child label tables** per label name (`"Node"`, `"EDGE"`)

**Truth:** `create_graph()` alone delivers (1) and (2) only.

---

## P2 — What does EdgeQuake assume?

| Operation | Table used | Created by |
| --------- | ---------- | ---------- |
| `pg_get_nodes_batch` | `{graph}."Node"` | `create_vlabel` |
| `pg_upsert_nodes_batch_native` | `{graph}."Node"` INSERT | `create_vlabel` |
| `pg_get_edges_for_nodes_batch` | `{graph}."EDGE"` | `create_elabel` |
| Cypher `MERGE (n:Node …)` | creates label lazily | first MERGE |

**Truth:** Read-before-write merge order requires labels **before** first `get_nodes_batch`.

---

## P3 — When must labels exist?

At `pg_initialize()` completion — before any API traffic or worker task.

**Invariant (SPEC-039):**

```text
∀ fresh graph G: after pg_initialize(G),
  to_regclass('G.Node') IS NOT NULL ∧
  to_regclass('G.EDGE') IS NOT NULL
```

---

## P4 — Idempotency

`create_vlabel` / `create_elabel` are not safely re-invoked on existing labels. Bootstrap must:

1. `EXISTS` check on `pg_class`
2. Create only if missing
3. Treat "already exists" races as success

---

## P5 — Provider independence

Graph bootstrap is storage-layer only. Mistral, Ollama (`gemma4:e4b`), and OpenAI paths share the same merge → persist pipeline. Fixing label bootstrap unblocks **all** Docker LLM configurations.
