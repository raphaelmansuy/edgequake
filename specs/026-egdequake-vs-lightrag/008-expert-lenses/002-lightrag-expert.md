# 002 — LightRAG Expert Lens

**Cross-ref:** [002 Algorithms](../002-algorithms/001-algorithm-comparison.md) · [004 Query](../004-query/001-query-comparison.md)

**Finding:** C-01

---

## Reference Model (code law)

LightRAG = dual-level keyword graph RAG with three vector indices and provenance chunk retrieval.

```text
         ┌─────────────────────────────────────────┐
         │              QUERY TIME                  │
         │  keywords_hi ──> rel VDB ──> GLOBAL     │
         │  keywords_lo ──> ent VDB ──> LOCAL      │
         │  query ────────> chunk VDB ─> NAIVE/MIX │
         └─────────────────────────────────────────┘
                              │
                              v
                    provenance chunk pick
                    (source_ids from graph)
                              │
                              v
                         LLM answer
```

**Source:** `operate.py::_build_query_context`, `base.py::QueryParam`.

---

## Parity Matrix (EdgeQuake vs LightRAG)

| Element | Match | EdgeQuake Delta |
|---------|:-----:|-----------------|
| Three vector types | ✓ | — |
| Dual-level keywords | ✓ | + conversation context |
| Entity normalization | ✓ | — |
| Provenance chunks | ✓ | + BM25 fusion |
| Local mode | ✓ | + multi-hop |
| Global mode | ✓ | + community_id |
| Hybrid round-robin | ✓ | — |
| Mix (+ naive arm) | ✓ | RRF not RR |
| Default mode = mix | ✓ | — |
| Gleaning | ✓ | — |
| Token budget ~30K | ✓ | 10K/10K/10K split |
| top_k ≈ 60/60/20 | ✓ | configurable |

**Parity score: 11/12 exact, 1 intentional deviation (Mix fusion).**

---

## Deviations

### D1 — Mix merge semantics

LightRAG: round-robin entities/rels + vector chunks.  
EdgeQuake: RRF across three parallel arms.

**Impact:** Ranking order differs on equal-score items. Quality likely **better** on EdgeQuake; **not** byte-identical.

### D2 — Community index

EdgeQuake runs Louvain at ingest. LightRAG has no equivalent.

**Impact:** Global mode behavior diverges. Not a bug — an extension.

### D3 — BM25 on all arms

Not in LightRAG reference. Users often add externally.

---

## What EdgeQuake Must Not Break

```text
  INVARIANT                          Test guard
  ─────────                          ──────────
  Local never skips entity VDB         contract + e2e_spec024
  Global never skips rel VDB           contract + e2e_spec024
  Graph modes use provenance chunks    chunk_retrieval.rs
  Hybrid round-robin order             contract_hybrid_lightrag.rs
  Entity names normalized              pipeline tests
```

---

## LightRAG Expert Verdict

| Era | EdgeQuake Grade |
|-----|:---------------:|
| Pre-SPEC-024 | C+ (broken hybrid, split paths) |
| Post-SPEC-025 | **A** (core parity) |
| vs LightRAG feature surface | **B+** (extensions + gaps) |

EdgeQuake is a **credible LightRAG Rust port** with Postgres adapters better than Python reference's postgres_impl for multi-tenant ops.

**Not A+ until:** semantic chunking parity, heading context, head-to-head eval on shared corpus.
