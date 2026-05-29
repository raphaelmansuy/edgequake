# `006-faq/` — pgvector FAQ

All answers cite either [pgvector README](https://github.com/pgvector/pgvector#readme)
or an EdgeQuake source file.

---

**Q. Do I need an index for small tables?**
No. Exact search is fast on small tables and gives 100% recall.
Source: [pgvector README → Exact Search](https://github.com/pgvector/pgvector#exact-search).

---

**Q. Why is my query not using the index?**
Most common causes (in order):

1. `ORDER BY` is not the bare `column <op> literal`.
2. Op-class in the index doesn't match the operator (e.g. `vector_l2_ops`
   index but `<=>` query).
3. There's a `WHERE` clause and you need iterative scans
   (see [003-indexing/003-filtering-and-iterative-scans.md](../003-indexing/003-filtering-and-iterative-scans.md)).
4. The table is small enough that the planner picks a seq scan.

Source: [pgvector README → Troubleshooting](https://github.com/pgvector/pgvector#troubleshooting).

---

**Q. Can I store vectors of different dimensions in one column?**
Yes, with bare `vector` (no `(n)`). Add a `model_id` column and use partial
indexes per dimension.
Source: [pgvector README → FAQ](https://github.com/pgvector/pgvector#frequently-asked-questions).

---

**Q. HNSW or IVFFlat — which one?**
HNSW unless build time is critical or memory is tight. EdgeQuake defaults
to HNSW. See [003-indexing/](../003-indexing/).

---

**Q. What's the max dimension?**
Column: 16,000. HNSW/IVFFlat index on `vector`: 2,000. On `halfvec`: 4,000.
On `bit`: 64,000.
Source: pgvector README — [Vector Type](https://github.com/pgvector/pgvector#vector-type),
[HNSW](https://github.com/pgvector/pgvector#hnsw).

---

**Q. Are queries deterministic?**
Exact search: yes. ANN with HNSW/IVFFlat: deterministic given the same data
and parameters, but recall is < 100%.
Source: [pgvector README → Exact Search](https://github.com/pgvector/pgvector#exact-search).

---

**Q. How do I migrate from `vector(N)` to `halfvec(N)`?**
```sql
ALTER TABLE items
  ALTER COLUMN embedding TYPE halfvec(1536) USING embedding::halfvec(1536);
REINDEX INDEX CONCURRENTLY items_embedding_idx;
```

---

**Q. What version of pgvector does EdgeQuake target?**
EdgeQuake calls `CREATE EXTENSION IF NOT EXISTS vector` without a version
pin (see [connection.rs](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs)).
The Docker Compose stack pins the `pgvector/pgvector:pg17` image, which
currently bundles **v0.8.2**.

---

**Q. Does pgvector work behind PgBouncer?**
Yes in `session` mode. In `transaction` mode, set search-time GUCs with
`SET LOCAL` inside the transaction so they don't leak.

---

**Q. Can I use cosine similarity directly?**
The operator returns *cosine distance*. Compute similarity as
`1 - (embedding <=> $1)`. EdgeQuake does this in SQL — see
[005-edgequake-usage/001-adapter-and-migrations.md](../005-edgequake-usage/001-adapter-and-migrations.md).
