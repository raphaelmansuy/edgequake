# SPEC-039 — 5 WHYs: Fresh Docker Install Graph Failure

**Evidence:** Docker E2E on `ghcr.io/raphaelmansuy/edgequake:0.13.0` with Mistral (2026-07-02)

---

## Symptom

User runs [v0.13.0 Docker quickstart](https://github.com/raphaelmansuy/edgequake/releases/tag/v0.13.0), uploads a markdown document, document shows **Failed**. Query returns:

```text
relation "eq_eq_default_graph.Node" does not exist
```

---

## WHY chain

```
WHY #1 — Why does document ingestion fail?
→ Knowledge graph persist reports "2 knowledge-graph merge error(s)"
  (entity batch + relationship batch both failed).

WHY #2 — Why do merge batches fail?
→ merge_entities_batch calls get_nodes_batch() which runs native SQL:
  FROM eq_eq_default_graph."Node" AS n
  Error: relation "eq_eq_default_graph.Node" does not exist

WHY #3 — Why doesn't the Node table exist?
→ pg_initialize() calls create_graph() which only runs ag_catalog.create_graph().
  AGE creates parent tables (_ag_label_vertex, _ag_label_edge) but NOT child
  label tables until first Cypher MERGE/CREATE with :Node / :EDGE.

WHY #4 — Why doesn't the first upsert create the label via Cypher?
→ merge_entities_batch reads BEFORE write (get_nodes_batch). Read fails first;
  upsert_nodes_batch (Cypher MERGE path) never runs. Chicken-and-egg on empty graph.

WHY #5 — ROOT CAUSE
→ SPEC-032/034 optimized graph I/O to native SQL on label child tables, but
  bootstrap only created the graph catalog — not the EdgeQuake-canonical labels
  (Node, EDGE). Fresh installs violate an implicit invariant:
  "label tables exist before any batch SQL".
```

---

## Manual proof (hypothesis validation)

```sql
SELECT create_vlabel('eq_eq_default_graph', 'Node');
SELECT create_elabel('eq_eq_default_graph', 'EDGE');
```

After manual labels: same stack completed ingestion (5 entities) and Mistral query answered correctly.

---

## Not the root cause

| Ruled out | Evidence |
| --------- | -------- |
| Mistral API failure | 8 entities extracted, 1440 tokens, lineage populated |
| Missing migrations | 76/76 applied, health `ready_for_traffic: true` |
| `EDGEQUAKE_NATIVE_GRAPH_WRITES` | Unset in container; Cypher path default |
| Auth misconfiguration | Registration + login + upload 202 succeeded |
