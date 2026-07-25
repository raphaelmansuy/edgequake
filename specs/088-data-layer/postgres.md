# Postgres data-layer operations (`DATA-PG-*`)

Plain SQL / JSONB / tasks / tenancy / auth. Secondary engine notes appear on hybrid ops.

## DATA-PG-VECTORS-TEXT-SEARCH-FILTERED-003

<a id="data-pg-vectors-text-search-filtered-003"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-VECTORS-TEXT-SEARCH-FILTERED-003` |
| **Engine** | PG |
| **Domain** | VECTORS |
| **Operation** | TEXT-SEARCH-FILTERED |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:698` |
| **Entry** | `VectorStorage::text_search_filtered` |
| **Type** | R |
| **Transactional** | Y |
| **Notes** | FTS GIN |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_VECTORS_TEXT_SEARCH_FILTERED_003` |
| **Benchmark** | [benchmarks/003.md](./benchmarks/003.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-VECTORS-DELETE-BY-ID-005

<a id="data-pg-vectors-delete-by-id-005"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-VECTORS-DELETE-BY-ID-005` |
| **Engine** | PG |
| **Domain** | VECTORS |
| **Operation** | DELETE-BY-ID |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:259` |
| **Entry** | `VectorStorage::delete` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_VECTORS_DELETE_BY_ID_005` |
| **Benchmark** | [benchmarks/005.md](./benchmarks/005.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-VECTORS-DELETE-ENTITY-006

<a id="data-pg-vectors-delete-entity-006"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-VECTORS-DELETE-ENTITY-006` |
| **Engine** | PG |
| **Domain** | VECTORS |
| **Operation** | DELETE-ENTITY |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:277` |
| **Entry** | `VectorStorage::delete_entity` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_VECTORS_DELETE_ENTITY_006` |
| **Benchmark** | [benchmarks/006.md](./benchmarks/006.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-VECTORS-DELETE-ENTITIES-BATCH-007

<a id="data-pg-vectors-delete-entities-batch-007"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-VECTORS-DELETE-ENTITIES-BATCH-007` |
| **Engine** | PG |
| **Domain** | VECTORS |
| **Operation** | DELETE-ENTITIES-BATCH |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:294` |
| **Entry** | `VectorStorage::delete_entities_batch` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_VECTORS_DELETE_ENTITIES_BATCH_007` |
| **Benchmark** | [benchmarks/007.md](./benchmarks/007.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-VECTORS-DELETE-ENTITY-RELATIONS-008

<a id="data-pg-vectors-delete-entity-relations-008"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-VECTORS-DELETE-ENTITY-RELATIONS-008` |
| **Engine** | PG |
| **Domain** | VECTORS |
| **Operation** | DELETE-ENTITY-RELATIONS |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:314` |
| **Entry** | `VectorStorage::delete_entity_relations` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_VECTORS_DELETE_ENTITY_RELATIONS_008` |
| **Benchmark** | [benchmarks/008.md](./benchmarks/008.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-VECTORS-GET-BY-ID-009

<a id="data-pg-vectors-get-by-id-009"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-VECTORS-GET-BY-ID-009` |
| **Engine** | PG |
| **Domain** | VECTORS |
| **Operation** | GET-BY-ID |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:337` |
| **Entry** | `VectorStorage::get_by_id` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_VECTORS_GET_BY_ID_009` |
| **Benchmark** | [benchmarks/009.md](./benchmarks/009.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-VECTORS-GET-BY-IDS-010

<a id="data-pg-vectors-get-by-ids-010"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-VECTORS-GET-BY-IDS-010` |
| **Engine** | PG |
| **Domain** | VECTORS |
| **Operation** | GET-BY-IDS |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:354` |
| **Entry** | `VectorStorage::get_by_ids` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_VECTORS_GET_BY_IDS_010` |
| **Benchmark** | [benchmarks/010.md](./benchmarks/010.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-VECTORS-COUNT-011

<a id="data-pg-vectors-count-011"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-VECTORS-COUNT-011` |
| **Engine** | PG |
| **Domain** | VECTORS |
| **Operation** | COUNT |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:394` |
| **Entry** | `VectorStorage::count` |
| **Type** | R |
| **Transactional** | Y |
| **Notes** | stats O(1) or COUNT* |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_VECTORS_COUNT_011` |
| **Benchmark** | [benchmarks/011.md](./benchmarks/011.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-VECTORS-IS-EMPTY-012

<a id="data-pg-vectors-is-empty-012"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-VECTORS-IS-EMPTY-012` |
| **Engine** | PG |
| **Domain** | VECTORS |
| **Operation** | IS-EMPTY |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:378` |
| **Entry** | `VectorStorage::is_empty` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_VECTORS_IS_EMPTY_012` |
| **Benchmark** | [benchmarks/012.md](./benchmarks/012.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-VECTORS-PING-013

<a id="data-pg-vectors-ping-013"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-VECTORS-PING-013` |
| **Engine** | PG |
| **Domain** | VECTORS |
| **Operation** | PING |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:432` |
| **Entry** | `VectorStorage::ping` |
| **Type** | R |
| **Transactional** | N |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_VECTORS_PING_013` |
| **Benchmark** | [benchmarks/013.md](./benchmarks/013.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-VECTORS-CLEAR-014

<a id="data-pg-vectors-clear-014"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-VECTORS-CLEAR-014` |
| **Engine** | PG |
| **Domain** | VECTORS |
| **Operation** | CLEAR |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:445` |
| **Entry** | `VectorStorage::clear` |
| **Type** | W |
| **Transactional** | Y |
| **Notes** | ADMIN |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_VECTORS_CLEAR_014` |
| **Benchmark** | [benchmarks/014.md](./benchmarks/014.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-VECTORS-CLEAR-WORKSPACE-015

<a id="data-pg-vectors-clear-workspace-015"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-VECTORS-CLEAR-WORKSPACE-015` |
| **Engine** | PG |
| **Domain** | VECTORS |
| **Operation** | CLEAR-WORKSPACE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:471` |
| **Entry** | `VectorStorage::clear_workspace` |
| **Type** | W |
| **Transactional** | Y |
| **Notes** | ADMIN |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_VECTORS_CLEAR_WORKSPACE_015` |
| **Benchmark** | [benchmarks/015.md](./benchmarks/015.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-VECTORS-DELETE-BY-DOCUMENT-016

<a id="data-pg-vectors-delete-by-document-016"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-VECTORS-DELETE-BY-DOCUMENT-016` |
| **Engine** | PG |
| **Domain** | VECTORS |
| **Operation** | DELETE-BY-DOCUMENT |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:492` |
| **Entry** | `VectorStorage::delete_by_document` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_VECTORS_DELETE_BY_DOCUMENT_016` |
| **Benchmark** | [benchmarks/016.md](./benchmarks/016.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-VECTORS-DDL-ENSURE-FTS-021

<a id="data-pg-vectors-ddl-ensure-fts-021"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-VECTORS-DDL-ENSURE-FTS-021` |
| **Engine** | PG |
| **Domain** | VECTORS |
| **Operation** | DDL-ENSURE-FTS |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/ddl.rs:484` |
| **Entry** | `ensure_content_fts` |
| **Type** | DDL |
| **Transactional** | N |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_VECTORS_DDL_ENSURE_FTS_021` |
| **Benchmark** | [benchmarks/021.md](./benchmarks/021.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-VECTORS-WS-DROP-TABLE-023

<a id="data-pg-vectors-ws-drop-table-023"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-VECTORS-WS-DROP-TABLE-023` |
| **Engine** | PG |
| **Domain** | VECTORS |
| **Operation** | WS-DROP-TABLE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/workspace_vector.rs:204` |
| **Entry** | `PgWorkspaceVectorRegistry::drop_workspace_table` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_VECTORS_WS_DROP_TABLE_023` |
| **Benchmark** | [benchmarks/023.md](./benchmarks/023.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-GET-BY-ID-075

<a id="data-pg-kv-get-by-id-075"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-GET-BY-ID-075` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | GET-BY-ID |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:200` |
| **Entry** | `KVStorage::get_by_id` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_GET_BY_ID_075` |
| **Benchmark** | [benchmarks/075.md](./benchmarks/075.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-GET-BY-IDS-076

<a id="data-pg-kv-get-by-ids-076"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-GET-BY-IDS-076` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | GET-BY-IDS |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:214` |
| **Entry** | `KVStorage::get_by_ids` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_GET_BY_IDS_076` |
| **Benchmark** | [benchmarks/076.md](./benchmarks/076.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-GET-BY-IDS-ORDERED-077

<a id="data-pg-kv-get-by-ids-ordered-077"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-GET-BY-IDS-ORDERED-077` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | GET-BY-IDS-ORDERED |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:239` |
| **Entry** | `KVStorage::get_by_ids_ordered` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_GET_BY_IDS_ORDERED_077` |
| **Benchmark** | [benchmarks/077.md](./benchmarks/077.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-FILTER-KEYS-078

<a id="data-pg-kv-filter-keys-078"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-FILTER-KEYS-078` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | FILTER-KEYS |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:263` |
| **Entry** | `KVStorage::filter_keys` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_FILTER_KEYS_078` |
| **Benchmark** | [benchmarks/078.md](./benchmarks/078.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-UPSERT-079

<a id="data-pg-kv-upsert-079"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-UPSERT-079` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | UPSERT |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:285` |
| **Entry** | `KVStorage::upsert` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_UPSERT_079` |
| **Benchmark** | [benchmarks/079.md](./benchmarks/079.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-DELETE-080

<a id="data-pg-kv-delete-080"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-DELETE-080` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | DELETE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:334` |
| **Entry** | `KVStorage::delete` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_DELETE_080` |
| **Benchmark** | [benchmarks/080.md](./benchmarks/080.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-COUNT-081

<a id="data-pg-kv-count-081"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-COUNT-081` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | COUNT |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:368` |
| **Entry** | `KVStorage::count` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_COUNT_081` |
| **Benchmark** | [benchmarks/081.md](./benchmarks/081.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-IS-EMPTY-082

<a id="data-pg-kv-is-empty-082"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-IS-EMPTY-082` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | IS-EMPTY |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:352` |
| **Entry** | `KVStorage::is_empty` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_IS_EMPTY_082` |
| **Benchmark** | [benchmarks/082.md](./benchmarks/082.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-PING-083

<a id="data-pg-kv-ping-083"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-PING-083` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | PING |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:407` |
| **Entry** | `KVStorage::ping` |
| **Type** | R |
| **Transactional** | N |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_PING_083` |
| **Benchmark** | [benchmarks/083.md](./benchmarks/083.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-COUNT-EMBEDDED-CHUNKS-084

<a id="data-pg-kv-count-embedded-chunks-084"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-COUNT-EMBEDDED-CHUNKS-084` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | COUNT-EMBEDDED-CHUNKS |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:422` |
| **Entry** | `KVStorage::count_embedded_chunks_for_docs` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_COUNT_EMBEDDED_CHUNKS_084` |
| **Benchmark** | [benchmarks/084.md](./benchmarks/084.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-KEYS-WITH-PREFIX-085

<a id="data-pg-kv-keys-with-prefix-085"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-KEYS-WITH-PREFIX-085` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | KEYS-WITH-PREFIX |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:473` |
| **Entry** | `KVStorage::keys_with_prefix` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_KEYS_WITH_PREFIX_085` |
| **Benchmark** | [benchmarks/085.md](./benchmarks/085.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-KEYS-WITH-PREFIX-LIMITED-086

<a id="data-pg-kv-keys-with-prefix-limited-086"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-KEYS-WITH-PREFIX-LIMITED-086` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | KEYS-WITH-PREFIX-LIMITED |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:487` |
| **Entry** | `KVStorage::keys_with_prefix_limited` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_KEYS_WITH_PREFIX_LIMITED_086` |
| **Benchmark** | [benchmarks/086.md](./benchmarks/086.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-KEYS-WITH-SUFFIX-087

<a id="data-pg-kv-keys-with-suffix-087"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-KEYS-WITH-SUFFIX-087` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | KEYS-WITH-SUFFIX |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:523` |
| **Entry** | `KVStorage::keys_with_suffix` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_KEYS_WITH_SUFFIX_087` |
| **Benchmark** | [benchmarks/087.md](./benchmarks/087.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-KEYS-WITH-SUFFIX-LIMITED-088

<a id="data-pg-kv-keys-with-suffix-limited-088"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-KEYS-WITH-SUFFIX-LIMITED-088` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | KEYS-WITH-SUFFIX-LIMITED |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:537` |
| **Entry** | `KVStorage::keys_with_suffix_limited` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_KEYS_WITH_SUFFIX_LIMITED_088` |
| **Benchmark** | [benchmarks/088.md](./benchmarks/088.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-KEYS-089

<a id="data-pg-kv-keys-089"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-KEYS-089` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | KEYS |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:571` |
| **Entry** | `KVStorage::keys` |
| **Type** | R |
| **Transactional** | Y |
| **Notes** | ADMIN mid-wildcard |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_KEYS_089` |
| **Benchmark** | [benchmarks/089.md](./benchmarks/089.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-CLEAR-090

<a id="data-pg-kv-clear-090"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-CLEAR-090` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | CLEAR |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:584` |
| **Entry** | `KVStorage::clear` |
| **Type** | W |
| **Transactional** | Y |
| **Notes** | ADMIN |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_CLEAR_090` |
| **Benchmark** | [benchmarks/090.md](./benchmarks/090.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-TRANSITION-IF-STATUS-091

<a id="data-pg-kv-transition-if-status-091"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-TRANSITION-IF-STATUS-091` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | TRANSITION-IF-STATUS |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:614` |
| **Entry** | `KVStorage::transition_if_status` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_TRANSITION_IF_STATUS_091` |
| **Benchmark** | [benchmarks/091.md](./benchmarks/091.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KV-DDL-CREATE-TABLE-092

<a id="data-pg-kv-ddl-create-table-092"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KV-DDL-CREATE-TABLE-092` |
| **Engine** | PG |
| **Domain** | KV |
| **Operation** | DDL-CREATE-TABLE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:107` |
| **Entry** | `PostgresKVStorage::create_table` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=keys, K=batch, M=prefix matches |
| **Time** | O(log N) single; O(K log N) batch; O(M) prefix |
| **Space** | O(K) or O(M) |
| **I/O** | index + heap per key |
| **Failure mode** | error on multi-workspace key batch; O(N) COUNT* fallback |
| **Tests** | `data_layer_*` containing `DATA_PG_KV_DDL_CREATE_TABLE_092` |
| **Benchmark** | [benchmarks/092.md](./benchmarks/092.md) |

**Limits**

- Batch chunk ≤1000 on upsert
- Postgres bind param cap 65535
- Avoid N+1 get_by_id loops
- count prefers stats table O(1)

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-PDF-STORE-093

<a id="data-pg-pdf-store-093"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-PDF-STORE-093` |
| **Engine** | PG |
| **Domain** | PDF |
| **Operation** | STORE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:1` |
| **Entry** | `PdfStorage::store_pdf` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_PDF_STORE_093` |
| **Benchmark** | [benchmarks/093.md](./benchmarks/093.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-PDF-GET-094

<a id="data-pg-pdf-get-094"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-PDF-GET-094` |
| **Engine** | PG |
| **Domain** | PDF |
| **Operation** | GET |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:1` |
| **Entry** | `PdfStorage::get_pdf` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_PDF_GET_094` |
| **Benchmark** | [benchmarks/094.md](./benchmarks/094.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-PDF-UPDATE-MARKDOWN-095

<a id="data-pg-pdf-update-markdown-095"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-PDF-UPDATE-MARKDOWN-095` |
| **Engine** | PG |
| **Domain** | PDF |
| **Operation** | UPDATE-MARKDOWN |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:1` |
| **Entry** | `PdfStorage::update_markdown` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_PDF_UPDATE_MARKDOWN_095` |
| **Benchmark** | [benchmarks/095.md](./benchmarks/095.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-PDF-UPDATE-STATUS-096

<a id="data-pg-pdf-update-status-096"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-PDF-UPDATE-STATUS-096` |
| **Engine** | PG |
| **Domain** | PDF |
| **Operation** | UPDATE-STATUS |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:392` |
| **Entry** | `PdfStorage::update_pdf_processing` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_PDF_UPDATE_STATUS_096` |
| **Benchmark** | [benchmarks/096.md](./benchmarks/096.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-PDF-LINK-TO-DOCUMENT-097

<a id="data-pg-pdf-link-to-document-097"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-PDF-LINK-TO-DOCUMENT-097` |
| **Engine** | PG |
| **Domain** | PDF |
| **Operation** | LINK-TO-DOCUMENT |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:439` |
| **Entry** | `PdfStorage::link_pdf_to_document` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_PDF_LINK_TO_DOCUMENT_097` |
| **Benchmark** | [benchmarks/097.md](./benchmarks/097.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-PDF-LIST-098

<a id="data-pg-pdf-list-098"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-PDF-LIST-098` |
| **Engine** | PG |
| **Domain** | PDF |
| **Operation** | LIST |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:461` |
| **Entry** | `PdfStorage::list_pdfs` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_PDF_LIST_098` |
| **Benchmark** | [benchmarks/098.md](./benchmarks/098.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-PDF-DELETE-099

<a id="data-pg-pdf-delete-099"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-PDF-DELETE-099` |
| **Engine** | PG |
| **Domain** | PDF |
| **Operation** | DELETE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:529` |
| **Entry** | `PdfStorage::delete_pdf` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_PDF_DELETE_099` |
| **Benchmark** | [benchmarks/099.md](./benchmarks/099.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-PDF-CLEAR-MARKDOWN-100

<a id="data-pg-pdf-clear-markdown-100"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-PDF-CLEAR-MARKDOWN-100` |
| **Engine** | PG |
| **Domain** | PDF |
| **Operation** | CLEAR-MARKDOWN |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:546` |
| **Entry** | `PdfStorage::clear_markdown` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_PDF_CLEAR_MARKDOWN_100` |
| **Benchmark** | [benchmarks/100.md](./benchmarks/100.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-DOCS-ENSURE-RECORD-101

<a id="data-pg-docs-ensure-record-101"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-DOCS-ENSURE-RECORD-101` |
| **Engine** | PG |
| **Domain** | DOCS |
| **Operation** | ENSURE-RECORD |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:570` |
| **Entry** | `ensure_document_record` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_DOCS_ENSURE_RECORD_101` |
| **Benchmark** | [benchmarks/101.md](./benchmarks/101.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-DOCS-UPDATE-STATS-102

<a id="data-pg-docs-update-stats-102"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-DOCS-UPDATE-STATS-102` |
| **Engine** | PG |
| **Domain** | DOCS |
| **Operation** | UPDATE-STATS |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:611` |
| **Entry** | `update_document_stats` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_DOCS_UPDATE_STATS_102` |
| **Benchmark** | [benchmarks/102.md](./benchmarks/102.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-DOCS-TOUCH-STATUS-103

<a id="data-pg-docs-touch-status-103"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-DOCS-TOUCH-STATUS-103` |
| **Engine** | PG |
| **Domain** | DOCS |
| **Operation** | TOUCH-STATUS |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:696` |
| **Entry** | `touch_document_status` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_DOCS_TOUCH_STATUS_103` |
| **Benchmark** | [benchmarks/103.md](./benchmarks/103.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-DOCS-DELETE-RECORD-104

<a id="data-pg-docs-delete-record-104"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-DOCS-DELETE-RECORD-104` |
| **Engine** | PG |
| **Domain** | DOCS |
| **Operation** | DELETE-RECORD |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:726` |
| **Entry** | `delete_document_record` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_DOCS_DELETE_RECORD_104` |
| **Benchmark** | [benchmarks/104.md](./benchmarks/104.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-PDF-COUNT-105

<a id="data-pg-pdf-count-105"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-PDF-COUNT-105` |
| **Engine** | PG |
| **Domain** | PDF |
| **Operation** | COUNT |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:749` |
| **Entry** | `count_pdfs` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_PDF_COUNT_105` |
| **Benchmark** | [benchmarks/105.md](./benchmarks/105.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-DOCS-LIST-SUMMARIES-106

<a id="data-pg-docs-list-summaries-106"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-DOCS-LIST-SUMMARIES-106` |
| **Engine** | PG |
| **Domain** | DOCS |
| **Operation** | LIST-SUMMARIES |
| **File:Line** | `edgequake/crates/edgequake-api/src/document_read_model.rs:126` |
| **Entry** | `list_relational_document_summaries` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_DOCS_LIST_SUMMARIES_106` |
| **Benchmark** | [benchmarks/106.md](./benchmarks/106.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-DOCS-DELETE-WORKSPACE-107

<a id="data-pg-docs-delete-workspace-107"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-DOCS-DELETE-WORKSPACE-107` |
| **Engine** | PG |
| **Domain** | DOCS |
| **Operation** | DELETE-WORKSPACE |
| **File:Line** | `edgequake/crates/edgequake-api/src/document_read_model.rs:314` |
| **Entry** | `delete_relational_documents_for_workspace` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_DOCS_DELETE_WORKSPACE_107` |
| **Benchmark** | [benchmarks/107.md](./benchmarks/107.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-ORIGINAL-STORE-108

<a id="data-pg-original-store-108"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-ORIGINAL-STORE-108` |
| **Engine** | PG |
| **Domain** | ORIGINAL |
| **Operation** | STORE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/original_storage_impl.rs:1` |
| **Entry** | `OriginalStorage store/get/delete` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_ORIGINAL_STORE_108` |
| **Benchmark** | [benchmarks/108.md](./benchmarks/108.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-MM-ASSET-STORE-109

<a id="data-pg-mm-asset-store-109"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-MM-ASSET-STORE-109` |
| **Engine** | PG |
| **Domain** | MM-ASSET |
| **Operation** | STORE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/mm_asset_storage_impl.rs:1` |
| **Entry** | `MmAssetStorage CRUD` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_MM_ASSET_STORE_109` |
| **Benchmark** | [benchmarks/109.md](./benchmarks/109.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-CREATE-110

<a id="data-pg-conv-create-110"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-CREATE-110` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | CREATE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:133` |
| **Entry** | `ConversationStorage::create_conversation` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_CREATE_110` |
| **Benchmark** | [benchmarks/110.md](./benchmarks/110.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-GET-111

<a id="data-pg-conv-get-111"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-GET-111` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | GET |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:178` |
| **Entry** | `ConversationStorage::get_conversation` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_GET_111` |
| **Benchmark** | [benchmarks/111.md](./benchmarks/111.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-UPDATE-112

<a id="data-pg-conv-update-112"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-UPDATE-112` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | UPDATE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:198` |
| **Entry** | `ConversationStorage::update_conversation` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_UPDATE_112` |
| **Benchmark** | [benchmarks/112.md](./benchmarks/112.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-DELETE-113

<a id="data-pg-conv-delete-113"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-DELETE-113` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | DELETE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:300` |
| **Entry** | `ConversationStorage::delete_conversation` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_DELETE_113` |
| **Benchmark** | [benchmarks/113.md](./benchmarks/113.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-LIST-114

<a id="data-pg-conv-list-114"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-LIST-114` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | LIST |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:323` |
| **Entry** | `ConversationStorage::list_conversations` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_LIST_114` |
| **Benchmark** | [benchmarks/114.md](./benchmarks/114.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-SHARE-115

<a id="data-pg-conv-share-115"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-SHARE-115` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | SHARE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:436` |
| **Entry** | `ConversationStorage::share_conversation` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_SHARE_115` |
| **Benchmark** | [benchmarks/115.md](./benchmarks/115.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-UNSHARE-116

<a id="data-pg-conv-unshare-116"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-UNSHARE-116` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | UNSHARE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:468` |
| **Entry** | `ConversationStorage::unshare_conversation` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_UNSHARE_116` |
| **Benchmark** | [benchmarks/116.md](./benchmarks/116.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-GET-SHARED-117

<a id="data-pg-conv-get-shared-117"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-GET-SHARED-117` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | GET-SHARED |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:489` |
| **Entry** | `ConversationStorage::get_shared_conversation` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_GET_SHARED_117` |
| **Benchmark** | [benchmarks/117.md](./benchmarks/117.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-MSG-CREATE-118

<a id="data-pg-conv-msg-create-118"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-MSG-CREATE-118` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | MSG-CREATE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:509` |
| **Entry** | `ConversationStorage::create_message` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_MSG_CREATE_118` |
| **Benchmark** | [benchmarks/118.md](./benchmarks/118.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-MSG-UPDATE-119

<a id="data-pg-conv-msg-update-119"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-MSG-UPDATE-119` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | MSG-UPDATE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:553` |
| **Entry** | `ConversationStorage::update_message` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_MSG_UPDATE_119` |
| **Benchmark** | [benchmarks/119.md](./benchmarks/119.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-MSG-GET-120

<a id="data-pg-conv-msg-get-120"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-MSG-GET-120` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | MSG-GET |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:632` |
| **Entry** | `ConversationStorage::get_message` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_MSG_GET_120` |
| **Benchmark** | [benchmarks/120.md](./benchmarks/120.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-MSG-DELETE-121

<a id="data-pg-conv-msg-delete-121"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-MSG-DELETE-121` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | MSG-DELETE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:643` |
| **Entry** | `ConversationStorage::delete_message` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_MSG_DELETE_121` |
| **Benchmark** | [benchmarks/121.md](./benchmarks/121.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-MSG-LIST-122

<a id="data-pg-conv-msg-list-122"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-MSG-LIST-122` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | MSG-LIST |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:661` |
| **Entry** | `ConversationStorage::list_messages` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_MSG_LIST_122` |
| **Benchmark** | [benchmarks/122.md](./benchmarks/122.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-FOLDER-CREATE-123

<a id="data-pg-conv-folder-create-123"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-FOLDER-CREATE-123` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | FOLDER-CREATE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:695` |
| **Entry** | `ConversationStorage::create_folder` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_FOLDER_CREATE_123` |
| **Benchmark** | [benchmarks/123.md](./benchmarks/123.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-FOLDER-LIST-124

<a id="data-pg-conv-folder-list-124"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-FOLDER-LIST-124` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | FOLDER-LIST |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:749` |
| **Entry** | `ConversationStorage::list_folders` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_FOLDER_LIST_124` |
| **Benchmark** | [benchmarks/124.md](./benchmarks/124.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-FOLDER-UPDATE-125

<a id="data-pg-conv-folder-update-125"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-FOLDER-UPDATE-125` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | FOLDER-UPDATE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:772` |
| **Entry** | `ConversationStorage::update_folder` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_FOLDER_UPDATE_125` |
| **Benchmark** | [benchmarks/125.md](./benchmarks/125.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-FOLDER-GET-126

<a id="data-pg-conv-folder-get-126"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-FOLDER-GET-126` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | FOLDER-GET |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:855` |
| **Entry** | `ConversationStorage::get_folder` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_FOLDER_GET_126` |
| **Benchmark** | [benchmarks/126.md](./benchmarks/126.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-FOLDER-DELETE-127

<a id="data-pg-conv-folder-delete-127"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-FOLDER-DELETE-127` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | FOLDER-DELETE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:866` |
| **Entry** | `ConversationStorage::delete_folder` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_FOLDER_DELETE_127` |
| **Benchmark** | [benchmarks/127.md](./benchmarks/127.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-BULK-DELETE-128

<a id="data-pg-conv-bulk-delete-128"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-BULK-DELETE-128` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | BULK-DELETE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:920` |
| **Entry** | `ConversationStorage::bulk_delete` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_BULK_DELETE_128` |
| **Benchmark** | [benchmarks/128.md](./benchmarks/128.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-BULK-ARCHIVE-129

<a id="data-pg-conv-bulk-archive-129"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-BULK-ARCHIVE-129` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | BULK-ARCHIVE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:935` |
| **Entry** | `ConversationStorage::bulk_archive` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_BULK_ARCHIVE_129` |
| **Benchmark** | [benchmarks/129.md](./benchmarks/129.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONV-BULK-MOVE-130

<a id="data-pg-conv-bulk-move-130"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONV-BULK-MOVE-130` |
| **Engine** | PG |
| **Domain** | CONV |
| **Operation** | BULK-MOVE |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:953` |
| **Entry** | `ConversationStorage::bulk_move_to_folder` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONV_BULK_MOVE_130` |
| **Benchmark** | [benchmarks/130.md](./benchmarks/130.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TASKS-CREATE-131

<a id="data-pg-tasks-create-131"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TASKS-CREATE-131` |
| **Engine** | PG |
| **Domain** | TASKS |
| **Operation** | CREATE |
| **File:Line** | `edgequake/crates/edgequake-tasks/src/postgres.rs:117` |
| **Entry** | `PostgresTaskStorage::create_task` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=tasks, W=active workspaces |
| **Time** | O(log N) get; claim O(W + log N) + row lock |
| **Space** | O(1) claim; O(page) list |
| **I/O** | index on status/workspace/lease |
| **Failure mode** | starvation without fair claim; lease expiry race |
| **Tests** | `data_layer_*` containing `DATA_PG_TASKS_CREATE_131` |
| **Benchmark** | [benchmarks/131.md](./benchmarks/131.md) |

**Limits**

- SKIP LOCKED concurrency
- Lease TTL reclaim
- Fair workspace ordering

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TASKS-GET-132

<a id="data-pg-tasks-get-132"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TASKS-GET-132` |
| **Engine** | PG |
| **Domain** | TASKS |
| **Operation** | GET |
| **File:Line** | `edgequake/crates/edgequake-tasks/src/postgres.rs:171` |
| **Entry** | `PostgresTaskStorage::get_task` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=tasks, W=active workspaces |
| **Time** | O(log N) get; claim O(W + log N) + row lock |
| **Space** | O(1) claim; O(page) list |
| **I/O** | index on status/workspace/lease |
| **Failure mode** | starvation without fair claim; lease expiry race |
| **Tests** | `data_layer_*` containing `DATA_PG_TASKS_GET_132` |
| **Benchmark** | [benchmarks/132.md](./benchmarks/132.md) |

**Limits**

- SKIP LOCKED concurrency
- Lease TTL reclaim
- Fair workspace ordering

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TASKS-TOUCH-133

<a id="data-pg-tasks-touch-133"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TASKS-TOUCH-133` |
| **Engine** | PG |
| **Domain** | TASKS |
| **Operation** | TOUCH |
| **File:Line** | `edgequake/crates/edgequake-tasks/src/postgres.rs:190` |
| **Entry** | `PostgresTaskStorage::touch_task` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=tasks, W=active workspaces |
| **Time** | O(log N) get; claim O(W + log N) + row lock |
| **Space** | O(1) claim; O(page) list |
| **I/O** | index on status/workspace/lease |
| **Failure mode** | starvation without fair claim; lease expiry race |
| **Tests** | `data_layer_*` containing `DATA_PG_TASKS_TOUCH_133` |
| **Benchmark** | [benchmarks/133.md](./benchmarks/133.md) |

**Limits**

- SKIP LOCKED concurrency
- Lease TTL reclaim
- Fair workspace ordering

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TASKS-UPDATE-134

<a id="data-pg-tasks-update-134"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TASKS-UPDATE-134` |
| **Engine** | PG |
| **Domain** | TASKS |
| **Operation** | UPDATE |
| **File:Line** | `edgequake/crates/edgequake-tasks/src/postgres.rs:199` |
| **Entry** | `PostgresTaskStorage::update_task` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=tasks, W=active workspaces |
| **Time** | O(log N) get; claim O(W + log N) + row lock |
| **Space** | O(1) claim; O(page) list |
| **I/O** | index on status/workspace/lease |
| **Failure mode** | starvation without fair claim; lease expiry race |
| **Tests** | `data_layer_*` containing `DATA_PG_TASKS_UPDATE_134` |
| **Benchmark** | [benchmarks/134.md](./benchmarks/134.md) |

**Limits**

- SKIP LOCKED concurrency
- Lease TTL reclaim
- Fair workspace ordering

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TASKS-DELETE-135

<a id="data-pg-tasks-delete-135"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TASKS-DELETE-135` |
| **Engine** | PG |
| **Domain** | TASKS |
| **Operation** | DELETE |
| **File:Line** | `edgequake/crates/edgequake-tasks/src/postgres.rs:254` |
| **Entry** | `PostgresTaskStorage::delete_task` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=tasks, W=active workspaces |
| **Time** | O(log N) get; claim O(W + log N) + row lock |
| **Space** | O(1) claim; O(page) list |
| **I/O** | index on status/workspace/lease |
| **Failure mode** | starvation without fair claim; lease expiry race |
| **Tests** | `data_layer_*` containing `DATA_PG_TASKS_DELETE_135` |
| **Benchmark** | [benchmarks/135.md](./benchmarks/135.md) |

**Limits**

- SKIP LOCKED concurrency
- Lease TTL reclaim
- Fair workspace ordering

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TASKS-LIST-136

<a id="data-pg-tasks-list-136"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TASKS-LIST-136` |
| **Engine** | PG |
| **Domain** | TASKS |
| **Operation** | LIST |
| **File:Line** | `edgequake/crates/edgequake-tasks/src/postgres.rs:268` |
| **Entry** | `PostgresTaskStorage::list_tasks` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=tasks, W=active workspaces |
| **Time** | O(log N) get; claim O(W + log N) + row lock |
| **Space** | O(1) claim; O(page) list |
| **I/O** | index on status/workspace/lease |
| **Failure mode** | starvation without fair claim; lease expiry race |
| **Tests** | `data_layer_*` containing `DATA_PG_TASKS_LIST_136` |
| **Benchmark** | [benchmarks/136.md](./benchmarks/136.md) |

**Limits**

- SKIP LOCKED concurrency
- Lease TTL reclaim
- Fair workspace ordering

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TASKS-STATS-137

<a id="data-pg-tasks-stats-137"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TASKS-STATS-137` |
| **Engine** | PG |
| **Domain** | TASKS |
| **Operation** | STATS |
| **File:Line** | `edgequake/crates/edgequake-tasks/src/postgres.rs:352` |
| **Entry** | `PostgresTaskStorage::get_statistics` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=tasks, W=active workspaces |
| **Time** | O(log N) get; claim O(W + log N) + row lock |
| **Space** | O(1) claim; O(page) list |
| **I/O** | index on status/workspace/lease |
| **Failure mode** | starvation without fair claim; lease expiry race |
| **Tests** | `data_layer_*` containing `DATA_PG_TASKS_STATS_137` |
| **Benchmark** | [benchmarks/137.md](./benchmarks/137.md) |

**Limits**

- SKIP LOCKED concurrency
- Lease TTL reclaim
- Fair workspace ordering

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TASKS-FIND-ACTIVE-PDF-138

<a id="data-pg-tasks-find-active-pdf-138"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TASKS-FIND-ACTIVE-PDF-138` |
| **Engine** | PG |
| **Domain** | TASKS |
| **Operation** | FIND-ACTIVE-PDF |
| **File:Line** | `edgequake/crates/edgequake-tasks/src/postgres.rs:429` |
| **Entry** | `PostgresTaskStorage::find_active_pdf_processing_task` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=tasks, W=active workspaces |
| **Time** | O(log N) get; claim O(W + log N) + row lock |
| **Space** | O(1) claim; O(page) list |
| **I/O** | index on status/workspace/lease |
| **Failure mode** | starvation without fair claim; lease expiry race |
| **Tests** | `data_layer_*` containing `DATA_PG_TASKS_FIND_ACTIVE_PDF_138` |
| **Benchmark** | [benchmarks/138.md](./benchmarks/138.md) |

**Limits**

- SKIP LOCKED concurrency
- Lease TTL reclaim
- Fair workspace ordering

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TASKS-FIND-ACTIVE-INGEST-139

<a id="data-pg-tasks-find-active-ingest-139"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TASKS-FIND-ACTIVE-INGEST-139` |
| **Engine** | PG |
| **Domain** | TASKS |
| **Operation** | FIND-ACTIVE-INGEST |
| **File:Line** | `edgequake/crates/edgequake-tasks/src/postgres.rs:467` |
| **Entry** | `PostgresTaskStorage::find_active_pdf_ingest_task` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=tasks, W=active workspaces |
| **Time** | O(log N) get; claim O(W + log N) + row lock |
| **Space** | O(1) claim; O(page) list |
| **I/O** | index on status/workspace/lease |
| **Failure mode** | starvation without fair claim; lease expiry race |
| **Tests** | `data_layer_*` containing `DATA_PG_TASKS_FIND_ACTIVE_INGEST_139` |
| **Benchmark** | [benchmarks/139.md](./benchmarks/139.md) |

**Limits**

- SKIP LOCKED concurrency
- Lease TTL reclaim
- Fair workspace ordering

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TASKS-CLAIM-NEXT-140

<a id="data-pg-tasks-claim-next-140"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TASKS-CLAIM-NEXT-140` |
| **Engine** | PG |
| **Domain** | TASKS |
| **Operation** | CLAIM-NEXT |
| **File:Line** | `edgequake/crates/edgequake-tasks/src/postgres.rs:500` |
| **Entry** | `PostgresTaskStorage::claim_next` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=tasks, W=active workspaces |
| **Time** | O(log N) get; claim O(W + log N) + row lock |
| **Space** | O(1) claim; O(page) list |
| **I/O** | index on status/workspace/lease |
| **Failure mode** | starvation without fair claim; lease expiry race |
| **Tests** | `data_layer_*` containing `DATA_PG_TASKS_CLAIM_NEXT_140` |
| **Benchmark** | [benchmarks/140.md](./benchmarks/140.md) |

**Limits**

- SKIP LOCKED concurrency
- Lease TTL reclaim
- Fair workspace ordering

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TASKS-REFRESH-LEASE-141

<a id="data-pg-tasks-refresh-lease-141"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TASKS-REFRESH-LEASE-141` |
| **Engine** | PG |
| **Domain** | TASKS |
| **Operation** | REFRESH-LEASE |
| **File:Line** | `edgequake/crates/edgequake-tasks/src/postgres.rs:575` |
| **Entry** | `PostgresTaskStorage::refresh_lease` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=tasks, W=active workspaces |
| **Time** | O(log N) get; claim O(W + log N) + row lock |
| **Space** | O(1) claim; O(page) list |
| **I/O** | index on status/workspace/lease |
| **Failure mode** | starvation without fair claim; lease expiry race |
| **Tests** | `data_layer_*` containing `DATA_PG_TASKS_REFRESH_LEASE_141` |
| **Benchmark** | [benchmarks/141.md](./benchmarks/141.md) |

**Limits**

- SKIP LOCKED concurrency
- Lease TTL reclaim
- Fair workspace ordering

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TASKS-RELEASE-CLAIM-142

<a id="data-pg-tasks-release-claim-142"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TASKS-RELEASE-CLAIM-142` |
| **Engine** | PG |
| **Domain** | TASKS |
| **Operation** | RELEASE-CLAIM |
| **File:Line** | `edgequake/crates/edgequake-tasks/src/postgres.rs:606` |
| **Entry** | `PostgresTaskStorage::release_claim` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=tasks, W=active workspaces |
| **Time** | O(log N) get; claim O(W + log N) + row lock |
| **Space** | O(1) claim; O(page) list |
| **I/O** | index on status/workspace/lease |
| **Failure mode** | starvation without fair claim; lease expiry race |
| **Tests** | `data_layer_*` containing `DATA_PG_TASKS_RELEASE_CLAIM_142` |
| **Benchmark** | [benchmarks/142.md](./benchmarks/142.md) |

**Limits**

- SKIP LOCKED concurrency
- Lease TTL reclaim
- Fair workspace ordering

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TASKS-QUEUE-METRICS-143

<a id="data-pg-tasks-queue-metrics-143"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TASKS-QUEUE-METRICS-143` |
| **Engine** | PG |
| **Domain** | TASKS |
| **Operation** | QUEUE-METRICS |
| **File:Line** | `edgequake/crates/edgequake-tasks/src/postgres.rs:637` |
| **Entry** | `PostgresTaskStorage::get_queue_metrics_filtered` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=tasks, W=active workspaces |
| **Time** | O(log N) get; claim O(W + log N) + row lock |
| **Space** | O(1) claim; O(page) list |
| **I/O** | index on status/workspace/lease |
| **Failure mode** | starvation without fair claim; lease expiry race |
| **Tests** | `data_layer_*` containing `DATA_PG_TASKS_QUEUE_METRICS_143` |
| **Benchmark** | [benchmarks/143.md](./benchmarks/143.md) |

**Limits**

- SKIP LOCKED concurrency
- Lease TTL reclaim
- Fair workspace ordering

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TASKS-TOTAL-COUNT-144

<a id="data-pg-tasks-total-count-144"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TASKS-TOTAL-COUNT-144` |
| **Engine** | PG |
| **Domain** | TASKS |
| **Operation** | TOTAL-COUNT |
| **File:Line** | `edgequake/crates/edgequake-tasks/src/postgres.rs:715` |
| **Entry** | `PostgresTaskStorage::get_total_count` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=tasks, W=active workspaces |
| **Time** | O(log N) get; claim O(W + log N) + row lock |
| **Space** | O(1) claim; O(page) list |
| **I/O** | index on status/workspace/lease |
| **Failure mode** | starvation without fair claim; lease expiry race |
| **Tests** | `data_layer_*` containing `DATA_PG_TASKS_TOTAL_COUNT_144` |
| **Benchmark** | [benchmarks/144.md](./benchmarks/144.md) |

**Limits**

- SKIP LOCKED concurrency
- Lease TTL reclaim
- Fair workspace ordering

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TENANT-CREATE-145

<a id="data-pg-tenant-create-145"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TENANT-CREATE-145` |
| **Engine** | PG |
| **Domain** | TENANT |
| **Operation** | CREATE |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/tenant_ops.rs:19` |
| **Entry** | `pg_create_tenant` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_TENANT_CREATE_145` |
| **Benchmark** | [benchmarks/145.md](./benchmarks/145.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TENANT-GET-146

<a id="data-pg-tenant-get-146"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TENANT-GET-146` |
| **Engine** | PG |
| **Domain** | TENANT |
| **Operation** | GET |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/tenant_ops.rs:49` |
| **Entry** | `pg_get_tenant` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_TENANT_GET_146` |
| **Benchmark** | [benchmarks/146.md](./benchmarks/146.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TENANT-GET-BY-SLUG-147

<a id="data-pg-tenant-get-by-slug-147"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TENANT-GET-BY-SLUG-147` |
| **Engine** | PG |
| **Domain** | TENANT |
| **Operation** | GET-BY-SLUG |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/tenant_ops.rs:65` |
| **Entry** | `pg_get_tenant_by_slug` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_TENANT_GET_BY_SLUG_147` |
| **Benchmark** | [benchmarks/147.md](./benchmarks/147.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TENANT-UPDATE-148

<a id="data-pg-tenant-update-148"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TENANT-UPDATE-148` |
| **Engine** | PG |
| **Domain** | TENANT |
| **Operation** | UPDATE |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/tenant_ops.rs:81` |
| **Entry** | `pg_update_tenant` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_TENANT_UPDATE_148` |
| **Benchmark** | [benchmarks/148.md](./benchmarks/148.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TENANT-DELETE-149

<a id="data-pg-tenant-delete-149"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TENANT-DELETE-149` |
| **Engine** | PG |
| **Domain** | TENANT |
| **Operation** | DELETE |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/tenant_ops.rs:109` |
| **Entry** | `pg_delete_tenant` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_TENANT_DELETE_149` |
| **Benchmark** | [benchmarks/149.md](./benchmarks/149.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-TENANT-LIST-150

<a id="data-pg-tenant-list-150"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-TENANT-LIST-150` |
| **Engine** | PG |
| **Domain** | TENANT |
| **Operation** | LIST |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/tenant_ops.rs:135` |
| **Entry** | `pg_list_tenants` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_TENANT_LIST_150` |
| **Benchmark** | [benchmarks/150.md](./benchmarks/150.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-WORKSPACE-CREATE-151

<a id="data-pg-workspace-create-151"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-WORKSPACE-CREATE-151` |
| **Engine** | PG |
| **Domain** | WORKSPACE |
| **Operation** | CREATE |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/workspace_ops.rs:23` |
| **Entry** | `pg_create_workspace` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_WORKSPACE_CREATE_151` |
| **Benchmark** | [benchmarks/151.md](./benchmarks/151.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-WORKSPACE-GET-152

<a id="data-pg-workspace-get-152"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-WORKSPACE-GET-152` |
| **Engine** | PG |
| **Domain** | WORKSPACE |
| **Operation** | GET |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/workspace_ops.rs:249` |
| **Entry** | `pg_get_workspace` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_WORKSPACE_GET_152` |
| **Benchmark** | [benchmarks/152.md](./benchmarks/152.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-WORKSPACE-GET-BY-SLUG-153

<a id="data-pg-workspace-get-by-slug-153"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-WORKSPACE-GET-BY-SLUG-153` |
| **Engine** | PG |
| **Domain** | WORKSPACE |
| **Operation** | GET-BY-SLUG |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/workspace_ops.rs:265` |
| **Entry** | `pg_get_workspace_by_slug` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_WORKSPACE_GET_BY_SLUG_153` |
| **Benchmark** | [benchmarks/153.md](./benchmarks/153.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-WORKSPACE-UPDATE-154

<a id="data-pg-workspace-update-154"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-WORKSPACE-UPDATE-154` |
| **Engine** | PG |
| **Domain** | WORKSPACE |
| **Operation** | UPDATE |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/workspace_ops.rs:286` |
| **Entry** | `pg_update_workspace` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_WORKSPACE_UPDATE_154` |
| **Benchmark** | [benchmarks/154.md](./benchmarks/154.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-WORKSPACE-DELETE-155

<a id="data-pg-workspace-delete-155"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-WORKSPACE-DELETE-155` |
| **Engine** | PG |
| **Domain** | WORKSPACE |
| **Operation** | DELETE |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/workspace_ops.rs:393` |
| **Entry** | `pg_delete_workspace` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_WORKSPACE_DELETE_155` |
| **Benchmark** | [benchmarks/155.md](./benchmarks/155.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-WORKSPACE-LIST-156

<a id="data-pg-workspace-list-156"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-WORKSPACE-LIST-156` |
| **Engine** | PG |
| **Domain** | WORKSPACE |
| **Operation** | LIST |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/workspace_ops.rs:404` |
| **Entry** | `pg_list_workspaces` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_WORKSPACE_LIST_156` |
| **Benchmark** | [benchmarks/156.md](./benchmarks/156.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-MEMBERSHIP-ADD-158

<a id="data-pg-membership-add-158"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-MEMBERSHIP-ADD-158` |
| **Engine** | PG |
| **Domain** | MEMBERSHIP |
| **Operation** | ADD |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/membership_ops.rs:19` |
| **Entry** | `pg_add_membership` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_MEMBERSHIP_ADD_158` |
| **Benchmark** | [benchmarks/158.md](./benchmarks/158.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-MEMBERSHIP-GET-USER-159

<a id="data-pg-membership-get-user-159"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-MEMBERSHIP-GET-USER-159` |
| **Engine** | PG |
| **Domain** | MEMBERSHIP |
| **Operation** | GET-USER |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/membership_ops.rs:40` |
| **Entry** | `pg_get_user_memberships` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_MEMBERSHIP_GET_USER_159` |
| **Benchmark** | [benchmarks/159.md](./benchmarks/159.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-MEMBERSHIP-GET-TENANT-160

<a id="data-pg-membership-get-tenant-160"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-MEMBERSHIP-GET-TENANT-160` |
| **Engine** | PG |
| **Domain** | MEMBERSHIP |
| **Operation** | GET-TENANT |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/membership_ops.rs:56` |
| **Entry** | `pg_get_tenant_memberships` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_MEMBERSHIP_GET_TENANT_160` |
| **Benchmark** | [benchmarks/160.md](./benchmarks/160.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-MEMBERSHIP-UPDATE-ROLE-161

<a id="data-pg-membership-update-role-161"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-MEMBERSHIP-UPDATE-ROLE-161` |
| **Engine** | PG |
| **Domain** | MEMBERSHIP |
| **Operation** | UPDATE-ROLE |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/membership_ops.rs:75` |
| **Entry** | `pg_update_membership_role` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_MEMBERSHIP_UPDATE_ROLE_161` |
| **Benchmark** | [benchmarks/161.md](./benchmarks/161.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-MEMBERSHIP-REMOVE-162

<a id="data-pg-membership-remove-162"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-MEMBERSHIP-REMOVE-162` |
| **Engine** | PG |
| **Domain** | MEMBERSHIP |
| **Operation** | REMOVE |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/membership_ops.rs:106` |
| **Entry** | `pg_remove_membership` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_MEMBERSHIP_REMOVE_162` |
| **Benchmark** | [benchmarks/162.md](./benchmarks/162.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-MEMBERSHIP-CHECK-TENANT-163

<a id="data-pg-membership-check-tenant-163"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-MEMBERSHIP-CHECK-TENANT-163` |
| **Engine** | PG |
| **Domain** | MEMBERSHIP |
| **Operation** | CHECK-TENANT |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/membership_ops.rs:116` |
| **Entry** | `pg_check_tenant_access` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_MEMBERSHIP_CHECK_TENANT_163` |
| **Benchmark** | [benchmarks/163.md](./benchmarks/163.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-MEMBERSHIP-CHECK-WORKSPACE-164

<a id="data-pg-membership-check-workspace-164"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-MEMBERSHIP-CHECK-WORKSPACE-164` |
| **Engine** | PG |
| **Domain** | MEMBERSHIP |
| **Operation** | CHECK-WORKSPACE |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/membership_ops.rs:133` |
| **Entry** | `pg_check_workspace_access` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_MEMBERSHIP_CHECK_WORKSPACE_164` |
| **Benchmark** | [benchmarks/164.md](./benchmarks/164.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-MEMBERSHIP-GET-ROLE-165

<a id="data-pg-membership-get-role-165"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-MEMBERSHIP-GET-ROLE-165` |
| **Engine** | PG |
| **Domain** | MEMBERSHIP |
| **Operation** | GET-ROLE |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/membership_ops.rs:150` |
| **Entry** | `pg_get_user_role` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_MEMBERSHIP_GET_ROLE_165` |
| **Benchmark** | [benchmarks/165.md](./benchmarks/165.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-QUOTA-UPDATE-TENANT-166

<a id="data-pg-quota-update-tenant-166"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-QUOTA-UPDATE-TENANT-166` |
| **Engine** | PG |
| **Domain** | QUOTA |
| **Operation** | UPDATE-TENANT |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/quota_ops.rs:19` |
| **Entry** | `pg_update_tenant_quota` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_QUOTA_UPDATE_TENANT_166` |
| **Benchmark** | [benchmarks/166.md](./benchmarks/166.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-METRICS-RECORD-SNAPSHOT-167

<a id="data-pg-metrics-record-snapshot-167"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-METRICS-RECORD-SNAPSHOT-167` |
| **Engine** | PG |
| **Domain** | METRICS |
| **Operation** | RECORD-SNAPSHOT |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/metrics_ops.rs:17` |
| **Entry** | `pg_record_metrics_snapshot` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_METRICS_RECORD_SNAPSHOT_167` |
| **Benchmark** | [benchmarks/167.md](./benchmarks/167.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-METRICS-GET-HISTORY-168

<a id="data-pg-metrics-get-history-168"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-METRICS-GET-HISTORY-168` |
| **Engine** | PG |
| **Domain** | METRICS |
| **Operation** | GET-HISTORY |
| **File:Line** | `edgequake/crates/edgequake-core/src/workspace_service_impl/metrics_ops.rs:82` |
| **Entry** | `pg_get_metrics_history` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_METRICS_GET_HISTORY_168` |
| **Benchmark** | [benchmarks/168.md](./benchmarks/168.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-AUTH-SYNC-USER-169

<a id="data-pg-auth-sync-user-169"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-AUTH-SYNC-USER-169` |
| **Engine** | PG |
| **Domain** | AUTH |
| **Operation** | SYNC-USER |
| **File:Line** | `edgequake/crates/edgequake-api/src/services/identity_storage.rs:118` |
| **Entry** | `sync_auth_user_to_postgres` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_AUTH_SYNC_USER_169` |
| **Benchmark** | [benchmarks/169.md](./benchmarks/169.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-AUTH-ENSURE-DEFAULT-TENANT-WS-170

<a id="data-pg-auth-ensure-default-tenant-ws-170"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-AUTH-ENSURE-DEFAULT-TENANT-WS-170` |
| **Engine** | PG |
| **Domain** | AUTH |
| **Operation** | ENSURE-DEFAULT-TENANT-WS |
| **File:Line** | `edgequake/crates/edgequake-api/src/services/identity_storage.rs:189` |
| **Entry** | `ensure_default_tenant_workspace` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_AUTH_ENSURE_DEFAULT_TENANT_WS_170` |
| **Benchmark** | [benchmarks/170.md](./benchmarks/170.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-AUTH-SYNC-MEMBERSHIP-171

<a id="data-pg-auth-sync-membership-171"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-AUTH-SYNC-MEMBERSHIP-171` |
| **Engine** | PG |
| **Domain** | AUTH |
| **Operation** | SYNC-MEMBERSHIP |
| **File:Line** | `edgequake/crates/edgequake-api/src/services/identity_storage.rs:239` |
| **Entry** | `sync_default_membership_to_postgres` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_AUTH_SYNC_MEMBERSHIP_171` |
| **Benchmark** | [benchmarks/171.md](./benchmarks/171.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-AUTH-VERIFY-MEMBERSHIP-172

<a id="data-pg-auth-verify-membership-172"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-AUTH-VERIFY-MEMBERSHIP-172` |
| **Engine** | PG |
| **Domain** | AUTH |
| **Operation** | VERIFY-MEMBERSHIP |
| **File:Line** | `edgequake/crates/edgequake-api/src/services/identity_storage.rs:279` |
| **Entry** | `verify_membership_active` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_AUTH_VERIFY_MEMBERSHIP_172` |
| **Benchmark** | [benchmarks/172.md](./benchmarks/172.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-AUTH-LOAD-USER-173

<a id="data-pg-auth-load-user-173"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-AUTH-LOAD-USER-173` |
| **Engine** | PG |
| **Domain** | AUTH |
| **Operation** | LOAD-USER |
| **File:Line** | `edgequake/crates/edgequake-api/src/services/identity_storage.rs:440` |
| **Entry** | `load_user_record_pg` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_AUTH_LOAD_USER_173` |
| **Benchmark** | [benchmarks/173.md](./benchmarks/173.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-AUTH-FIND-USER-BY-LOGIN-174

<a id="data-pg-auth-find-user-by-login-174"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-AUTH-FIND-USER-BY-LOGIN-174` |
| **Engine** | PG |
| **Domain** | AUTH |
| **Operation** | FIND-USER-BY-LOGIN |
| **File:Line** | `edgequake/crates/edgequake-api/src/services/identity_storage.rs:477` |
| **Entry** | `find_user_record_by_login_pg` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_AUTH_FIND_USER_BY_LOGIN_174` |
| **Benchmark** | [benchmarks/174.md](./benchmarks/174.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-AUTH-LIST-USERS-175

<a id="data-pg-auth-list-users-175"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-AUTH-LIST-USERS-175` |
| **Engine** | PG |
| **Domain** | AUTH |
| **Operation** | LIST-USERS |
| **File:Line** | `edgequake/crates/edgequake-api/src/services/identity_storage.rs:516` |
| **Entry** | `list_user_records_pg` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_AUTH_LIST_USERS_175` |
| **Benchmark** | [benchmarks/175.md](./benchmarks/175.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-AUTH-DELETE-USER-176

<a id="data-pg-auth-delete-user-176"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-AUTH-DELETE-USER-176` |
| **Engine** | PG |
| **Domain** | AUTH |
| **Operation** | DELETE-USER |
| **File:Line** | `edgequake/crates/edgequake-api/src/services/identity_storage.rs:551` |
| **Entry** | `delete_user_pg` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_AUTH_DELETE_USER_176` |
| **Benchmark** | [benchmarks/176.md](./benchmarks/176.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SESSION-PERSIST-REFRESH-177

<a id="data-pg-session-persist-refresh-177"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SESSION-PERSIST-REFRESH-177` |
| **Engine** | PG |
| **Domain** | SESSION |
| **Operation** | PERSIST-REFRESH |
| **File:Line** | `edgequake/crates/edgequake-api/src/services/session_storage.rs:38` |
| **Entry** | `persist_refresh_token_pg` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_SESSION_PERSIST_REFRESH_177` |
| **Benchmark** | [benchmarks/177.md](./benchmarks/177.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SESSION-LOAD-REFRESH-178

<a id="data-pg-session-load-refresh-178"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SESSION-LOAD-REFRESH-178` |
| **Engine** | PG |
| **Domain** | SESSION |
| **Operation** | LOAD-REFRESH |
| **File:Line** | `edgequake/crates/edgequake-api/src/services/session_storage.rs:78` |
| **Entry** | `load_refresh_token_pg` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_SESSION_LOAD_REFRESH_178` |
| **Benchmark** | [benchmarks/178.md](./benchmarks/178.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SESSION-REVOKE-REFRESH-179

<a id="data-pg-session-revoke-refresh-179"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SESSION-REVOKE-REFRESH-179` |
| **Engine** | PG |
| **Domain** | SESSION |
| **Operation** | REVOKE-REFRESH |
| **File:Line** | `edgequake/crates/edgequake-api/src/services/session_storage.rs:129` |
| **Entry** | `revoke_refresh_token_pg` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_SESSION_REVOKE_REFRESH_179` |
| **Benchmark** | [benchmarks/179.md](./benchmarks/179.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SESSION-PERSIST-API-KEY-180

<a id="data-pg-session-persist-api-key-180"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SESSION-PERSIST-API-KEY-180` |
| **Engine** | PG |
| **Domain** | SESSION |
| **Operation** | PERSIST-API-KEY |
| **File:Line** | `edgequake/crates/edgequake-api/src/services/session_storage.rs:256` |
| **Entry** | `persist_api_key_pg` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_SESSION_PERSIST_API_KEY_180` |
| **Benchmark** | [benchmarks/180.md](./benchmarks/180.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SESSION-LIST-API-KEYS-181

<a id="data-pg-session-list-api-keys-181"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SESSION-LIST-API-KEYS-181` |
| **Engine** | PG |
| **Domain** | SESSION |
| **Operation** | LIST-API-KEYS |
| **File:Line** | `edgequake/crates/edgequake-api/src/services/session_storage.rs:358` |
| **Entry** | `list_api_keys_pg` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_SESSION_LIST_API_KEYS_181` |
| **Benchmark** | [benchmarks/181.md](./benchmarks/181.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SESSION-FIND-API-KEY-PREFIX-182

<a id="data-pg-session-find-api-key-prefix-182"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SESSION-FIND-API-KEY-PREFIX-182` |
| **Engine** | PG |
| **Domain** | SESSION |
| **Operation** | FIND-API-KEY-PREFIX |
| **File:Line** | `edgequake/crates/edgequake-api/src/services/session_storage.rs:393` |
| **Entry** | `find_api_keys_by_prefix_pg` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_SESSION_FIND_API_KEY_PREFIX_182` |
| **Benchmark** | [benchmarks/182.md](./benchmarks/182.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SESSION-REVOKE-API-KEY-183

<a id="data-pg-session-revoke-api-key-183"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SESSION-REVOKE-API-KEY-183` |
| **Engine** | PG |
| **Domain** | SESSION |
| **Operation** | REVOKE-API-KEY |
| **File:Line** | `edgequake/crates/edgequake-api/src/services/session_storage.rs:426` |
| **Entry** | `revoke_api_key_pg` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_SESSION_REVOKE_API_KEY_183` |
| **Benchmark** | [benchmarks/183.md](./benchmarks/183.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-ENTITY-UPSERT-184

<a id="data-pg-entity-upsert-184"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-ENTITY-UPSERT-184` |
| **Engine** | PG |
| **Domain** | ENTITY |
| **Operation** | UPSERT |
| **File:Line** | `edgequake/crates/edgequake-api/src/postgres_entity_sink.rs:78` |
| **Entry** | `PostgresEntitySink::upsert_entity` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_ENTITY_UPSERT_184` |
| **Benchmark** | [benchmarks/184.md](./benchmarks/184.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-ENTITY-REMOVE-SOURCES-185

<a id="data-pg-entity-remove-sources-185"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-ENTITY-REMOVE-SOURCES-185` |
| **Engine** | PG |
| **Domain** | ENTITY |
| **Operation** | REMOVE-SOURCES |
| **File:Line** | `edgequake/crates/edgequake-api/src/postgres_entity_sink.rs:132` |
| **Entry** | `remove_entity_sources` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_ENTITY_REMOVE_SOURCES_185` |
| **Benchmark** | [benchmarks/185.md](./benchmarks/185.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-LINEAGE-RECORD-ENTITY-LINK-186

<a id="data-pg-lineage-record-entity-link-186"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-LINEAGE-RECORD-ENTITY-LINK-186` |
| **Engine** | PG |
| **Domain** | LINEAGE |
| **Operation** | RECORD-ENTITY-LINK |
| **File:Line** | `edgequake/crates/edgequake-api/src/postgres_lineage_sink.rs:68` |
| **Entry** | `record_entity_link` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_LINEAGE_RECORD_ENTITY_LINK_186` |
| **Benchmark** | [benchmarks/186.md](./benchmarks/186.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-LINEAGE-RECORD-RELATION-LINK-187

<a id="data-pg-lineage-record-relation-link-187"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-LINEAGE-RECORD-RELATION-LINK-187` |
| **Engine** | PG |
| **Domain** | LINEAGE |
| **Operation** | RECORD-RELATION-LINK |
| **File:Line** | `edgequake/crates/edgequake-api/src/postgres_lineage_sink.rs:94` |
| **Entry** | `record_relation_link` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_LINEAGE_RECORD_RELATION_LINK_187` |
| **Benchmark** | [benchmarks/187.md](./benchmarks/187.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-LINEAGE-RECORD-RELATION-LINKS-BATCH-188

<a id="data-pg-lineage-record-relation-links-batch-188"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-LINEAGE-RECORD-RELATION-LINKS-BATCH-188` |
| **Engine** | PG |
| **Domain** | LINEAGE |
| **Operation** | RECORD-RELATION-LINKS-BATCH |
| **File:Line** | `edgequake/crates/edgequake-api/src/postgres_lineage_sink.rs:122` |
| **Entry** | `record_relation_links_batch` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_LINEAGE_RECORD_RELATION_LINKS_BATCH_188` |
| **Benchmark** | [benchmarks/188.md](./benchmarks/188.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-LINEAGE-RECORD-ENTITY-LINKS-BATCH-189

<a id="data-pg-lineage-record-entity-links-batch-189"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-LINEAGE-RECORD-ENTITY-LINKS-BATCH-189` |
| **Engine** | PG |
| **Domain** | LINEAGE |
| **Operation** | RECORD-ENTITY-LINKS-BATCH |
| **File:Line** | `edgequake/crates/edgequake-api/src/postgres_lineage_sink.rs:157` |
| **Entry** | `record_entity_links_batch` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_LINEAGE_RECORD_ENTITY_LINKS_BATCH_189` |
| **Benchmark** | [benchmarks/189.md](./benchmarks/189.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-LINEAGE-APPEND-DESC-HISTORY-190

<a id="data-pg-lineage-append-desc-history-190"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-LINEAGE-APPEND-DESC-HISTORY-190` |
| **Engine** | PG |
| **Domain** | LINEAGE |
| **Operation** | APPEND-DESC-HISTORY |
| **File:Line** | `edgequake/crates/edgequake-api/src/postgres_lineage_sink.rs:190` |
| **Entry** | `append_description_history` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_LINEAGE_APPEND_DESC_HISTORY_190` |
| **Benchmark** | [benchmarks/190.md](./benchmarks/190.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-LINEAGE-LOAD-DOC-FROM-CHUNKS-191

<a id="data-pg-lineage-load-doc-from-chunks-191"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-LINEAGE-LOAD-DOC-FROM-CHUNKS-191` |
| **Engine** | PG |
| **Domain** | LINEAGE |
| **Operation** | LOAD-DOC-FROM-CHUNKS |
| **File:Line** | `edgequake/crates/edgequake-api/src/services/postgres_chunk_lineage.rs:31` |
| **Entry** | `load_document_lineage_from_chunk_links` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_LINEAGE_LOAD_DOC_FROM_CHUNKS_191` |
| **Benchmark** | [benchmarks/191.md](./benchmarks/191.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-FAILED-CHUNKS-INSERT-192

<a id="data-pg-failed-chunks-insert-192"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-FAILED-CHUNKS-INSERT-192` |
| **Engine** | PG |
| **Domain** | FAILED-CHUNKS |
| **Operation** | INSERT |
| **File:Line** | `edgequake/crates/edgequake-storage/src/failed_chunks.rs:130` |
| **Entry** | `insert_failed_chunks` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_FAILED_CHUNKS_INSERT_192` |
| **Benchmark** | [benchmarks/192.md](./benchmarks/192.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-FAILED-CHUNKS-LIST-193

<a id="data-pg-failed-chunks-list-193"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-FAILED-CHUNKS-LIST-193` |
| **Engine** | PG |
| **Domain** | FAILED-CHUNKS |
| **Operation** | LIST |
| **File:Line** | `edgequake/crates/edgequake-storage/src/failed_chunks.rs:173` |
| **Entry** | `list_failed_chunks` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_FAILED_CHUNKS_LIST_193` |
| **Benchmark** | [benchmarks/193.md](./benchmarks/193.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-FAILED-CHUNKS-MARK-STATUS-194

<a id="data-pg-failed-chunks-mark-status-194"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-FAILED-CHUNKS-MARK-STATUS-194` |
| **Engine** | PG |
| **Domain** | FAILED-CHUNKS |
| **Operation** | MARK-STATUS |
| **File:Line** | `edgequake/crates/edgequake-storage/src/failed_chunks.rs:216` |
| **Entry** | `mark_chunk_status` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_FAILED_CHUNKS_MARK_STATUS_194` |
| **Benchmark** | [benchmarks/194.md](./benchmarks/194.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-RLS-SET-TENANT-CONTEXT-195

<a id="data-pg-rls-set-tenant-context-195"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-RLS-SET-TENANT-CONTEXT-195` |
| **Engine** | PG |
| **Domain** | RLS |
| **Operation** | SET-TENANT-CONTEXT |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/rls.rs:204` |
| **Entry** | `set_tenant_context_on_conn` |
| **Type** | SESSION |
| **Transactional** | Y |
| **Variables** | — |
| **Time** | O(1) |
| **Space** | O(1) |
| **I/O** | none |
| **Failure mode** | GUC leak → wrong recall/plan for next borrower |
| **Tests** | `data_layer_*` containing `DATA_PG_RLS_SET_TENANT_CONTEXT_195` |
| **Benchmark** | [benchmarks/195.md](./benchmarks/195.md) |

**Limits**

- SET LOCAL only inside short transactions
- Do not leak GUCs on pooled conns

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-RLS-CLEAR-TENANT-CONTEXT-196

<a id="data-pg-rls-clear-tenant-context-196"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-RLS-CLEAR-TENANT-CONTEXT-196` |
| **Engine** | PG |
| **Domain** | RLS |
| **Operation** | CLEAR-TENANT-CONTEXT |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/rls.rs:190` |
| **Entry** | `clear_tenant_context_on_conn` |
| **Type** | SESSION |
| **Transactional** | Y |
| **Variables** | — |
| **Time** | O(1) |
| **Space** | O(1) |
| **I/O** | none |
| **Failure mode** | GUC leak → wrong recall/plan for next borrower |
| **Tests** | `data_layer_*` containing `DATA_PG_RLS_CLEAR_TENANT_CONTEXT_196` |
| **Benchmark** | [benchmarks/196.md](./benchmarks/196.md) |

**Limits**

- SET LOCAL only inside short transactions
- Do not leak GUCs on pooled conns

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-POOL-ACQUIRE-CONNECT-197

<a id="data-pg-pool-acquire-connect-197"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-POOL-ACQUIRE-CONNECT-197` |
| **Engine** | PG |
| **Domain** | POOL |
| **Operation** | ACQUIRE-CONNECT |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs:1` |
| **Entry** | `PostgresPool connect/acquire` |
| **Type** | SESSION |
| **Transactional** | N |
| **Variables** | — |
| **Time** | O(1) |
| **Space** | O(1) |
| **I/O** | none |
| **Failure mode** | GUC leak → wrong recall/plan for next borrower |
| **Tests** | `data_layer_*` containing `DATA_PG_POOL_ACQUIRE_CONNECT_197` |
| **Benchmark** | [benchmarks/197.md](./benchmarks/197.md) |

**Limits**

- SET LOCAL only inside short transactions
- Do not leak GUCs on pooled conns

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-AUDIT-WRITE-EVENT-198

<a id="data-pg-audit-write-event-198"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-AUDIT-WRITE-EVENT-198` |
| **Engine** | PG |
| **Domain** | AUDIT |
| **Operation** | WRITE-EVENT |
| **File:Line** | `edgequake/crates/edgequake-audit/src/logger.rs:85` |
| **Entry** | `write_audit_event` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_AUDIT_WRITE_EVENT_198` |
| **Benchmark** | [benchmarks/198.md](./benchmarks/198.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-AUDIT-QUERY-LOGS-199

<a id="data-pg-audit-query-logs-199"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-AUDIT-QUERY-LOGS-199` |
| **Engine** | PG |
| **Domain** | AUDIT |
| **Operation** | QUERY-LOGS |
| **File:Line** | `edgequake/crates/edgequake-audit/src/logger.rs:172` |
| **Entry** | `query_audit_logs` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_AUDIT_QUERY_LOGS_199` |
| **Benchmark** | [benchmarks/199.md](./benchmarks/199.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONFIG-LOAD-LLM-DEFAULTS-200

<a id="data-pg-config-load-llm-defaults-200"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONFIG-LOAD-LLM-DEFAULTS-200` |
| **Engine** | PG |
| **Domain** | CONFIG |
| **Operation** | LOAD-LLM-DEFAULTS |
| **File:Line** | `edgequake/crates/edgequake-api/src/server_config_store.rs:165` |
| **Entry** | `load_llm_defaults` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONFIG_LOAD_LLM_DEFAULTS_200` |
| **Benchmark** | [benchmarks/200.md](./benchmarks/200.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONFIG-SAVE-LLM-DEFAULTS-201

<a id="data-pg-config-save-llm-defaults-201"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONFIG-SAVE-LLM-DEFAULTS-201` |
| **Engine** | PG |
| **Domain** | CONFIG |
| **Operation** | SAVE-LLM-DEFAULTS |
| **File:Line** | `edgequake/crates/edgequake-api/src/server_config_store.rs:179` |
| **Entry** | `save_llm_defaults` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONFIG_SAVE_LLM_DEFAULTS_201` |
| **Benchmark** | [benchmarks/201.md](./benchmarks/201.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONFIG-LOAD-PRIORITY-MODE-202

<a id="data-pg-config-load-priority-mode-202"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONFIG-LOAD-PRIORITY-MODE-202` |
| **Engine** | PG |
| **Domain** | CONFIG |
| **Operation** | LOAD-PRIORITY-MODE |
| **File:Line** | `edgequake/crates/edgequake-api/src/server_config_store.rs:202` |
| **Entry** | `load_priority_mode` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONFIG_LOAD_PRIORITY_MODE_202` |
| **Benchmark** | [benchmarks/202.md](./benchmarks/202.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-CONFIG-SAVE-PRIORITY-MODE-203

<a id="data-pg-config-save-priority-mode-203"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-CONFIG-SAVE-PRIORITY-MODE-203` |
| **Engine** | PG |
| **Domain** | CONFIG |
| **Operation** | SAVE-PRIORITY-MODE |
| **File:Line** | `edgequake/crates/edgequake-api/src/server_config_store.rs:216` |
| **Entry** | `save_priority_mode` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_CONFIG_SAVE_PRIORITY_MODE_203` |
| **Benchmark** | [benchmarks/203.md](./benchmarks/203.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KEYWORDS-CACHE-GET-204

<a id="data-pg-keywords-cache-get-204"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KEYWORDS-CACHE-GET-204` |
| **Engine** | PG |
| **Domain** | KEYWORDS |
| **Operation** | CACHE-GET |
| **File:Line** | `edgequake/crates/edgequake-query/src/keywords/cache.rs:264` |
| **Entry** | `KeywordCache::get` |
| **Type** | R |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_KEYWORDS_CACHE_GET_204` |
| **Benchmark** | [benchmarks/204.md](./benchmarks/204.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KEYWORDS-CACHE-SET-205

<a id="data-pg-keywords-cache-set-205"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KEYWORDS-CACHE-SET-205` |
| **Engine** | PG |
| **Domain** | KEYWORDS |
| **Operation** | CACHE-SET |
| **File:Line** | `edgequake/crates/edgequake-query/src/keywords/cache.rs:298` |
| **Entry** | `KeywordCache::set` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_KEYWORDS_CACHE_SET_205` |
| **Benchmark** | [benchmarks/205.md](./benchmarks/205.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KEYWORDS-CACHE-DELETE-206

<a id="data-pg-keywords-cache-delete-206"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KEYWORDS-CACHE-DELETE-206` |
| **Engine** | PG |
| **Domain** | KEYWORDS |
| **Operation** | CACHE-DELETE |
| **File:Line** | `edgequake/crates/edgequake-query/src/keywords/cache.rs:334` |
| **Entry** | `KeywordCache::delete` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_KEYWORDS_CACHE_DELETE_206` |
| **Benchmark** | [benchmarks/206.md](./benchmarks/206.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-KEYWORDS-CACHE-INIT-207

<a id="data-pg-keywords-cache-init-207"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-KEYWORDS-CACHE-INIT-207` |
| **Engine** | PG |
| **Domain** | KEYWORDS |
| **Operation** | CACHE-INIT |
| **File:Line** | `edgequake/crates/edgequake-query/src/keywords/cache.rs:233` |
| **Entry** | `KeywordCache::initialize` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_KEYWORDS_CACHE_INIT_207` |
| **Benchmark** | [benchmarks/207.md](./benchmarks/207.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-STATS-ENSURE-ROW-COUNT-208

<a id="data-pg-stats-ensure-row-count-208"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-STATS-ENSURE-ROW-COUNT-208` |
| **Engine** | PG |
| **Domain** | STATS |
| **Operation** | ENSURE-ROW-COUNT |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/row_count_stats.rs:45` |
| **Entry** | `ensure_row_count_stats` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_STATS_ENSURE_ROW_COUNT_208` |
| **Benchmark** | [benchmarks/208.md](./benchmarks/208.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-ID-ALLOCATE-DOCUMENT-209

<a id="data-pg-id-allocate-document-209"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-ID-ALLOCATE-DOCUMENT-209` |
| **Engine** | PG |
| **Domain** | ID |
| **Operation** | ALLOCATE-DOCUMENT |
| **File:Line** | `edgequake/crates/edgequake-storage/src/adapters/postgres/id_allocation.rs:1` |
| **Entry** | `allocate_document_id` |
| **Type** | W |
| **Transactional** | Y |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_ID_ALLOCATE_DOCUMENT_209` |
| **Benchmark** | [benchmarks/209.md](./benchmarks/209.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-INSPECT-CHECK-EXTENSIONS-210

<a id="data-pg-inspect-check-extensions-210"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-INSPECT-CHECK-EXTENSIONS-210` |
| **Engine** | PG |
| **Domain** | INSPECT |
| **Operation** | CHECK-EXTENSIONS |
| **File:Line** | `edgequake/crates/edgequake-api/src/storage_inspector.rs:416` |
| **Entry** | `check_extensions` |
| **Type** | R |
| **Transactional** | N |
| **Notes** | ADMIN |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_INSPECT_CHECK_EXTENSIONS_210` |
| **Benchmark** | [benchmarks/210.md](./benchmarks/210.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-INSPECT-CHECK-TABLES-211

<a id="data-pg-inspect-check-tables-211"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-INSPECT-CHECK-TABLES-211` |
| **Engine** | PG |
| **Domain** | INSPECT |
| **Operation** | CHECK-TABLES |
| **File:Line** | `edgequake/crates/edgequake-api/src/storage_inspector.rs:443` |
| **Entry** | `check_required_tables` |
| **Type** | R |
| **Transactional** | N |
| **Notes** | ADMIN |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_INSPECT_CHECK_TABLES_211` |
| **Benchmark** | [benchmarks/211.md](./benchmarks/211.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-INSPECT-CHECK-INVARIANTS-212

<a id="data-pg-inspect-check-invariants-212"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-INSPECT-CHECK-INVARIANTS-212` |
| **Engine** | PG |
| **Domain** | INSPECT |
| **Operation** | CHECK-INVARIANTS |
| **File:Line** | `edgequake/crates/edgequake-api/src/storage_inspector.rs:605` |
| **Entry** | `check_inv* family` |
| **Type** | R |
| **Transactional** | N |
| **Notes** | ADMIN integrity suite |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_INSPECT_CHECK_INVARIANTS_212` |
| **Benchmark** | [benchmarks/212.md](./benchmarks/212.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-INSPECT-APPLY-REPAIR-213

<a id="data-pg-inspect-apply-repair-213"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-INSPECT-APPLY-REPAIR-213` |
| **Engine** | PG |
| **Domain** | INSPECT |
| **Operation** | APPLY-REPAIR |
| **File:Line** | `edgequake/crates/edgequake-api/src/storage_inspector.rs:1187` |
| **Entry** | `apply_repair` |
| **Type** | W |
| **Transactional** | Y |
| **Notes** | ADMIN |
| **Variables** | N=table rows, K=result/limit, B=batch |
| **Time** | O(log N) indexed; O(N) scan |
| **Space** | O(K) |
| **I/O** | index or seq |
| **Failure mode** | timeout / lock wait / OOM on unbounded SELECT |
| **Tests** | `data_layer_*` containing `DATA_PG_INSPECT_APPLY_REPAIR_213` |
| **Benchmark** | [benchmarks/213.md](./benchmarks/213.md) |

**Limits**

- Use keyset pagination not large OFFSET
- statement_timeout recommended

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIGRATE-RUNNER-214

<a id="data-pg-schema-migrate-runner-214"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIGRATE-RUNNER-214` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIGRATE-RUNNER |
| **File:Line** | `edgequake/crates/edgequake-api/src/state/migration_bootstrap/mod.rs:1` |
| **Entry** | `sqlx migrate + reconcile hooks` |
| **Type** | DDL |
| **Transactional** | Y |
| **Notes** | 97 checksum-locked migrations |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIGRATE_RUNNER_214` |
| **Benchmark** | [benchmarks/214.md](./benchmarks/214.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-INIT-BASE-215

<a id="data-pg-schema-mig-init-base-215"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-INIT-BASE-215` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-INIT-BASE |
| **File:Line** | `edgequake/migrations/001_initial_schema.sql:1` |
| **Entry** | `migration 001` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_INIT_BASE_215` |
| **Benchmark** | [benchmarks/215.md](./benchmarks/215.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-TASKS-TABLE-216

<a id="data-pg-schema-mig-tasks-table-216"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-TASKS-TABLE-216` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-TASKS-TABLE |
| **File:Line** | `edgequake/migrations/002_add_tasks_table.sql:1` |
| **Entry** | `migration 002` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_TASKS_TABLE_216` |
| **Benchmark** | [benchmarks/216.md](./benchmarks/216.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-CONVERSATION-TABLE-217

<a id="data-pg-schema-mig-conversation-table-217"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-CONVERSATION-TABLE-217` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-CONVERSATION-TABLE |
| **File:Line** | `edgequake/migrations/004_add_conversation_history_table.sql:1` |
| **Entry** | `migration 004` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_CONVERSATION_TABLE_217` |
| **Benchmark** | [benchmarks/217.md](./benchmarks/217.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-AUDIT-LOG-218

<a id="data-pg-schema-mig-audit-log-218"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-AUDIT-LOG-218` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-AUDIT-LOG |
| **File:Line** | `edgequake/migrations/005_add_audit_log_table.sql:1` |
| **Entry** | `migration 005` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_AUDIT_LOG_218` |
| **Benchmark** | [benchmarks/218.md](./benchmarks/218.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-RLS-POLICIES-219

<a id="data-pg-schema-mig-rls-policies-219"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-RLS-POLICIES-219` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-RLS-POLICIES |
| **File:Line** | `edgequake/migrations/009_add_rls_policies.sql:1` |
| **Entry** | `migration 009` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_RLS_POLICIES_219` |
| **Benchmark** | [benchmarks/219.md](./benchmarks/219.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-AGE-GRAPH-220

<a id="data-pg-schema-mig-age-graph-220"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-AGE-GRAPH-220` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-AGE-GRAPH |
| **File:Line** | `edgequake/migrations/013_add_age_graph.sql:1` |
| **Entry** | `migration 013` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_AGE_GRAPH_220` |
| **Benchmark** | [benchmarks/220.md](./benchmarks/220.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-FULLTEXT-SEARCH-221

<a id="data-pg-schema-mig-fulltext-search-221"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-FULLTEXT-SEARCH-221` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-FULLTEXT-SEARCH |
| **File:Line** | `edgequake/migrations/015_add_fulltext_search.sql:1` |
| **Entry** | `migration 015` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_FULLTEXT_SEARCH_221` |
| **Benchmark** | [benchmarks/221.md](./benchmarks/221.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-FAILED-CHUNKS-222

<a id="data-pg-schema-mig-failed-chunks-222"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-FAILED-CHUNKS-222` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-FAILED-CHUNKS |
| **File:Line** | `edgequake/migrations/021_add_failed_chunks_table.sql:1` |
| **Entry** | `migration 021` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_FAILED_CHUNKS_222` |
| **Benchmark** | [benchmarks/222.md](./benchmarks/222.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-PDF-DOCUMENTS-223

<a id="data-pg-schema-mig-pdf-documents-223"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-PDF-DOCUMENTS-223` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-PDF-DOCUMENTS |
| **File:Line** | `edgequake/migrations/022_add_pdf_documents_table.sql:1` |
| **Entry** | `migration 022` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_PDF_DOCUMENTS_223` |
| **Benchmark** | [benchmarks/223.md](./benchmarks/223.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-VECTOR-BTREE-INDEXES-224

<a id="data-pg-schema-mig-vector-btree-indexes-224"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-VECTOR-BTREE-INDEXES-224` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-VECTOR-BTREE-INDEXES |
| **File:Line** | `edgequake/migrations/029_add_vector_btree_indexes.sql:1` |
| **Entry** | `migration 029` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_VECTOR_BTREE_INDEXES_224` |
| **Benchmark** | [benchmarks/224.md](./benchmarks/224.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-SOURCE-IDS-GIN-225

<a id="data-pg-schema-mig-source-ids-gin-225"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-SOURCE-IDS-GIN-225` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-SOURCE-IDS-GIN |
| **File:Line** | `edgequake/migrations/038_add_source_ids_gin_indexes.sql:1` |
| **Entry** | `migration 038` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_SOURCE_IDS_GIN_225` |
| **Benchmark** | [benchmarks/225.md](./benchmarks/225.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-CQRS-ENTITIES-226

<a id="data-pg-schema-mig-cqrs-entities-226"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-CQRS-ENTITIES-226` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-CQRS-ENTITIES |
| **File:Line** | `edgequake/migrations/039_cqrs_entities_schema.sql:1` |
| **Entry** | `migration 039` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_CQRS_ENTITIES_226` |
| **Benchmark** | [benchmarks/226.md](./benchmarks/226.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-CHUNK-LINEAGE-227

<a id="data-pg-schema-mig-chunk-lineage-227"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-CHUNK-LINEAGE-227` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-CHUNK-LINEAGE |
| **File:Line** | `edgequake/migrations/066_chunk_lineage_tables.sql:1` |
| **Entry** | `migration 066` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_CHUNK_LINEAGE_227` |
| **Benchmark** | [benchmarks/227.md](./benchmarks/227.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-AGE-INDEXES-CONSOLIDATE-228

<a id="data-pg-schema-mig-age-indexes-consolidate-228"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-AGE-INDEXES-CONSOLIDATE-228` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-AGE-INDEXES-CONSOLIDATE |
| **File:Line** | `edgequake/migrations/070_consolidate_age_indexes.sql:1` |
| **Entry** | `migration 070` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_AGE_INDEXES_CONSOLIDATE_228` |
| **Benchmark** | [benchmarks/228.md](./benchmarks/228.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-HNSW-OPTIMIZE-229

<a id="data-pg-schema-mig-hnsw-optimize-229"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-HNSW-OPTIMIZE-229` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-HNSW-OPTIMIZE |
| **File:Line** | `edgequake/migrations/071_hnsw_optimize.sql:1` |
| **Entry** | `migration 071` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_HNSW_OPTIMIZE_229` |
| **Benchmark** | [benchmarks/229.md](./benchmarks/229.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-HALFVEC-EMBEDDINGS-230

<a id="data-pg-schema-mig-halfvec-embeddings-230"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-HALFVEC-EMBEDDINGS-230` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-HALFVEC-EMBEDDINGS |
| **File:Line** | `edgequake/migrations/080_halfvec_embeddings_marker.sql:1` |
| **Entry** | `migration 080` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_HALFVEC_EMBEDDINGS_230` |
| **Benchmark** | [benchmarks/230.md](./benchmarks/230.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-DOCUMENT-ORIGINALS-231

<a id="data-pg-schema-mig-document-originals-231"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-DOCUMENT-ORIGINALS-231` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-DOCUMENT-ORIGINALS |
| **File:Line** | `edgequake/migrations/082_add_document_originals.sql:1` |
| **Entry** | `migration 082` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_DOCUMENT_ORIGINALS_231` |
| **Benchmark** | [benchmarks/231.md](./benchmarks/231.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-MM-ASSETS-232

<a id="data-pg-schema-mig-mm-assets-232"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-MM-ASSETS-232` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-MM-ASSETS |
| **File:Line** | `edgequake/migrations/084_add_document_mm_assets.sql:1` |
| **Entry** | `migration 084` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_MM_ASSETS_232` |
| **Benchmark** | [benchmarks/232.md](./benchmarks/232.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-TASK-LEASE-233

<a id="data-pg-schema-mig-task-lease-233"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-TASK-LEASE-233` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-TASK-LEASE |
| **File:Line** | `edgequake/migrations/088_task_lease_columns.sql:1` |
| **Entry** | `migration 088` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_TASK_LEASE_233` |
| **Benchmark** | [benchmarks/233.md](./benchmarks/233.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-MERGE-GRAPH-PROPS-234

<a id="data-pg-schema-mig-merge-graph-props-234"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-MERGE-GRAPH-PROPS-234` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-MERGE-GRAPH-PROPS |
| **File:Line** | `edgequake/migrations/090_eq_merge_graph_properties.sql:1` |
| **Entry** | `migration 090` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_MERGE_GRAPH_PROPS_234` |
| **Benchmark** | [benchmarks/234.md](./benchmarks/234.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.

## DATA-PG-SCHEMA-MIG-EQ-ID-DENORM-235

<a id="data-pg-schema-mig-eq-id-denorm-235"></a>

| Field | Value |
|---|---|
| **Ref ID** | `DATA-PG-SCHEMA-MIG-EQ-ID-DENORM-235` |
| **Engine** | PG |
| **Domain** | SCHEMA |
| **Operation** | MIG-EQ-ID-DENORM |
| **File:Line** | `edgequake/migrations/092_eq_id_denorm_marker.sql:1` |
| **Entry** | `migration 092` |
| **Type** | DDL |
| **Transactional** | Y |
| **Variables** | N=rows for index build |
| **Time** | O(N log N) HNSW build; O(N) btree |
| **Space** | maintenance_work_mem |
| **I/O** | full table scan build |
| **Failure mode** | lock contention ACCESS EXCLUSIVE; build OOM |
| **Tests** | `data_layer_*` containing `DATA_PG_SCHEMA_MIG_EQ_ID_DENORM_235` |
| **Benchmark** | [benchmarks/235.md](./benchmarks/235.md) |

**Limits**

- Never REINDEX on request path
- max_parallel_maintenance_workers for HNSW (CVE floor ≥0.8.2)
- Migrations checksum-locked

**Annotation (code)** — full `@dataop` block required above the operation; see Phase 1.
