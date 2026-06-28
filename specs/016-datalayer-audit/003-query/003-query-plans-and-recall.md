# Query Plans, Recall & Precision

How recall and precision evolve with corpus size, and the two knobs EdgeQuake never turns.

## The recall equation (First Principles)

HNSW recall is governed by the search budget `ef_search` relative to `k`:

- pgvector default `hnsw.ef_search = 40` (range 1..1000) — grounded in
  [`zz-reference/001-pgvector`](../../../zz-reference/001-pgvector/README.md).
- Recall rises with `ef_search` and falls as the corpus grows (the same 40-candidate
  window covers a smaller fraction of `N`).
- **EdgeQuake never issues `SET hnsw.ef_search`** anywhere
  (verified: no occurrence in `edgequake/crates`). → **F6.**

**Consequence over time:** at 10k vectors, default `ef_search=40` gives high recall.
At 1M+ vectors, the *same* setting yields a measurably lower recall because the index
is deeper. Recall silently degrades with growth — the worst kind of regression because
nothing errors.

## Filtered search interaction — F7 (recap)

`WHERE <filter> ORDER BY <ann> LIMIT k` post-filters HNSW output. Two failure modes:

| Mode                  | Effect                                            | Trigger                              |
| --------------------- | ------------------------------------------------- | ------------------------------------ |
| **Short result**      | returns `< k` rows                                | selective filter + small `ef_search` |
| **Missed neighbours** | relevant rows beyond the 40-window are never seen | large corpus, scoped query           |

`hnsw.iterative_scan` fixes this by re-scanning the index until `k` *filtered* rows are
found. pgvector supports `strict_order` and `relaxed_order` for HNSW (IVFFlat only
`relaxed_order`) — grounded in `zz-reference/001-pgvector`. EdgeQuake uses HNSW, so
`strict_order` is available and preserves exact ordering.

## Precision

Precision (returned rows actually being correct) is **not** at risk — distances are
computed exactly for the score projection, and the filters are exact predicates. The
problem space is entirely **recall**.

## Recommended query-time tuning

Per query (session-local, no global config change):

```sql
SET LOCAL hnsw.ef_search = GREATEST(40, k * 4);     -- scale budget to k
SET LOCAL hnsw.iterative_scan = strict_order;       -- only when a selective filter is present
```

- Scale `ef_search` with `k` and with expected filter selectivity.
- Enable `iterative_scan` **only** when a metadata/ID filter is present (it adds cost on
  unfiltered queries).
- Bound it with `hnsw.max_scan_tuples` (default 20000) to cap worst-case latency.

See remediation in [`007-improvements/001-quick-wins.md`](../007-improvements/001-quick-wins.md).

## How query latency evolves with N

| N (vectors) | Unfiltered p99 (index in RAM)    | Risk                                              |
| ----------- | -------------------------------- | ------------------------------------------------- |
| ≤ 100k      | < 10 ms                          | none                                              |
| 1M          | 10–50 ms                         | recall drift at default ef_search (F6)            |
| 5M          | 50–100 ms                        | index nears RAM limit; filtered recall (F7) bites |
| > 10M       | > 100 ms or index spills to disk | needs partitioning / quantization                 |

_(latency figures are inference from HNSW `O(ef·log N)` scaling; see
[`006-capacity/`](../006-capacity/001-limits-and-scaling.md) for the RAM-bound model.)_
