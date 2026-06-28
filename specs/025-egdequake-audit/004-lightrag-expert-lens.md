# 004 — LightRAG Expert Lens

**Cross-ref:** [002 Ingestion](./002-ingestion-pipeline-audit.md) · [003 Query](./003-query-retrieval-audit.md) · [012 Plan](./012-improvement-plan.md)

**Findings:** R-01, R-02, R-05, R-06, R-10, N-02, N-04

---

## LightRAG Reference Model

LightRAG builds:

1. Chunk vectors (naive)
2. Entity vectors (local / low-level keywords)
3. Relationship vectors (global / high-level keywords)
4. Knowledge graph with source provenance
5. Dual-level keyword extraction at query time

Query modes are **views** over one index. Local/Global retrieve chunks via entity/relationship provenance — not direct chunk embedding in those modes.

---

## Parity Matrix (post SPEC-024)

| LightRAG element | EdgeQuake | Match |
|------------------|-----------|:-----:|
| Three vector types | `build_chunk_vector_batch`, merger entity/rel embed | ✓ |
| Dual-level keywords | `QueryEmbeddings`, validate against graph | ✓ |
| Local = entity-centric | `modes/local.rs` | ✓ |
| Global = relationship-centric | `modes/global.rs` | ✓ |
| Provenance chunk retrieval | `chunk_retrieval.rs` ID-restricted ANN | ✓ |
| Hybrid combine arms | `hybrid_merge.rs` round-robin | ✓ |
| Entity normalization | UPPERCASE_UNDERSCORE | ✓ |
| Token budget ~30K | `TruncationConfig` 10K/10K/10K | ✓ |
| top_k ≈ 60/60/20 | `max_entities/relationships/chunks` | ✓ |
| Hybrid default semantics | EdgeQuake default is **Mix+RRF**, not Hybrid | △ |
| Gleaning multi-pass | Library only (`orchestrator/ingestion.rs`) | ✗ N-02 |
| Adaptive chunk size | Library only | ✗ N-02 |
| Community at query | Moved to **debounced ingest** | △ deviation |
| Single insert path | HTTP unified; library separate | △ |

---

## ASCII: Canonical vs EdgeQuake (2026-06-27)

```text
  LightRAG (reference)              EdgeQuake (code today)
  ────────────────────              ───────────────────────

  insert() one path                 HTTP → worker queue        ✓
                                    SDK → sync insert          △

  chunk → extract → merge           same Pipeline + Persister   ✓

  query keywords hi/lo              QueryEmbeddings triple      ✓

  LOCAL: entity ANN                 LOCAL + graph_depth BFS     ✓+
        └─ 1-hop graph                    └─ multi-hop (2 default)
       └─ provenance chunks              └─ + BM25 fusion          ✓+

  GLOBAL: rel vectors                 GLOBAL + community_id       ✓+
  NAIVE: chunk ANN                    NAIVE + FTS                 ✓+

  HYBRID: merge combined              HYBRID: round-robin         ✓
  (score/dedupe varies)               MIX: RRF (DEFAULT)          ✓+

  optional community                  Louvain @ ingest debounced  △
```

**✓+** = exceeds stock LightRAG. **△** = intentional deviation.

---

## Deviations That Still Matter

### D1 — Default mode is Mix, not Hybrid

LightRAG users expect "hybrid" naming for combined retrieval. EdgeQuake **defaults to Mix+RRF** — better for ranking, different name and cost profile.

**Honest recommendation:** Document in API that `mode=mix` is production default; treat `hybrid` as LightRAG-compat interleave mode.

### D2 — Community index at ingest (now debounced)

Pre-SPEC-024: Louvain every ingest — **anti-LightRAG incremental model**.  
Post-SPEC-024: 300s debounce per workspace — **acceptable**.

Remaining risk: first ingest after idle still triggers full Louvain on large graph.

### D3 — Library path feature gap (N-02)

LightRAG Python `insert()` behavior maps to EdgeQuake **`EdgeQuake::insert()`**, not HTTP upload.

Production HTTP is the primary surface for most deployments — **Gleaning off by default there is a parity bug**, not an extension.

### D4 — Injection extension

Not in stock LightRAG. EdgeQuake tags `source_type: "injection"`. Reasonable; quality depends on same persister (now ✓).

---

## Hybrid Merge (code law)

```text
  slot i:  local[i] → global[i] → naive[i]
           │
           ├── skip duplicate chunk IDs
           └── stop at max_chunks

  optional: EDGEQUAKE_HYBRID_FUSION=rrf
```

Contract: `contract_hybrid_lightrag.rs`, `e2e_spec024_hybrid_lightrag.rs`.

**Grade for Hybrid fidelity: A** (was F in SPEC-024 baseline).

---

## LightRAG Expert Verdict

| Era | Grade | Notes |
|-----|:-----:|-------|
| SPEC-024 baseline | B- | Hybrid wrong, four ingest paths |
| SPEC-025 (now) | **A** | Core algorithm parity achieved |

**Still not A+ until:**

1. Worker path gets adaptive chunk + Gleaning (N-02)
2. Operators understand Mix default cost (N-04)
3. Injection list scales (N-09)

**EdgeQuake is now a credible LightRAG Rust port** — better Postgres adapters than Python reference, with explicit extensions (BM25, RRF, reranker) that LightRAG users often bolt on manually.
