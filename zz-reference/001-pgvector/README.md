# `001-pgvector/` — Vector Similarity Search for Postgres

> Upstream: <https://github.com/pgvector/pgvector> · Current version: **v0.8.2**

## 30-second overview

pgvector is a PostgreSQL extension that adds:

- Four vector types: `vector`, `halfvec`, `bit`, `sparsevec`
- Six distance operators: `<->` `<#>` `<=>` `<+>` `<~>` `<%>`
- Two ANN index types: **HNSW** and **IVFFlat**
- Exact NN search out of the box (no index required)

Because it is a regular Postgres extension, you keep ACID, WAL replication,
JOINs, partial indexes, and every other Postgres feature.
Source: [pgvector README → top](https://github.com/pgvector/pgvector#readme).

```
+------------------------+        +------------------------+
| Embedding model        |  ---> | items.embedding         |
| (OpenAI / Ollama / ...)|        | vector(1536) column     |
+------------------------+        +------------------------+
                                          |
                                          v
                              +-------------------------+
                              | HNSW or IVFFlat index   |
                              | (vector_cosine_ops etc.)|
                              +-------------------------+
                                          |
                                          v
                  SELECT ... ORDER BY embedding <=> $1 LIMIT k
```

## Sub-sections

| #   | Folder / file                                | What you'll learn                                                                       |
| --- | -------------------------------------------- | --------------------------------------------------------------------------------------- |
| 0   | [000-mental-model.md](000-mental-model.md)   | **Start here** — the one-screen picture                                                 |
| 1   | [001-why/](001-why/)                         | Why pgvector (vs FAISS / Pinecone / Qdrant) — five-whys                                 |
| 2   | [002-fundamentals/](002-fundamentals/)       | Install, types, operators                                                               |
| 3   | [003-indexing/](003-indexing/)               | HNSW, IVFFlat, filtering, iterative scans                                               |
| 4   | [004-performance/](004-performance/)         | Tuning, quantization, scaling                                                           |
| 5   | [005-edgequake-usage/](005-edgequake-usage/) | How EdgeQuake actually wires it up                                                      |
| 6   | [006-faq/](006-faq/)                         | Short answers to the recurring questions                                                |
| 7   | [007-code-audit.md](007-code-audit.md)       | Gaps found by code audit — upstream APIs + EdgeQuake patterns not yet covered elsewhere |

## When to reach for which page

| Goal                              | Start here                                                                                             |
| --------------------------------- | ------------------------------------------------------------------------------------------------------ |
| "Should we use pgvector at all?"  | [001-why/](001-why/)                                                                                   |
| "Add a vector column to my table" | [002-fundamentals/002-vector-types.md](002-fundamentals/002-vector-types.md)                           |
| "My query is slow"                | [003-indexing/001-hnsw.md](003-indexing/001-hnsw.md) + [004-performance/](004-performance/)            |
| "I have a `WHERE` clause + ANN"   | [003-indexing/003-filtering-and-iterative-scans.md](003-indexing/003-filtering-and-iterative-scans.md) |
| "How does EdgeQuake call it?"     | [005-edgequake-usage/](005-edgequake-usage/)                                                           |
