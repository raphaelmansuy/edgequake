# 005 — GraphRAG Expert Lens

**Cross-ref:** [003 Query](./003-query-retrieval-audit.md) · [006 SOTA](./006-sota-rag-expert-lens.md) · [012 Plan](./012-improvement-plan.md)

**Findings:** R-04, R-03, N-05, N-13

---

## GraphRAG Reference (Microsoft)

GraphRAG pipeline:

1. Extract entities/relationships from chunks
2. **Detect communities** (Leiden/Louvain hierarchy)
3. **Generate community reports** (LLM summaries per community)
4. At query time: **select relevant communities** → map-reduce over reports
5. Global search answers cross-document thematic questions

**Retrieval unit:** Community **report text**, not relationship embedding metadata.

---

## What EdgeQuake Actually Has

```text
  INGEST                          QUERY (Global mode)
  ─────                           ─────────────────

  Entity/rel extract              high_level embedding
       │                               │
       v                               v
  AGE graph merge                 relationship vector ANN
       │                               │
       v                               v
  Louvain (debounced)             metadata → RetrievedRelationship
       │                               │
       v                               v
  community_id label on nodes   community_global:
                                expand co-community entities
                                       │
                                       v
                                provenance chunk retrieval
```

**Code citation:**

```64:66:edgequake/crates/edgequake-query/src/modes.rs
    /// Relationship-centric global search using high-level embeddings over
    /// relationship vectors, with degree-based fallback when no relationship
    /// vectors match. **Not** GraphRAG hierarchical community reports (SPEC-023 I2).
```

---

## Comparison Table (brutal)

| GraphRAG capability | EdgeQuake | Gap |
|---------------------|-----------|-----|
| Hierarchical communities | Flat Louvain labels | **No hierarchy** |
| Community reports (LLM) | None | **Missing entirely** |
| Report index for retrieval | None | **Missing** |
| Map-reduce global answer | Single LLM call on mixed context | **Different paradigm** |
| Local search on entities | Local mode (LightRAG) | ✓ (LightRAG, not GraphRAG) |
| Dynamic community selection at query | `community_global` scans popular nodes | **Weak proxy** |
| Incremental community update | Debounced full re-run | **Not incremental** |

---

## ASCII: GraphRAG vs EdgeQuake Global

```text
  GraphRAG Global Query              EdgeQuake Global Query
  ─────────────────────              ──────────────────────

  Query                              Query
    │                                  │
    v                                  v
  Match community REPORTS            Match RELATIONSHIP VECTORS
  (pre-generated summaries)          (embedding metadata)
    │                                  │
    v                                  v
  Rank communities                   Rank relationships
    │                                  │
    v                                  v
  Map-reduce over reports            Union entities + rels
    │                                  │
    v                                  v
  Thematic answer                    + optional co-community_id
                                     + chunk hydration
                                     │
                                     v
                                   Single LLM pass
```

**These are different products.** Naming Global mode "GraphRAG" in docs or sales would be **misrepresentation**. Code is honest; marketing must match.

---

## Community Index (R-03)

`community_index_service.rs` + `community_persist.rs`:

- Louvain at ingest (debounced 300s)
- Writes `community_id` on graph nodes
- `community_global.rs` uses labels to pull related entities

**What this buys:** Slightly broader entity context for thematic queries.  
**What it does not buy:** GraphRAG-style summarized community knowledge.

### N-13 — Expansion cost

`community_global` uses `get_popular_nodes_with_degree(2 × max_entities)` — scans high-degree nodes, filters by `community_id`. **O(scan)**, not indexed community lookup.

At 500K nodes this is not GraphRAG global search — it is a heuristic patch.

---

## GraphRAG Expert Verdict

**Grade: D+**

EdgeQuake has a **graph** and **community labels**. It does **not** have GraphRAG.

| If you need… | Use… |
|--------------|------|
| Entity-local factual RAG | EdgeQuake Local |
| Thematic cross-doc (LightRAG style) | EdgeQuake Global + Mix |
| GraphRAG community reports | **Not EdgeQuake today** — Phase 3 track |

---

## Phase 3 Minimum Viable GraphRAG (see 012)

To honestly claim GraphRAG:

1. **Offline:** LLM community report generation per Louvain cluster
2. **Index:** Report vectors (or summary chunks) in pgvector
3. **Query:** Global mode selects reports first, then optional entity drill-down
4. **Incremental:** Delta community update, not full Louvain on debounce fire

Without reports, you have **Graph-enhanced LightRAG**, not GraphRAG.
