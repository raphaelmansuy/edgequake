# `003-indexing/` — HNSW, IVFFlat, and the filtering trap

| File                                                                         | Topic                                                |
| ---------------------------------------------------------------------------- | ---------------------------------------------------- |
| [001-hnsw.md](001-hnsw.md)                                                   | Default ANN index — fast queries, slow build         |
| [002-ivfflat.md](002-ivfflat.md)                                             | List-based ANN index — fast build, recall trade-offs |
| [003-filtering-and-iterative-scans.md](003-filtering-and-iterative-scans.md) | The `WHERE + ORDER BY <=>` problem and the 0.8.0 fix |

## Quick comparison

| Trait                | HNSW                     | IVFFlat                      |
| -------------------- | ------------------------ | ---------------------------- |
| Build speed          | Slow                     | Fast                         |
| Query speed          | Faster (at equal recall) | Slower                       |
| Build memory         | High                     | Low                          |
| Needs training data? | No                       | Yes (must INSERT data first) |
| Default in EdgeQuake | **Yes**                  | Fallback                     |

Source: [pgvector README → Indexing](https://github.com/pgvector/pgvector#indexing).
