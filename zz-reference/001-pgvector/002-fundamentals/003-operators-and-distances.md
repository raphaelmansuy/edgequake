# Distance Operators

Source: [pgvector README → Querying](https://github.com/pgvector/pgvector#querying)
and [Reference → Vector Operators](https://github.com/pgvector/pgvector#vector-operators).

## The six operators

| Op    | Meaning                           | Available on               | Notes                                          |
| ----- | --------------------------------- | -------------------------- | ---------------------------------------------- |
| `<->` | L2 (Euclidean) distance           | vector, halfvec, sparsevec | Default geometric distance                     |
| `<#>` | **Negative** inner product        | vector, halfvec, sparsevec | Negated because PG only does `ASC` index scans |
| `<=>` | Cosine distance (= `1 - cos_sim`) | vector, halfvec, sparsevec | Most common for text embeddings                |
| `<+>` | L1 (taxicab/Manhattan)            | vector, halfvec, sparsevec | Added in 0.7.0                                 |
| `<~>` | Hamming distance                  | bit                        | Binary vectors only                            |
| `<%>` | Jaccard distance                  | bit                        | Binary vectors only                            |

## Index op-class names (the other half you must remember)

Source: [pgvector README → HNSW](https://github.com/pgvector/pgvector#hnsw)
and [IVFFlat](https://github.com/pgvector/pgvector#ivfflat).

| Operator | `vector` op-class   | `halfvec` op-class   | `sparsevec` op-class   | `bit` op-class    |
| -------- | ------------------- | -------------------- | ---------------------- | ----------------- |
| `<->`    | `vector_l2_ops`     | `halfvec_l2_ops`     | `sparsevec_l2_ops`     | —                 |
| `<#>`    | `vector_ip_ops`     | `halfvec_ip_ops`     | `sparsevec_ip_ops`     | —                 |
| `<=>`    | `vector_cosine_ops` | `halfvec_cosine_ops` | `sparsevec_cosine_ops` | —                 |
| `<+>`    | `vector_l1_ops`     | `halfvec_l1_ops`     | `sparsevec_l1_ops`     | —                 |
| `<~>`    | —                   | —                    | —                      | `bit_hamming_ops` |
| `<%>`    | —                   | —                    | —                      | `bit_jaccard_ops` |

## Choosing a distance

```
Are your vectors L2-normalized (||v|| = 1)?
   |
   |--- Yes ---> inner product (<#>) is fastest and equivalent to cosine.
   |           OpenAI embeddings are normalized => prefer <#>.
   |
   |--- No  ---> use cosine (<=>) if you care about direction,
                 L2 (<->) if you care about magnitude.
```

Source: [pgvector README → Exact Search](https://github.com/pgvector/pgvector#exact-search)
("If vectors are normalized to length 1 (like OpenAI embeddings), use inner product for best performance").

## Reading the result

The operators return a **distance**, not a similarity. Convert as needed:

```sql
-- cosine similarity from cosine distance
SELECT 1 - (embedding <=> '[3,1,2]') AS cosine_similarity FROM items;

-- positive inner product (since <#> is negated)
SELECT (embedding <#> '[3,1,2]') * -1 AS inner_product FROM items;
```

Source: [pgvector README → Distances](https://github.com/pgvector/pgvector#distances).

## The single mistake everyone makes

> `ORDER BY` must be **exactly** `column <op> literal`, ascending.
> An expression on top will silently disable the index.

```sql
-- USES the index
ORDER BY embedding <=> '[3,1,2]' LIMIT 5;

-- DOES NOT use the index (wrapped in 1 - ..., DESC)
ORDER BY 1 - (embedding <=> '[3,1,2]') DESC LIMIT 5;
```

Source: [pgvector README → Troubleshooting "Why isn't a query using an index"](https://github.com/pgvector/pgvector#troubleshooting).

## EdgeQuake usage

EdgeQuake uses cosine throughout. From
[edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs):

```rust
"SELECT id, metadata, 1 - (embedding <=> $1::vector) as score
 FROM ... ORDER BY embedding <=> $1::vector LIMIT $2"
```

Note: the `ORDER BY` is the bare operator (index-eligible), while the
`score` column is the human-friendly similarity.
