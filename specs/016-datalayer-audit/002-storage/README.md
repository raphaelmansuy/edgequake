# 002 — Storage

How EdgeQuake lays out vector and graph data on disk, and what it costs.

## Documents

- [`001-vector-storage.md`](001-vector-storage.md) — `eq_{prefix}_vectors` table, columns, on-disk cost.
- [`002-graph-storage.md`](002-graph-storage.md) — AGE graph, label tables, agtype layout.
- [`003-schema-and-indexes.md`](003-schema-and-indexes.md) — every index, why it exists, and its cost.

## Summary verdict

The **schema is well-considered**: materialized filter columns (SPEC-007), an O(1) row
counter (SPEC-011), partial B-tree indexes, and a GIN index on metadata all reflect
real tuning work. The **two storage-level liabilities** are:

- **F5** — chunk text co-located in the vector `metadata` JSONB inflates the hot heap
  and the GIN index ([`001-vector-storage.md`](001-vector-storage.md)).
- The GIN `jsonb_path_ops` index is maintained on a column that also carries large text
  payloads, amplifying write cost ([`003-schema-and-indexes.md`](003-schema-and-indexes.md)).

Cross-reference: [`zz-reference/001-pgvector`](../../../zz-reference/001-pgvector/README.md)
for index internals, [`zz-reference/002-apache-age`](../../../zz-reference/002-apache-age/README.md)
for agtype/label-table storage.
