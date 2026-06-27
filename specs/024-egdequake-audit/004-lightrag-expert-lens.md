# 004 — LightRAG Expert Lens

**Cross-ref:** [002 Ingestion](./002-ingestion-pipeline-audit.md) · [003 Query](./003-query-retrieval-audit.md) · [012 Plan](./012-improvement-plan.md)

---

## LightRAG Core Pattern (reference)

LightRAG indexes:
1. **Chunk vectors** (naive retrieval)
2. **Entity vectors** (local — low-level keywords)
3. **Relationship vectors** (global — high-level keywords)
4. **Knowledge graph** (entities + edges with source provenance)
5. **Dual-level keyword extraction** at query time

Query modes map question types to retrieval strategies. Chunks are retrieved via **entity/relationship provenance**, not by embedding chunks directly in local/global modes.

---

## EdgeQuake Alignment

| LightRAG element | EdgeQuake code | Match |
|------------------|----------------|:-----:|
| Multi-vector index (chunk/entity/rel) | `build_chunk_vector_batch`, merger entity/rel embeddings | ✓ |
| Dual-level keywords | `QueryEmbeddings`, keyword extract + validate | ✓ |
| Local = entity-centric | `query_local_with_vector_storage` | ✓ |
| Global = high-level / thematic | `query_global_with_vector_storage` (rel vectors) | ✓ |
| Provenance chunk hydration | ID-restricted chunk ANN in local/global | ✓ |
| Hybrid = combine modes | `query_hybrid_*` — **round-robin, not LightRAG merge** | △ |
| Entity name normalization | UPPERCASE_UNDERSCORE via `EntityId` | ✓ |
| Gleaning (multi-pass extract) | `GleaningExtractor` in orchestrator init | ✓ (path-dependent) |
| Community at query time | **Moved to ingest** (`refresh_community_index`) | ✗ deviation |

---

## ASCII: LightRAG vs EdgeQuake Query Flow

```text
  LightRAG (canonical)                EdgeQuake (code)
  ───────────────────               ─────────────────

  Query                             Query
    │                                 │
    ├─ extract kw (hi/lo)             ├─ extract kw (hi/lo)     ✓
    ├─ embed query levels             ├─ QueryEmbeddings        ✓
    │                                 │
    ├─ LOCAL: entity ANN              ├─ LOCAL: entity ANN      ✓
    │     └─ graph 1-hop              │     └─ batch nodes/edges  ✓
    │     └─ chunks via source_id     │     └─ provenance IDs     ✓
    │                                 │
    ├─ GLOBAL: rel/theme vectors      ├─ GLOBAL: rel ANN        ✓
    │                                 │     └─ + community_id scan (extra)
    │                                 │
    ├─ NAIVE: chunk ANN               ├─ NAIVE: chunk ANN + FTS  ✓+
    │                                 │
    └─ HYBRID: merge by score/dedupe  └─ HYBRID: round-robin     ✗
                                      └─ MIX: weighted/RRF       ✓ (not default)
```

---

## Ingestion: LightRAG Expectations

LightRAG assumes **one insert path** that:
1. Chunks document
2. LLM extracts entities/relationships per chunk
3. Merges into graph
4. Embeds and stores all vector types

EdgeQuake does this in `Pipeline` + `IngestionPersister`, but **entry paths differ**:

| LightRAG expectation | EdgeQuake reality |
|---------------------|-------------------|
| Consistent chunk sizing | Adaptive 600/800/1200 in `orchestrator/ingestion.rs` only |
| Resilient per-chunk extract | Worker yes; injection/library no |
| Single async queue optional | Four execution models (F-01) |

---

## Deviations That Matter

### D1 — Hybrid default is not LightRAG hybrid

LightRAG hybrid typically **deduplicates and ranks** combined context. EdgeQuake `Hybrid` alternates local → global → naive chunks in round-robin. **Different semantics, same name.**

**Fix:** Rename Hybrid → `Interleave` or change default to Mix.

### D2 — Community index at ingest

LightRAG often computes community structure lazily or at query. EdgeQuake runs Louvain on **every successful ingest** (`ingestion_persister.rs:295`).

Trade-off: faster global queries vs **O(graph) ingest tax**. At 100k+ nodes this breaks LightRAG-style incremental ingest.

### D3 — Injection as first-class source

Not in stock LightRAG. EdgeQuake tags `source_type: "injection"` and excludes from citation paths. Reasonable extension; **implementation quality low** (fail-fast, KV scan).

### D4 — Tuple + JSON extraction parsers

EdgeQuake pipeline supports SOTA tuple format beyond stock LightRAG JSON. More robust, more complexity — acceptable if tested (contract tests exist).

---

## LightRAG Expert Verdict

**Grade: B-**

EdgeQuake **understands** LightRAG deeply — provenance chunks, dual keywords, three vector types, mode separation. Recent P-G2 persist unification is what a LightRAG port should look like.

**Fails LightRAG parity on:**
1. Hybrid merge semantics (naming + default)
2. Ingestion path fragmentation
3. Community refresh policy (anti-incremental)

**Exceeds LightRAG on:**
1. Postgres production adapters (batch upsert, workspace isolation)
2. Naive-mode FTS/BM25 fusion
3. Task queue + checkpoints + tenant fairness

See [012-improvement-plan.md](./012-improvement-plan.md) Phase 1 for LightRAG parity fixes.
