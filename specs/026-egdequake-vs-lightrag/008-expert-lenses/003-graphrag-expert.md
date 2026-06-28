# 003 — GraphRAG Expert Lens

**Cross-ref:** [002 Algorithms](../002-algorithms/001-algorithm-comparison.md) · [004 Query](../004-query/001-query-comparison.md)

**Finding:** C-06

---

## GraphRAG Reference Model (Microsoft)

```text
  INDEX TIME                         QUERY TIME
  ──────────                         ──────────

  Extract graph                      Detect query type
       │                                  │
       ▼                                  ▼
  Community detection               Select communities
  (Leiden/Louvain)                       │
       │                                  ▼
       ▼                             Load community REPORTS
  Generate community                     │
  SUMMARIES (LLM)                        ▼
       │                             Map-reduce answer
       ▼
  Store reports + graph
```

**Key:** GraphRAG retrieval unit = **pre-computed community report text**, not entity/relationship vectors.

---

## LightRAG vs GraphRAG

| GraphRAG element | LightRAG | EdgeQuake |
|------------------|:--------:|:---------:|
| Community detection | ✗ | △ Louvain IDs only |
| Community reports | ✗ | ✗ |
| Map-reduce query | ✗ | ✗ |
| Hierarchical summary | ✗ | ✗ |
| Entity-rel graph | ✓ | ✓ |
| Relationship vectors | ✓ | ✓ |

**LightRAG is NOT GraphRAG.** It is entity-centric graph RAG (different paper, different algorithm).

---

## EdgeQuake "Global" Mode — Honest Label

```text
  EdgeQuake Global mode:
  ─────────────────────

  hl_keywords → relationship VDB ANN
       │
       ├── matched relationships
       ├── seed entities
       └── community_id co-members (Louvain)
       │
       └── provenance chunks

  This is LightRAG Global + community hint.
  This is NOT GraphRAG global search.
```

**Source:** `modes/global.rs`, `community_global.rs`.

---

## Community Index Reality

EdgeQuake stores `community_id` on nodes from debounced Louvain (`community_index_service.rs`).

Uses:
- Global mode entity expansion by shared community
- **Not used:** report generation, hierarchical query routing

```text
  GraphRAG community report:     500-2000 token LLM summary per community
  EdgeQuake community_id:        integer cluster label on node properties
```

Orders of magnitude different.

---

## GraphRAG Expert Verdict

| System | GraphRAG Grade | Actual Category |
|--------|:--------------:|-----------------|
| LightRAG | **D** | Entity graph RAG |
| EdgeQuake | **D+** | Entity graph RAG + cluster hints |

**Do not sell either as GraphRAG.**

To become GraphRAG-class, EdgeQuake needs:
1. Community report generation at index time
2. Dynamic community selection at query time
3. Map-reduce over reports for broad queries

Estimated effort: **8-12 weeks** (not a config flag).

---

## ASCII: Three Categories

```text
  Naive RAG          LightRAG / EdgeQuake       GraphRAG
  ─────────          ────────────────────       ────────

  chunks only        graph + 3 VDBs             community reports
       │                    │                        │
       ▼                    ▼                        ▼
  vector search      provenance retrieval      hierarchical map-reduce

  Complexity: low    Complexity: medium        Complexity: high
  Broad Q: weak      Broad Q: medium           Broad Q: strong
```

EdgeQuake and LightRAG occupy the **same category** — neither is GraphRAG.
