# 009 — O(n) Complexity Expert Lens

**Cross-ref:** [002 Ingestion](./002-ingestion-pipeline-audit.md) · [003 Query](./003-query-retrieval-audit.md) · [F-03,F-08,F-10](./README.md#cross-reference-matrix)

---

## Complexity Map

```text
  PHASE                    DOMINANT COST           BIG-O (typical)
  ─────                    ─────────────           ───────────────
  Chunking                 CPU                   O(|doc|)
  LLM extraction           network               O(chunks × passes × retries)
  Embedding generation     network/API           O(chunks + entities + rels)
  Vector upsert            Postgres              O(vectors) batched ✓
  Graph merge              AGE Cypher            O(entities) batched ✓
  Community refresh        Louvain + write       O(V + E)  ← PER INGEST ⚠
  KV chunk write           I/O                   O(chunks)
  Query embed              API                   O(1) per query
  Query ANN                pgvector              O(log N) per index
  Query graph batch        AGE                   O(seeds)
  Community expand         popular scan          O(popular_limit)
  Hybrid retrieve          3 parallel ANN        O(3 × log N)
  Rerank BM25              CPU                   O(chunks × terms)
```

---

## Hot Spots (brutal ranking)

### H1 — LLM extraction (unavoidable, tunable)

`pipeline/extraction.rs`

```text
  cost ≈ num_chunks × gleaning_passes × retry_attempts × LLM_latency
```

150KB doc → ~250 chunks @ 600 tokens → **250+ LLM calls** before gleaning multiplier.

Mitigations in code:
- Semaphore (`max_concurrent_extractions`)
- Adaptive chunk size in orchestrator only
- Checkpoint skip on retry (worker only)

**Not mitigated:** sync upload on same path without adaptive sizing.

---

### H2 — Community refresh per ingest (avoidable, catastrophic)

`ingestion_persister.rs` → `refresh_community_index`

```text
  each ingest:  Louvain(V, E) + batch upsert(V)
```

| Graph size | Ingests/hour | Louvain runs/hour |
|------------|:------------:|:-----------------:|
| 1k nodes | 100 | 100 |
| 100k nodes | 100 | 100 (each O(100k)) |

**This is the #1 scale killer in the codebase.**

Fix: debounce 5min, or incremental, or nightly job.

---

### H3 — Injection KV scan (avoidable)

`injection.rs` list/delete:

```text
  keys = kv_storage.keys()     ── O(K) all keys in namespace
  filter prefix injection::    ── O(K) memory
```

Does not scale past ~10k keys. Needs prefix scan API or Postgres index on injection table.

---

### H4 — Chunk content duplication (storage multiplier)

F-08: full text in:
1. KV chunk record
2. Vector metadata JSONB
3. FTS generated from metadata JSON

```text
  storage ≈ 3 × |chunk_text|  (plus embedding)
```

100MB corpus → **300MB+ text duplication** before embeddings.

---

### H5 — Global cache invalidation (amortized query cost)

F-09: O(1) invalidate, but forces O(full_retrieval) on every subsequent query.

Effective query cost under ingest load:

```text
  query_cost → always_miss_cache × (ANN + graph + rerank + LLM)
```

---

### H6 — Merger extraction clone

`ingestion_persister.rs`:

```rust
merger.merge(result.extractions.clone())
```

O(extraction_size) copy per persist — minor vs LLM, but unnecessary on large docs.

---

### H7 — Relational sink per-entity upsert

`merger/entity.rs` — loop upsert to relational sink, not batched.

O(entities) SQL round trips when CQRS sink enabled.

---

### H8 — Community expansion scan

`community_global.rs`:

```text
  get_popular_nodes_with_degree(max_entities * 2)
  linear scan for community_id match
```

O(popular_limit) per global query — acceptable small graph; weak at 1M nodes.

---

## Query Path Complexity (acceptable)

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Single ANN | O(log N) | HNSW |
| Filtered ANN | O(log N) to O(N) worst | iterative scan helps |
| Batch graph read | O(seeds) | not O(graph) ✓ |
| Hybrid parallel | O(3 log N) | constant factor |
| BM25 rerank | O(k × \|q\|) | k ≤ rerank_top_k |

Query path is **not** the primary scale bottleneck. Ingest + community refresh is.

---

## Batch Upload

`batch_upload.rs`: sequential O(files × pipeline_cost).

No parallelism — intentional? If so, document. If not, use worker queue batch task.

---

## O(n) Expert Verdict

**Grade: C**

Query retrieval complexity is **sound**. Ingest path contains **two O(graph) per-document operations** (community refresh) and **O(all_keys) injection ops** that violate basic scalability invariants.

**Priority fixes:**
1. Debounce/remove per-ingest Louvain (P0)
2. Prefix-index injections (P1)
3. Deduplicate chunk storage (P1)
4. Workspace-scoped cache (P1)

See [012-improvement-plan.md](./012-improvement-plan.md).
