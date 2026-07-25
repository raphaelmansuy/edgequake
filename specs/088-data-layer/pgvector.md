# pgvector data-layer operations (`DATA-PGVEC-*`)

ANN search, HNSW/IVF, halfvec, search GUCs. Sources: [pgvector README](https://github.com/pgvector/pgvector) v0.8.5.

## DATA-PGVEC-VECTORS-ANN-QUERY-001

<a id="data-pgvec-vectors-ann-query-001"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PGVEC-VECTORS-ANN-QUERY-001` |
| **Engine** | PGVEC |
| **Domain** | VECTORS |
| **Operation** | ANN-QUERY |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:28` |
| **Entry** | `VectorStorage::query` |
| **Type** | R |
| **Transactional** | Y |
| **Notes** | unfiltered HNSW/IVF |
| **Variables** | N=rows, K=top_k, D=dim, ef=ef_search |
| **Time** | O(ef * log N) expected ANN; worst O(N) if seq scan |
| **Space** | O(K + ef) |
| **I/O** | ~O(ef) random pages; index residency critical |
| **Failure mode** | timeout / under-K results / silent recall loss / seq-scan cliff |
| **Tests** | `data_layer_*` containing `DATA_PGVEC_VECTORS_ANN_QUERY_001` |
| **Benchmark** | [benchmarks/001.md](./benchmarks/001.md) |

**Limits**

- ef_search = clamp(4*K, 40, 1000)
- iterative_scan required for filtered (max_scan_tuples=20000)
- HNSW dim caps: vector 2000 / halfvec 4000
- Recall degrades if filter selectivity low without iterative_scan

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PGVEC-VECTORS-ANN-QUERY-FILTERED-002

<a id="data-pgvec-vectors-ann-query-filtered-002"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PGVEC-VECTORS-ANN-QUERY-FILTERED-002` |
| **Engine** | PGVEC |
| **Domain** | VECTORS |
| **Operation** | ANN-QUERY-FILTERED |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:528` |
| **Entry** | `VectorStorage::query_filtered` |
| **Type** | R |
| **Transactional** | Y |
| **Notes** | tenant/ws/doc + iterative_scan |
| **Variables** | N=rows, K=top_k, D=dim, ef=ef_search |
| **Time** | O(ef * log N) expected ANN; worst O(N) if seq scan |
| **Space** | O(K + ef) |
| **I/O** | ~O(ef) random pages; index residency critical |
| **Failure mode** | timeout / under-K results / silent recall loss / seq-scan cliff |
| **Tests** | `data_layer_*` containing `DATA_PGVEC_VECTORS_ANN_QUERY_FILTERED_002` |
| **Benchmark** | [benchmarks/002.md](./benchmarks/002.md) |

**Limits**

- ef_search = clamp(4*K, 40, 1000)
- iterative_scan required for filtered (max_scan_tuples=20000)
- HNSW dim caps: vector 2000 / halfvec 4000
- Recall degrades if filter selectivity low without iterative_scan

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PGVEC-VECTORS-UPSERT-BATCH-004

<a id="data-pgvec-vectors-upsert-batch-004"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PGVEC-VECTORS-UPSERT-BATCH-004` |
| **Engine** | PGVEC |
| **Domain** | VECTORS |
| **Operation** | UPSERT-BATCH |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:120` |
| **Entry** | `VectorStorage::upsert_report_created` |
| **Type** | W |
| **Transactional** | Y |
| **Notes** | UNNEST ON CONFLICT |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PGVEC_VECTORS_UPSERT_BATCH_004` |
| **Benchmark** | [benchmarks/004.md](./benchmarks/004.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PGVEC-VECTORS-WARMUP-ANN-017

<a id="data-pgvec-vectors-warmup-ann-017"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PGVEC-VECTORS-WARMUP-ANN-017` |
| **Engine** | PGVEC |
| **Domain** | VECTORS |
| **Operation** | WARMUP-ANN |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:711` |
| **Entry** | `VectorStorage::warmup_workspace_ann` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=rows, K=top_k, D=dim, ef=ef_search |
| **Time** | O(ef * log N) expected ANN; worst O(N) if seq scan |
| **Space** | O(K + ef) |
| **I/O** | ~O(ef) random pages; index residency critical |
| **Failure mode** | timeout / under-K results / silent recall loss / seq-scan cliff |
| **Tests** | `data_layer_*` containing `DATA_PGVEC_VECTORS_WARMUP_ANN_017` |
| **Benchmark** | [benchmarks/017.md](./benchmarks/017.md) |

**Limits**

- ef_search = clamp(4*K, 40, 1000)
- iterative_scan required for filtered (max_scan_tuples=20000)
- HNSW dim caps: vector 2000 / halfvec 4000
- Recall degrades if filter selectivity low without iterative_scan

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PGVEC-VECTORS-DDL-CREATE-TABLE-018

<a id="data-pgvec-vectors-ddl-create-table-018"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PGVEC-VECTORS-DDL-CREATE-TABLE-018` |
| **Engine** | PGVEC |
| **Domain** | VECTORS |
| **Operation** | DDL-CREATE-TABLE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/ddl.rs:78` |
| **Entry** | `create_table` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PGVEC_VECTORS_DDL_CREATE_TABLE_018` |
| **Benchmark** | [benchmarks/018.md](./benchmarks/018.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PGVEC-VECTORS-DDL-ENSURE-ANN-INDEX-019

<a id="data-pgvec-vectors-ddl-ensure-ann-index-019"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PGVEC-VECTORS-DDL-ENSURE-ANN-INDEX-019` |
| **Engine** | PGVEC |
| **Domain** | VECTORS |
| **Operation** | DDL-ENSURE-ANN-INDEX |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/ddl.rs:234` |
| **Entry** | `ensure_ann_index` |
| **Type** | DDL |
| **Transactional** | N |
| **Variables** | N=rows, K=top_k, D=dim, ef=ef_search |
| **Time** | O(ef * log N) expected ANN; worst O(N) if seq scan |
| **Space** | O(K + ef) |
| **I/O** | ~O(ef) random pages; index residency critical |
| **Failure mode** | timeout / under-K results / silent recall loss / seq-scan cliff |
| **Tests** | `data_layer_*` containing `DATA_PGVEC_VECTORS_DDL_ENSURE_ANN_INDEX_019` |
| **Benchmark** | [benchmarks/019.md](./benchmarks/019.md) |

**Limits**

- ef_search = clamp(4*K, 40, 1000)
- iterative_scan required for filtered (max_scan_tuples=20000)
- HNSW dim caps: vector 2000 / halfvec 4000
- Recall degrades if filter selectivity low without iterative_scan

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PGVEC-VECTORS-DDL-PARTIAL-HNSW-020

<a id="data-pgvec-vectors-ddl-partial-hnsw-020"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PGVEC-VECTORS-DDL-PARTIAL-HNSW-020` |
| **Engine** | PGVEC |
| **Domain** | VECTORS |
| **Operation** | DDL-PARTIAL-HNSW |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/ddl.rs:272` |
| **Entry** | `ensure_partial_hnsw_for_workspace` |
| **Type** | DDL |
| **Transactional** | N |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PGVEC_VECTORS_DDL_PARTIAL_HNSW_020` |
| **Benchmark** | [benchmarks/020.md](./benchmarks/020.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PGVEC-VECTORS-SESSION-SEARCH-TUNING-022

<a id="data-pgvec-vectors-session-search-tuning-022"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PGVEC-VECTORS-SESSION-SEARCH-TUNING-022` |
| **Engine** | PGVEC |
| **Domain** | VECTORS |
| **Operation** | SESSION-SEARCH-TUNING |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/search_tuning.rs:90` |
| **Entry** | `search_tuning_statements` |
| **Type** | SESSION |
| **Transactional** | Y |
| **Variables** | — |
| **Time** | O(1) |
| **Space** | O(1) |
| **I/O** | none |
| **Failure mode** | GUC leak → wrong recall/plan for next borrower |
| **Tests** | `data_layer_*` containing `DATA_PGVEC_VECTORS_SESSION_SEARCH_TUNING_022` |
| **Benchmark** | [benchmarks/022.md](./benchmarks/022.md) |

**Limits**

- SET LOCAL only inside short transactions
- Do not leak GUCs on pooled conns

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PGVEC-VECTORS-DIM-RECONCILE-024

<a id="data-pgvec-vectors-dim-reconcile-024"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PGVEC-VECTORS-DIM-RECONCILE-024` |
| **Engine** | PGVEC |
| **Domain** | VECTORS |
| **Operation** | DIM-RECONCILE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/migration.rs:111` |
| **Entry** | `reconcile_dimension` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PGVEC_VECTORS_DIM_RECONCILE_024` |
| **Benchmark** | [benchmarks/024.md](./benchmarks/024.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.
