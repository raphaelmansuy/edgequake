# 005 — GraphRAG Expert Lens

**Cross-ref:** [003 Query](./003-query-retrieval-audit.md) · [F-04](./README.md#cross-reference-matrix) · [007 Postgres](./007-postgres-age-pgvector-lens.md)

---

## GraphRAG Core Pattern (reference)

Microsoft GraphRAG pipeline:
1. **Extract** entities/relationships from text
2. **Cluster** graph into communities (Leiden/Louvain)
3. **Summarize** each community → hierarchical reports
4. **Query:**
   - **Local search:** entity neighborhood + community context
   - **Global search:** map-reduce over community summaries

Key distinction: **Global mode answers from pre-computed community reports**, not relationship-vector ANN.

---

## What EdgeQuake Calls "Global"

Code explicitly denies GraphRAG semantics (`modes.rs:31`):

```text
  EdgeQuake "Global"                    GraphRAG "Global"
  ──────────────────                    ─────────────────

  high-level keyword embed              community summary reports
         │                                      │
         v                                      v
  ANN on relationship vectors           select relevant communities
         │                                      │
         v                                      v
  batch read entity endpoints           map-reduce LLM over summaries
         │                                      │
         v                                      v
  community_id expansion                answer from hierarchical context
  (scan popular nodes)
         │
         v
  provenance chunk retrieval
```

**Finding F-04:** EdgeQuake global mode is **LightRAG relationship-vector global**, optionally expanded by Louvain **labels** — not GraphRAG global search.

---

## Community Handling in Code

### Index time (`community_persist.rs`)

```text
  ingest success
       │
       v
  refresh_community_index()
       │
       v
  detect_communities_unchecked()  ──> Louvain
       │
       v
  persist_community_labels()      ──> node.properties.community_id
```

Migration marker: `044_community_labels_marker.sql`

### Query time (`community_global.rs`)

```text
  seed entities (from rel ANN)
       │
       v
  collect community_id from seeds
       │
       v
  get_popular_nodes_with_degree(N)
       │
       v
  for each popular node:
      if community_id matches seed ──> add entity
```

**No community summaries stored.** `community_id` is an integer label only.

---

## Graph Traversal

GraphRAG local search uses structured subgraph context. EdgeQuake local mode:

- `get_nodes_batch`, `node_degrees_batch`, `get_edges_for_nodes_batch`
- **1-hop** from matched entities only
- `graph_depth: 2` in config — **never used** (F-07)

No path finding, no weighted random walk, no GraphRAG-style entity ranking beyond degree.

---

## ASCII: Missing GraphRAG Layers

```text
  GraphRAG stack                 EdgeQuake stack
  ──────────────                 ───────────────

  [Text chunks]                  [Text chunks]              ✓
       │                              │
       v                              v
  [Entity graph]                 [Entity graph AGE]         ✓
       │                              │
       v                              v
  [Community detection]          [Louvain labels]           △ (labels only)
       │                              │
       v                              X  MISSING
  [Community summaries]          (no summary nodes/tables)
       │                              │
       v                              X  MISSING
  [Global map-reduce query]      [Rel vector ANN + chunks]
```

---

## GraphRAG Expert Assessment

| GraphRAG capability | Present | Code location |
|--------------------|:-------:|---------------|
| Entity extraction to graph | ✓ | pipeline merger |
| Community detection | △ | Louvain at ingest |
| Community summaries | ✗ | — |
| Hierarchical indexing | ✗ | — |
| Global map-reduce | ✗ | — |
| Local subgraph context | △ | 1-hop batch reads |
| Citation to source chunks | ✓ | provenance IDs |

---

## If GraphRAG Were the Goal (code gaps)

1. **Persist community reports** as separate vector/doc records (new `vector_type=community_summary`)
2. **Global query path** selects communities by embedding similarity to query, not rel-vector ANN
3. **Map-reduce** prompt over top-k community summaries before chunk retrieval
4. **Stop running full Louvain per ingest** — incremental community update or scheduled job

None of this exists in code today.

---

## GraphRAG Expert Verdict

**Grade: D+ as GraphRAG, B as LightRAG-global**

Marketing EdgeQuake "Global mode" as GraphRAG would be **misleading**. The code is honest about this in `modes.rs` comments — **keep that honesty in external docs**.

Louvain labels are a **cheap GraphRAG hint**, not GraphRAG. Useful for co-entity expansion on small graphs; **degrades to popular-node scan** at scale.

**Recommendation:** Either commit to GraphRAG (summaries + map-reduce) or rename mode to `RelationshipGlobal` and drop GraphRAG comparisons entirely.

See [012-improvement-plan.md](./012-improvement-plan.md) Phase 3 (optional GraphRAG track).
