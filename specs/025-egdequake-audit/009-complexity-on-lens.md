# 009 — O(n) Complexity Expert Lens

**Cross-ref:** [002 Ingestion](./002-ingestion-pipeline-audit.md) · [003 Query](./003-query-retrieval-audit.md) · [007 Postgres](./007-postgres-age-pgvector-lens.md)

**Findings:** R-03, N-03, N-04, N-06, N-09, N-13

---

## First Principle

Every hot path should have **documented asymptotic behavior**. EdgeQuake is honest in comments (`graph_hops.rs`, `community_global.rs`) but several paths are **linear in corpus or graph size** without pagination caps.

**Grade: B** — No hidden exponential traps; several O(n) cliffs at scale.

---

## Ingestion Complexity

| Operation | Complexity | Dominant term | Finding |
|-----------|------------|---------------|---------|
| Single doc ingest | O(c) chunks | c × LLM extract | Expected |
| Merger | O(e + r) | entities + rels per doc | `extractions.clone()` extra copy |
| Community refresh | O(V + E) | Louvain full graph | Debounced — R-03 ✅ |
| Task create | O(\|text\|) | JSONB payload size | N-03 |
| Batch upload admission | O(f) serial | f files sequential | No parallel enqueue |
| Progress KV updates | O(c/3) spawns | tokio::spawn per 3 chunks | Thundering herd |
| Purge tasks for doc | O(t) | list_tasks page 10K | Linear task table scan |

### N-03 — Double storage

```text
  |document| stored in:
    1. KV {doc}-content
    2. tasks.payload.text (JSONB)

  Space: 2 × |document| per in-flight ingest
  Write: 2 × WAL pressure
```

**Fix:** Payload = `{ doc_id, workspace_id, tenant_id }` only; worker reads KV.

---

## Query Complexity

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Naive mode | O(log n) ANN + O(k) BM25 | k = max_chunks × multiplier |
| Local mode | O(e) ANN + O(h × f) graph | h=depth, f=frontier — **N+1** |
| Global mode | O(r) ANN + O(p) popular scan | N-13 |
| Mix/Hybrid | **3×** above | Parallel wall-clock, not CPU — N-04 |
| Keyword extract | O(1) LLM call | 24h cache on repeat queries |
| Rerank | O(c) cross-encoder | c = chunks after merge |
| Hydration | O(c) KV batch | Good — batched |

### N-06 — Graph BFS

```text
  depth d, frontier size f, avg degree δ

  Edge reads: O(d × f × δ) round trips
  Current: 1 RTT per node via get_node_edges

  Example: d=2, f=60, δ=20 → up to 1200 sequential graph calls
```

**This dominates Local mode latency at scale**, not vector ANN.

### N-13 — Community expansion

```text
  get_popular_nodes_with_degree(2 × max_entities)
       │
       └── O(scan popular) not O(communities)
```

---

## Storage Scan Patterns

| API | Pattern | Scale limit |
|-----|---------|-------------|
| `list_injections` | prefix keys + N gets | N-09 |
| Document delete | prefix chunk keys | OK for single doc |
| `keys_with_prefix` SLO test | spec011 200ms target | Works to ~10K keys/workspace |

---

## Cost Amplification Diagram

```text
  1 query (Mix default)

  ┌─────────┐  ┌─────────┐  ┌─────────┐
  │ Local   │  │ Global  │  │ Naive   │
  │ O(e+h×f)│  │ O(r+p)  │  │ O(k)    │
  └────┬────┘  └────┬────┘  └────┬────┘
       │            │            │
       └────────────┼────────────┘
                    v
              RRF O(chunks)
                    v
              Rerank O(chunks)
                    v
              LLM O(tokens)

  Total wall-clock ≈ max(arms) [parallel]
  Total load ≈ SUM(arms)       [database]
```

**Parallelism hides latency, not database load.**

---

## O(n) Expert Recommendations (priority)

1. **P0:** Batch graph edge reads (kill N+6 multiplier)
2. **P1:** Slim task payloads (kill 2× doc storage)
3. **P1:** Pagination on injection list
4. **P2:** Cheap mode routing — skip triple-arm for low-intent queries
5. **P2:** Community lookup index by `community_id` (kill popular scan)

---

## Verdict

EdgeQuake is **not accidentally quadratic**. It is **deliberately thorough** — and thoroughness has a bill.

Load-test before claiming "enterprise scale":
- 100K entity graph + Mix mode + depth=2
- 10K injection keys + list API
- 50MB document + concurrent uploads

See [012-improvement-plan.md](./012-improvement-plan.md) Phase 5 (performance).
