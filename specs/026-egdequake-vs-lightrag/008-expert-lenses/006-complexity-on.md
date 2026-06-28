# 006 — O(n) Complexity Lens

**Cross-ref:** [003 Ingestion](../003-ingestion/001-ingestion-comparison.md) · [004 Query](../004-query/001-query-comparison.md)

---

## Ingestion Complexity

| Operation | LightRAG | EdgeQuake | Dominant term |
|-----------|:--------:|:---------:|---------------|
| Chunk document | O(n) tokens | O(n) tokens | n = doc size |
| Extract per chunk | O(c) LLM calls | O(c) parallel | c = chunk count |
| Gleaning | O(c × g) | O(c × g) | g = gleaning passes |
| Merge entities | O(e) keyed locks | O(e) AGE upsert | e = entities/doc |
| Embed chunks | O(c) batch | O(c) UNNEST batch | |
| Community index | ✗ | O(V+E) Louvain | **debounced** |

**EdgeQuake Louvain:** debounced 300s — amortized O(V+E) per workspace burst, not per doc. First run after idle still hurts.

---

## Query Complexity

### LightRAG Mix (typical)

```text
  O(kw) + O(ANN_ent) + O(ANN_rel) + O(ANN_chunk) + O(hydrate)
        + O(edges_1hop) + O(LLM)

  kw = keyword LLM (constant)
  ANN = O(log N) per index with HNSW
  hydrate = O(chunks retrieved)
  edges_1hop = O(entities × degree)
```

### EdgeQuake Mix (default, no intent routing)

```text
  O(kw) + 3 × [O(ANN) + O(BM25_pool) + O(BFS_depth)] + O(rerank) + O(LLM)

  BM25_pool = O(max_chunks × 5) candidate scan
  BFS_depth = O(frontier × batch_edges) — improved by batch (SPEC-025 6.2)
  rerank = O(candidates × cross_encoder)
```

**EdgeQuake Mix ≈ 3× LightRAG Mix** on vector+sparse+graph work.

Intent routing (SPEC-025 6.4) reduces to single-arm for ~40-60% of queries (estimated — needs telemetry).

---

## Known Hotspots

### EdgeQuake (post SPEC-025 fixes)

| Hotspot | Before | After | Status |
|---------|--------|-------|:------:|
| Graph BFS N+1 | O(E) round trips | batch `get_incident_edges_batch` | ✅ |
| community_global scan | O(popular nodes) | `community_ids` filter push-down | ✅ |
| Task payload 2× text | full text in task | KV ref only | ✅ |
| Injection list | O(n) GETs | paginated prefix scan | ⚠ |
| Prefix KV scan | O(all keys) | workspace prefix | ⚠ |

### LightRAG

| Hotspot | Complexity | Notes |
|---------|------------|-------|
| `operate.py` monolith | maintainability | 5995 LOC |
| Full KV scan (some backends) | O(N) | backend-dependent |
| Pipeline status refetch | O(inflight docs) | each pipeline tick |
| Entity merge locks | O(entities) contended | keyed locks help |

---

## Storage Access Patterns

```text
  EdgeQuake Postgres path
  ────────────────────────

  pgvector ANN:     O(log N)  — HNSW index ✓
  FTS BM25:         O(M)      — GIN index, M = corpus
  AGE graph BFS:    O(V+E)    — batch per level ✓
  KV get_by_id:     O(1)      — primary key ✓
  KV prefix scan:   O(K)      — K = matching keys ⚠

  LightRAG Postgres path
  ──────────────────────

  Similar ANN/FTS if postgres_impl configured
  NetworkX default: O(V+E) in-memory — fast, not durable
```

---

## ASCII: Query Cost by Mode

```text
  Cost ▲
       │  ████ EdgeQuake Mix (3-arm + BM25 + rerank)
       │  ███  LightRAG Mix
       │  ██   EdgeQuake Hybrid
       │  ██   LightRAG Hybrid
       │  █    EdgeQuake Local/Naive (intent routed)
       │  █    LightRAG Local/Naive
       └────────────────────────────> Quality (rough)
```

---

## O(n) Expert Verdict

| System | Grade | Notes |
|--------|:-----:|-------|
| LightRAG | **C+** | Predictable per-mode cost; monolith hides hotspots |
| EdgeQuake | **B** | Hotspots documented + partially fixed; Mix default expensive |

**Rule:** Never run EdgeQuake Mix at high QPS without intent routing or explicit mode selection.

**Rule:** Never run Louvain on every ingest (EdgeQuake fixed with debounce; was P0 in SPEC-024).
