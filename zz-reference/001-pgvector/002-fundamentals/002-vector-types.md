# Vector Types

Source: [pgvector README → Reference](https://github.com/pgvector/pgvector#reference).

pgvector ships four storage types. Pick based on **(precision needed) × (working-set size) × (max indexable dimensions)**.

| Type        | Element                      | Storage / element    | Max dims (column) | Max dims (HNSW index) | Max dims (IVFFlat index) |
| ----------- | ---------------------------- | -------------------- | ----------------- | --------------------- | ------------------------ |
| `vector`    | single-precision float (4 B) | `4 × dims + 8` bytes | 16,000            | 2,000                 | 2,000                    |
| `halfvec`   | half-precision float (2 B)   | `2 × dims + 8` bytes | 16,000            | 4,000                 | 4,000                    |
| `bit`       | bit                          | `dims / 8 + 8` bytes | n/a               | 64,000                | 64,000                   |
| `sparsevec` | (idx,value) pair             | `8 × nnz + 16` bytes | 16,000 non-zero   | 1,000 non-zero        | —                        |

Source: pgvector README — [Vector Type](https://github.com/pgvector/pgvector#vector-type),
[Halfvec Type](https://github.com/pgvector/pgvector#halfvec-type),
[Bit Type](https://github.com/pgvector/pgvector#bit-type),
[Sparsevec Type](https://github.com/pgvector/pgvector#sparsevec-type),
plus the "Supported types" lists in the [HNSW](https://github.com/pgvector/pgvector#hnsw)
and [IVFFlat](https://github.com/pgvector/pgvector#ivfflat) sections.

> All elements must be **finite** — no `NaN`, `Infinity`, or `-Infinity`.

## Decision tree

```
                       +-------------------+
                       |  Need > 4000 dims?|
                       +---------+---------+
                                 |
                Yes              |               No
                +----------------+----------------+
                |                                 |
        +-------v-------+                +--------v--------+
        | Use bit +     |                | Need quantize / |
        | binary_quant. |                | tiny RAM?       |
        +---------------+                +--------+--------+
                                                  |
                                  Yes             |          No
                                  +---------------+----------+
                                  |                          |
                          +-------v-------+         +--------v-------+
                          | halfvec       |         | vector (real)  |
                          +---------------+         +----------------+
```

## DDL examples

```sql
-- Standard dense vector (OpenAI text-embedding-3-small is 1536)
CREATE TABLE chunks (id bigserial PRIMARY KEY, embedding vector(1536));

-- Half-precision: same dims, half the storage
CREATE TABLE chunks (id bigserial PRIMARY KEY, embedding halfvec(1536));

-- Binary (e.g. image hashes, quantized text)
CREATE TABLE items (id bigserial PRIMARY KEY, fingerprint bit(256));

-- Sparse (e.g. BM25 / lexical features)
-- Format: {index1:value1,index2:value2}/dimensions  (indices are 1-based)
CREATE TABLE items (id bigserial PRIMARY KEY, lexical sparsevec(50000));
INSERT INTO items (lexical) VALUES ('{1:1,42:0.7,9999:0.3}/50000');
```

Source: [pgvector README → Storing](https://github.com/pgvector/pgvector#storing),
[Sparse Vectors](https://github.com/pgvector/pgvector#sparse-vectors)
(format `{index1:value1,index2:value2}/dimensions`, *"indices start at 1 like SQL arrays"*).

## Mixed-dimension columns

Use bare `vector` (no `(n)`) for tables holding multiple models. Index per
dimension with an expression + partial index:

```sql
CREATE TABLE embeddings (
  model_id bigint,
  item_id  bigint,
  embedding vector,
  PRIMARY KEY (model_id, item_id)
);
CREATE INDEX ON embeddings
  USING hnsw ((embedding::vector(3)) vector_l2_ops)
  WHERE (model_id = 123);
```

Source: [pgvector README → Frequently Asked Questions](https://github.com/pgvector/pgvector#frequently-asked-questions)
("Can I store vectors with different dimensions in the same column?").

## EdgeQuake choice

EdgeQuake uses `vector(1536)` everywhere (chunks, entities, relationships).
See [edgequake/docker/init.sql lines 110, 135, 175](../../../../edgequake/docker/init.sql)
and [edgequake/migrations/001_init_database.sql line 215](../../../../edgequake/migrations/001_init_database.sql).
This matches OpenAI `text-embedding-3-small` and is interchangeable with
Ollama `embeddinggemma` (also 768/1536 depending on model — verify per
deployment).
