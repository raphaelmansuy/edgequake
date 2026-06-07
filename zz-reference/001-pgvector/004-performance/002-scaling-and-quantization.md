# Scaling and Quantization

Source: [pgvector README → Scaling](https://github.com/pgvector/pgvector#scaling),
[Half-Precision Vectors](https://github.com/pgvector/pgvector#half-precision-vectors),
[Binary Quantization](https://github.com/pgvector/pgvector#binary-quantization).

## Three axes

```
              memory
                |
                v
   +----------------------+
   |  reduce vector size  |   <- halfvec, bit + binary_quantize
   +----------------------+
                |
                v
              compute
   +----------------------+
   |  add cores / nodes   |   <- read replicas, Citus, PgDog
   +----------------------+
                |
                v
              latency
   +----------------------+
   |  shrink working set  |   <- partial index, partitioning, tenancy
   +----------------------+
```

## Halve memory with `halfvec`

```sql
ALTER TABLE items
  ALTER COLUMN embedding TYPE halfvec(1536) USING embedding::halfvec(1536);

CREATE INDEX ON items USING hnsw (embedding halfvec_cosine_ops);
```

Half-precision halves storage and roughly halves index RAM with negligible
recall loss for most text embeddings.
Source: [pgvector README → Half-Precision Indexing](https://github.com/pgvector/pgvector#half-precision-indexing).

## 32× memory reduction with binary quantization

```sql
CREATE INDEX ON items USING hnsw
  ((binary_quantize(embedding)::bit(1536)) bit_hamming_ops);
```

Re-rank with the real distance:

```sql
SELECT id
FROM (
  SELECT id, embedding <=> $1 AS distance
  FROM items
  ORDER BY binary_quantize(embedding)::bit(1536)
           <~> binary_quantize($1)
  LIMIT 100
) candidates
ORDER BY distance LIMIT 5;
```

Source: [pgvector README → Binary Quantization](https://github.com/pgvector/pgvector#binary-quantization).

## Read replicas

pgvector is a regular extension — streaming/logical replicas carry the
index. Route ANN reads to replicas using your driver's read/write split.

## Sharding

Two production-tested options:

| Tool                                        | Model                                | Use when                                         |
| ------------------------------------------- | ------------------------------------ | ------------------------------------------------ |
| [Citus](https://github.com/citusdata/citus) | distributed Postgres, shard key      | Multi-tenant SaaS with hard per-tenant isolation |
| [PgDog](https://github.com/levkk/pgdog)     | proxy-level sharding, scatter-gather | Hot dataset that exceeds a single node           |

Both work with pgvector because the extension is installed per node.

## EdgeQuake practical path

For tenants with > ~5M vectors per workspace:

1. Switch the column to `halfvec(1536)`.
2. Add an HNSW index with `halfvec_cosine_ops`.
3. If still tight, add a `binary_quantize`-based index for a pre-filter and
   re-rank against the original.
4. Only then consider Citus.
