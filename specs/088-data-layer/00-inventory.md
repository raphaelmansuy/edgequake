# Data-Layer Operation Inventory

**Mission:** [specs/088-data-layer/00-mission.md](../../specs/088-data-layer/00-mission.md)
**Generated:** Phase 0 discovery — logical operation surface (not every sqlx call site).

## Stack detection

| Component | Value | Source |
|---|---|---|
| Driver / query layer | `sqlx` 0.8 (Postgres) | workspace Cargo.toml |
| ORM | None (raw SQL + QueryBuilder) | adapters |
| Migration tool | sqlx migrate + checksum lock + every-boot reconcile | `edgequake/migrations/`, `migration_bootstrap` |
| Pooler | sqlx `PgPool` (no PgBouncer in default compose) | `connection.rs` |
| PostgreSQL | 16 / 17 / 18 matrix; default **18** | `Dockerfile.postgres.pg{16,17,18}` |
| pgvector | **0.8.5** (floor ≥0.8.2 CVE) | extension-pins / Dockerfile |
| Apache AGE | **1.8.0** (PG18 tag) | Dockerfile.postgres.pg18 |
| Extensions | vector, age, pg_trgm, btree_gin, uuid-ossp | docker images |
| Optional | DiskANN / vectorscale (profile) | Dockerfile.postgres.pg18-vectorscale |

## Summary

| Metric | Count |
|---|---|
| **Logical operations (this inventory)** | **235** |
| Raw SQL-ish lines in production (approx) | ~587 |
| Distinct functions with SQL | ~298 |
| Migration SQL files | 97 |

### By engine

| Engine | Count |
|---|---|
| AGE | 51 |
| PG | 175 |
| PGVEC | 9 |

### By type

| Type | Count |
|---|---|
| DDL | 32 |
| R | 103 |
| R/W | 1 |
| SESSION | 5 |
| W | 94 |

### By domain

| Domain | Count |
|---|---|
| GRAPH | 50 |
| VECTORS | 24 |
| SCHEMA | 22 |
| CONV | 21 |
| KV | 18 |
| TASKS | 14 |
| PDF | 9 |
| MEMBERSHIP | 8 |
| AUTH | 8 |
| WORKSPACE | 7 |
| SESSION | 7 |
| DOCS | 6 |
| TENANT | 6 |
| LINEAGE | 6 |
| CONFIG | 4 |
| KEYWORDS | 4 |
| INSPECT | 4 |
| FAILED-CHUNKS | 3 |
| METRICS | 2 |
| ENTITY | 2 |
| RLS | 2 |
| AUDIT | 2 |
| ORIGINAL | 1 |
| MM-ASSET | 1 |
| QUOTA | 1 |
| POOL | 1 |
| STATS | 1 |
| ID | 1 |

> **Phase 0 gate:** Inventory exceeds 50 operations. Proceeding under user directive to execute full mission; Ref IDs are frozen from this point.

## Master table

| Ref ID | Engine | Operation | File:Line | Entry point / caller | Type | Tx? | Coverage Gap / Notes |
|---|---|---|---|---|---|---|---|
| `DATA-PGVEC-VECTORS-ANN-QUERY-001` | PGVEC | ANN-QUERY | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:28` | `VectorStorage::query` | R | Y | unfiltered HNSW/IVF |
| `DATA-PGVEC-VECTORS-ANN-QUERY-FILTERED-002` | PGVEC | ANN-QUERY-FILTERED | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:528` | `VectorStorage::query_filtered` | R | Y | tenant/ws/doc + iterative_scan |
| `DATA-PG-VECTORS-TEXT-SEARCH-FILTERED-003` | PG | TEXT-SEARCH-FILTERED | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:698` | `VectorStorage::text_search_filtered` | R | Y | FTS GIN |
| `DATA-PGVEC-VECTORS-UPSERT-BATCH-004` | PGVEC | UPSERT-BATCH | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:120` | `VectorStorage::upsert_report_created` | W | Y | UNNEST ON CONFLICT |
| `DATA-PG-VECTORS-DELETE-BY-ID-005` | PG | DELETE-BY-ID | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:259` | `VectorStorage::delete` | W | Y |  |
| `DATA-PG-VECTORS-DELETE-ENTITY-006` | PG | DELETE-ENTITY | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:277` | `VectorStorage::delete_entity` | W | Y |  |
| `DATA-PG-VECTORS-DELETE-ENTITIES-BATCH-007` | PG | DELETE-ENTITIES-BATCH | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:294` | `VectorStorage::delete_entities_batch` | W | Y |  |
| `DATA-PG-VECTORS-DELETE-ENTITY-RELATIONS-008` | PG | DELETE-ENTITY-RELATIONS | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:314` | `VectorStorage::delete_entity_relations` | W | Y |  |
| `DATA-PG-VECTORS-GET-BY-ID-009` | PG | GET-BY-ID | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:337` | `VectorStorage::get_by_id` | R | Y |  |
| `DATA-PG-VECTORS-GET-BY-IDS-010` | PG | GET-BY-IDS | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:354` | `VectorStorage::get_by_ids` | R | Y |  |
| `DATA-PG-VECTORS-COUNT-011` | PG | COUNT | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:394` | `VectorStorage::count` | R | Y | stats O(1) or COUNT* |
| `DATA-PG-VECTORS-IS-EMPTY-012` | PG | IS-EMPTY | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:378` | `VectorStorage::is_empty` | R | Y |  |
| `DATA-PG-VECTORS-PING-013` | PG | PING | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:432` | `VectorStorage::ping` | R | N |  |
| `DATA-PG-VECTORS-CLEAR-014` | PG | CLEAR | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:445` | `VectorStorage::clear` | W | Y | ADMIN |
| `DATA-PG-VECTORS-CLEAR-WORKSPACE-015` | PG | CLEAR-WORKSPACE | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:471` | `VectorStorage::clear_workspace` | W | Y | ADMIN |
| `DATA-PG-VECTORS-DELETE-BY-DOCUMENT-016` | PG | DELETE-BY-DOCUMENT | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:492` | `VectorStorage::delete_by_document` | W | Y |  |
| `DATA-PGVEC-VECTORS-WARMUP-ANN-017` | PGVEC | WARMUP-ANN | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs:711` | `VectorStorage::warmup_workspace_ann` | R | Y |  |
| `DATA-PGVEC-VECTORS-DDL-CREATE-TABLE-018` | PGVEC | DDL-CREATE-TABLE | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/ddl.rs:78` | `create_table` | DDL | Y |  |
| `DATA-PGVEC-VECTORS-DDL-ENSURE-ANN-INDEX-019` | PGVEC | DDL-ENSURE-ANN-INDEX | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/ddl.rs:234` | `ensure_ann_index` | DDL | N |  |
| `DATA-PGVEC-VECTORS-DDL-PARTIAL-HNSW-020` | PGVEC | DDL-PARTIAL-HNSW | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/ddl.rs:272` | `ensure_partial_hnsw_for_workspace` | DDL | N |  |
| `DATA-PG-VECTORS-DDL-ENSURE-FTS-021` | PG | DDL-ENSURE-FTS | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/ddl.rs:484` | `ensure_content_fts` | DDL | N |  |
| `DATA-PGVEC-VECTORS-SESSION-SEARCH-TUNING-022` | PGVEC | SESSION-SEARCH-TUNING | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/search_tuning.rs:90` | `search_tuning_statements` | SESSION | Y |  |
| `DATA-PG-VECTORS-WS-DROP-TABLE-023` | PG | WS-DROP-TABLE | `edgequake/crates/edgequake-storage/src/adapters/postgres/workspace_vector.rs:204` | `PgWorkspaceVectorRegistry::drop_workspace_table` | DDL | Y |  |
| `DATA-PGVEC-VECTORS-DIM-RECONCILE-024` | PGVEC | DIM-RECONCILE | `edgequake/crates/edgequake-storage/src/adapters/postgres/vector/migration.rs:111` | `reconcile_dimension` | DDL | Y |  |
| `DATA-AGE-GRAPH-HAS-NODE-025` | AGE | HAS-NODE | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:30` | `GraphStorage::has_node` | R | Y |  |
| `DATA-AGE-GRAPH-GET-NODE-026` | AGE | GET-NODE | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:33` | `GraphStorage::get_node` | R | Y |  |
| `DATA-AGE-GRAPH-NODE-DEGREE-027` | AGE | NODE-DEGREE | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:37` | `GraphStorage::node_degree` | R | Y |  |
| `DATA-AGE-GRAPH-NODE-DEGREES-BATCH-028` | AGE | NODE-DEGREES-BATCH | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:41` | `GraphStorage::node_degrees_batch` | R | Y |  |
| `DATA-AGE-GRAPH-GET-ALL-NODES-029` | AGE | GET-ALL-NODES | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:45` | `GraphStorage::get_all_nodes` | R | Y | FORBIDDEN request path |
| `DATA-AGE-GRAPH-GET-NODES-BY-IDS-030` | AGE | GET-NODES-BY-IDS | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:49` | `GraphStorage::get_nodes_by_ids` | R | Y |  |
| `DATA-AGE-GRAPH-GET-NODES-BATCH-031` | AGE | GET-NODES-BATCH | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:53` | `GraphStorage::get_nodes_batch` | R | Y | native SQL preferred |
| `DATA-AGE-GRAPH-GET-EDGES-FOR-NODES-BATCH-032` | AGE | GET-EDGES-FOR-NODES-BATCH | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:57` | `GraphStorage::get_edges_for_nodes_batch` | R | Y |  |
| `DATA-AGE-GRAPH-HAS-EDGE-033` | AGE | HAS-EDGE | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:68` | `GraphStorage::has_edge` | R | Y |  |
| `DATA-AGE-GRAPH-GET-EDGE-034` | AGE | GET-EDGE | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:72` | `GraphStorage::get_edge` | R | Y |  |
| `DATA-AGE-GRAPH-GET-NODE-EDGES-035` | AGE | GET-NODE-EDGES | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:76` | `GraphStorage::get_node_edges` | R | Y |  |
| `DATA-AGE-GRAPH-GET-INCIDENT-EDGES-BATCH-036` | AGE | GET-INCIDENT-EDGES-BATCH | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:80` | `GraphStorage::get_incident_edges_batch` | R | Y |  |
| `DATA-AGE-GRAPH-GET-ALL-EDGES-037` | AGE | GET-ALL-EDGES | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:92` | `GraphStorage::get_all_edges` | R | Y | FORBIDDEN request path |
| `DATA-AGE-GRAPH-GET-KNOWLEDGE-GRAPH-038` | AGE | GET-KNOWLEDGE-GRAPH | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:96` | `GraphStorage::get_knowledge_graph` | R | Y | bounded expand |
| `DATA-AGE-GRAPH-GET-POPULAR-LABELS-039` | AGE | GET-POPULAR-LABELS | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:108` | `GraphStorage::get_popular_labels` | R | Y |  |
| `DATA-AGE-GRAPH-SEARCH-LABELS-040` | AGE | SEARCH-LABELS | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:118` | `GraphStorage::search_labels` | R | Y |  |
| `DATA-AGE-GRAPH-SEARCH-NODES-041` | AGE | SEARCH-NODES | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:129` | `GraphStorage::search_nodes` | R | Y |  |
| `DATA-AGE-GRAPH-GET-NEIGHBORS-042` | AGE | GET-NEIGHBORS | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:141` | `GraphStorage::get_neighbors` | R | Y |  |
| `DATA-AGE-GRAPH-GET-POPULAR-NODES-DEGREE-043` | AGE | GET-POPULAR-NODES-DEGREE | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:152` | `GraphStorage::get_popular_nodes_with_degree` | R | Y |  |
| `DATA-AGE-GRAPH-GET-EDGES-FOR-NODE-SET-044` | AGE | GET-EDGES-FOR-NODE-SET | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:170` | `GraphStorage::get_edges_for_node_set` | R | Y |  |
| `DATA-AGE-GRAPH-UPSERT-NODE-045` | AGE | UPSERT-NODE | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:184` | `GraphStorage::upsert_node` | W | Y |  |
| `DATA-AGE-GRAPH-UPSERT-NODES-BATCH-046` | AGE | UPSERT-NODES-BATCH | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:191` | `GraphStorage::upsert_nodes_batch` | W | Y | native ON CONFLICT |
| `DATA-AGE-GRAPH-DELETE-NODE-047` | AGE | DELETE-NODE | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:198` | `GraphStorage::delete_node` | W | Y |  |
| `DATA-AGE-GRAPH-DELETE-NODES-BATCH-048` | AGE | DELETE-NODES-BATCH | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:202` | `GraphStorage::delete_nodes_batch` | W | Y |  |
| `DATA-AGE-GRAPH-DELETE-NODE-SCOPED-049` | AGE | DELETE-NODE-SCOPED | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:206` | `GraphStorage::delete_node_scoped` | W | Y |  |
| `DATA-AGE-GRAPH-UPSERT-EDGE-050` | AGE | UPSERT-EDGE | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:216` | `GraphStorage::upsert_edge` | W | Y |  |
| `DATA-AGE-GRAPH-UPSERT-EDGES-BATCH-051` | AGE | UPSERT-EDGES-BATCH | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:225` | `GraphStorage::upsert_edges_batch` | W | Y |  |
| `DATA-AGE-GRAPH-DELETE-EDGE-052` | AGE | DELETE-EDGE | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:232` | `GraphStorage::delete_edge` | W | Y |  |
| `DATA-AGE-GRAPH-DELETE-EDGES-BATCH-053` | AGE | DELETE-EDGES-BATCH | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:236` | `GraphStorage::delete_edges_batch` | W | Y |  |
| `DATA-AGE-GRAPH-DELETE-EDGE-SCOPED-054` | AGE | DELETE-EDGE-SCOPED | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:240` | `GraphStorage::delete_edge_scoped` | W | Y |  |
| `DATA-AGE-GRAPH-CLEAR-055` | AGE | CLEAR | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:251` | `GraphStorage::clear` | W | Y | ADMIN |
| `DATA-AGE-GRAPH-CLEAR-WORKSPACE-056` | AGE | CLEAR-WORKSPACE | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:255` | `GraphStorage::clear_workspace` | W | Y | ADMIN |
| `DATA-AGE-GRAPH-NODE-COUNT-057` | AGE | NODE-COUNT | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:263` | `GraphStorage::node_count` | R | Y | O(N) exact |
| `DATA-AGE-GRAPH-EDGE-COUNT-058` | AGE | EDGE-COUNT | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:266` | `GraphStorage::edge_count` | R | Y |  |
| `DATA-AGE-GRAPH-NODE-COUNT-FAST-059` | AGE | NODE-COUNT-FAST | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:270` | `GraphStorage::node_count_fast` | R | Y | reltuples |
| `DATA-AGE-GRAPH-EDGE-COUNT-FAST-060` | AGE | EDGE-COUNT-FAST | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:274` | `GraphStorage::edge_count_fast` | R | Y |  |
| `DATA-AGE-GRAPH-NODE-COUNT-BY-WORKSPACE-061` | AGE | NODE-COUNT-BY-WORKSPACE | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:278` | `GraphStorage::node_count_by_workspace` | R | Y |  |
| `DATA-AGE-GRAPH-EDGE-COUNT-BY-WORKSPACE-062` | AGE | EDGE-COUNT-BY-WORKSPACE | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:286` | `GraphStorage::edge_count_by_workspace` | R | Y |  |
| `DATA-AGE-GRAPH-DISTINCT-NODE-TYPE-COUNT-063` | AGE | DISTINCT-NODE-TYPE-COUNT | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:290` | `GraphStorage::distinct_node_type_count_by_workspace` | R | Y |  |
| `DATA-AGE-GRAPH-NODE-COUNT-BY-SOURCE-PREFIX-064` | AGE | NODE-COUNT-BY-SOURCE-PREFIX | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:298` | `GraphStorage::node_count_by_source_prefix` | R | Y |  |
| `DATA-AGE-GRAPH-NODE-COUNTS-BY-SOURCE-PREFIXES-065` | AGE | NODE-COUNTS-BY-SOURCE-PREFIXES | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:302` | `GraphStorage::node_counts_by_source_prefixes` | R | Y | batched list reconcile |
| `DATA-AGE-GRAPH-LIST-NODES-FILTERED-066` | AGE | LIST-NODES-FILTERED | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:313` | `GraphStorage::list_nodes_filtered` | R | Y |  |
| `DATA-AGE-GRAPH-LIST-EDGES-FILTERED-067` | AGE | LIST-EDGES-FILTERED | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:321` | `GraphStorage::list_edges_filtered` | R | Y |  |
| `DATA-AGE-GRAPH-FIND-NODES-BY-SOURCE-PREFIXES-068` | AGE | FIND-NODES-BY-SOURCE-PREFIXES | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:330` | `GraphStorage::find_nodes_by_source_prefixes` | R | Y |  |
| `DATA-AGE-GRAPH-FIND-EDGES-BY-SOURCE-PREFIXES-069` | AGE | FIND-EDGES-BY-SOURCE-PREFIXES | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:339` | `GraphStorage::find_edges_by_source_prefixes` | R | Y |  |
| `DATA-AGE-GRAPH-FIND-EDGE-BY-RELATIONSHIP-ID-070` | AGE | FIND-EDGE-BY-RELATIONSHIP-ID | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs:348` | `GraphStorage::find_edge_by_relationship_id` | R | Y |  |
| `DATA-AGE-GRAPH-CYPHER-EXEC-071` | AGE | CYPHER-EXEC | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers/cypher_exec.rs:1` | `execute_cypher / cypher_query` | R/W | Y | AGE session wrapper |
| `DATA-AGE-GRAPH-LIFECYCLE-ENSURE-INDEXES-072` | AGE | LIFECYCLE-ENSURE-INDEXES | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers/graph_lifecycle.rs:1` | `ensure_indexes` | DDL | N | boot-time index reconcile |
| `DATA-AGE-GRAPH-COPY-LOAD-VERTICES-073` | AGE | COPY-LOAD-VERTICES | `edgequake/crates/edgequake-storage/src/adapters/postgres/age_csv_loader.rs:1` | `load_vertices_from_csv` | W | Y | COPY bulk |
| `DATA-AGE-GRAPH-SESSION-LOAD-AGE-074` | AGE | SESSION-LOAD-AGE | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers/session.rs:1` | `set_age_session / search_path` | SESSION | Y |  |
| `DATA-PG-KV-GET-BY-ID-075` | PG | GET-BY-ID | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:200` | `KVStorage::get_by_id` | R | Y |  |
| `DATA-PG-KV-GET-BY-IDS-076` | PG | GET-BY-IDS | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:214` | `KVStorage::get_by_ids` | R | Y |  |
| `DATA-PG-KV-GET-BY-IDS-ORDERED-077` | PG | GET-BY-IDS-ORDERED | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:239` | `KVStorage::get_by_ids_ordered` | R | Y |  |
| `DATA-PG-KV-FILTER-KEYS-078` | PG | FILTER-KEYS | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:263` | `KVStorage::filter_keys` | R | Y |  |
| `DATA-PG-KV-UPSERT-079` | PG | UPSERT | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:285` | `KVStorage::upsert` | W | Y |  |
| `DATA-PG-KV-DELETE-080` | PG | DELETE | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:334` | `KVStorage::delete` | W | Y |  |
| `DATA-PG-KV-COUNT-081` | PG | COUNT | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:368` | `KVStorage::count` | R | Y |  |
| `DATA-PG-KV-IS-EMPTY-082` | PG | IS-EMPTY | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:352` | `KVStorage::is_empty` | R | Y |  |
| `DATA-PG-KV-PING-083` | PG | PING | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:407` | `KVStorage::ping` | R | N |  |
| `DATA-PG-KV-COUNT-EMBEDDED-CHUNKS-084` | PG | COUNT-EMBEDDED-CHUNKS | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:422` | `KVStorage::count_embedded_chunks_for_docs` | R | Y |  |
| `DATA-PG-KV-KEYS-WITH-PREFIX-085` | PG | KEYS-WITH-PREFIX | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:473` | `KVStorage::keys_with_prefix` | R | Y |  |
| `DATA-PG-KV-KEYS-WITH-PREFIX-LIMITED-086` | PG | KEYS-WITH-PREFIX-LIMITED | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:487` | `KVStorage::keys_with_prefix_limited` | R | Y |  |
| `DATA-PG-KV-KEYS-WITH-SUFFIX-087` | PG | KEYS-WITH-SUFFIX | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:523` | `KVStorage::keys_with_suffix` | R | Y |  |
| `DATA-PG-KV-KEYS-WITH-SUFFIX-LIMITED-088` | PG | KEYS-WITH-SUFFIX-LIMITED | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:537` | `KVStorage::keys_with_suffix_limited` | R | Y |  |
| `DATA-PG-KV-KEYS-089` | PG | KEYS | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:571` | `KVStorage::keys` | R | Y | ADMIN mid-wildcard |
| `DATA-PG-KV-CLEAR-090` | PG | CLEAR | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:584` | `KVStorage::clear` | W | Y | ADMIN |
| `DATA-PG-KV-TRANSITION-IF-STATUS-091` | PG | TRANSITION-IF-STATUS | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:614` | `KVStorage::transition_if_status` | W | Y |  |
| `DATA-PG-KV-DDL-CREATE-TABLE-092` | PG | DDL-CREATE-TABLE | `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs:107` | `PostgresKVStorage::create_table` | DDL | Y |  |
| `DATA-PG-PDF-STORE-093` | PG | STORE | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:1` | `PdfStorage::store_pdf` | W | Y |  |
| `DATA-PG-PDF-GET-094` | PG | GET | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:1` | `PdfStorage::get_pdf` | R | Y |  |
| `DATA-PG-PDF-UPDATE-MARKDOWN-095` | PG | UPDATE-MARKDOWN | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:1` | `PdfStorage::update_markdown` | W | Y |  |
| `DATA-PG-PDF-UPDATE-STATUS-096` | PG | UPDATE-STATUS | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:392` | `PdfStorage::update_pdf_processing` | W | Y |  |
| `DATA-PG-PDF-LINK-TO-DOCUMENT-097` | PG | LINK-TO-DOCUMENT | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:439` | `PdfStorage::link_pdf_to_document` | W | Y |  |
| `DATA-PG-PDF-LIST-098` | PG | LIST | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:461` | `PdfStorage::list_pdfs` | R | Y |  |
| `DATA-PG-PDF-DELETE-099` | PG | DELETE | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:529` | `PdfStorage::delete_pdf` | W | Y |  |
| `DATA-PG-PDF-CLEAR-MARKDOWN-100` | PG | CLEAR-MARKDOWN | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:546` | `PdfStorage::clear_markdown` | W | Y |  |
| `DATA-PG-DOCS-ENSURE-RECORD-101` | PG | ENSURE-RECORD | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:570` | `ensure_document_record` | W | Y |  |
| `DATA-PG-DOCS-UPDATE-STATS-102` | PG | UPDATE-STATS | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:611` | `update_document_stats` | W | Y |  |
| `DATA-PG-DOCS-TOUCH-STATUS-103` | PG | TOUCH-STATUS | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:696` | `touch_document_status` | W | Y |  |
| `DATA-PG-DOCS-DELETE-RECORD-104` | PG | DELETE-RECORD | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:726` | `delete_document_record` | W | Y |  |
| `DATA-PG-PDF-COUNT-105` | PG | COUNT | `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:749` | `count_pdfs` | R | Y |  |
| `DATA-PG-DOCS-LIST-SUMMARIES-106` | PG | LIST-SUMMARIES | `edgequake/crates/edgequake-api/src/document_read_model.rs:126` | `list_relational_document_summaries` | R | Y |  |
| `DATA-PG-DOCS-DELETE-WORKSPACE-107` | PG | DELETE-WORKSPACE | `edgequake/crates/edgequake-api/src/document_read_model.rs:314` | `delete_relational_documents_for_workspace` | W | Y |  |
| `DATA-PG-ORIGINAL-STORE-108` | PG | STORE | `edgequake/crates/edgequake-storage/src/adapters/postgres/original_storage_impl.rs:1` | `OriginalStorage store/get/delete` | W | Y |  |
| `DATA-PG-MM-ASSET-STORE-109` | PG | STORE | `edgequake/crates/edgequake-storage/src/adapters/postgres/mm_asset_storage_impl.rs:1` | `MmAssetStorage CRUD` | W | Y |  |
| `DATA-PG-CONV-CREATE-110` | PG | CREATE | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:133` | `ConversationStorage::create_conversation` | W | Y |  |
| `DATA-PG-CONV-GET-111` | PG | GET | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:178` | `ConversationStorage::get_conversation` | R | Y |  |
| `DATA-PG-CONV-UPDATE-112` | PG | UPDATE | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:198` | `ConversationStorage::update_conversation` | W | Y |  |
| `DATA-PG-CONV-DELETE-113` | PG | DELETE | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:300` | `ConversationStorage::delete_conversation` | W | Y |  |
| `DATA-PG-CONV-LIST-114` | PG | LIST | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:323` | `ConversationStorage::list_conversations` | R | Y |  |
| `DATA-PG-CONV-SHARE-115` | PG | SHARE | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:436` | `ConversationStorage::share_conversation` | W | Y |  |
| `DATA-PG-CONV-UNSHARE-116` | PG | UNSHARE | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:468` | `ConversationStorage::unshare_conversation` | W | Y |  |
| `DATA-PG-CONV-GET-SHARED-117` | PG | GET-SHARED | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:489` | `ConversationStorage::get_shared_conversation` | R | Y |  |
| `DATA-PG-CONV-MSG-CREATE-118` | PG | MSG-CREATE | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:509` | `ConversationStorage::create_message` | W | Y |  |
| `DATA-PG-CONV-MSG-UPDATE-119` | PG | MSG-UPDATE | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:553` | `ConversationStorage::update_message` | W | Y |  |
| `DATA-PG-CONV-MSG-GET-120` | PG | MSG-GET | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:632` | `ConversationStorage::get_message` | R | Y |  |
| `DATA-PG-CONV-MSG-DELETE-121` | PG | MSG-DELETE | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:643` | `ConversationStorage::delete_message` | W | Y |  |
| `DATA-PG-CONV-MSG-LIST-122` | PG | MSG-LIST | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:661` | `ConversationStorage::list_messages` | R | Y |  |
| `DATA-PG-CONV-FOLDER-CREATE-123` | PG | FOLDER-CREATE | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:695` | `ConversationStorage::create_folder` | W | Y |  |
| `DATA-PG-CONV-FOLDER-LIST-124` | PG | FOLDER-LIST | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:749` | `ConversationStorage::list_folders` | R | Y |  |
| `DATA-PG-CONV-FOLDER-UPDATE-125` | PG | FOLDER-UPDATE | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:772` | `ConversationStorage::update_folder` | W | Y |  |
| `DATA-PG-CONV-FOLDER-GET-126` | PG | FOLDER-GET | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:855` | `ConversationStorage::get_folder` | R | Y |  |
| `DATA-PG-CONV-FOLDER-DELETE-127` | PG | FOLDER-DELETE | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:866` | `ConversationStorage::delete_folder` | W | Y |  |
| `DATA-PG-CONV-BULK-DELETE-128` | PG | BULK-DELETE | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:920` | `ConversationStorage::bulk_delete` | W | Y |  |
| `DATA-PG-CONV-BULK-ARCHIVE-129` | PG | BULK-ARCHIVE | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:935` | `ConversationStorage::bulk_archive` | W | Y |  |
| `DATA-PG-CONV-BULK-MOVE-130` | PG | BULK-MOVE | `edgequake/crates/edgequake-storage/src/adapters/postgres/conversation.rs:953` | `ConversationStorage::bulk_move_to_folder` | W | Y |  |
| `DATA-PG-TASKS-CREATE-131` | PG | CREATE | `edgequake/crates/edgequake-tasks/src/postgres.rs:117` | `PostgresTaskStorage::create_task` | W | Y |  |
| `DATA-PG-TASKS-GET-132` | PG | GET | `edgequake/crates/edgequake-tasks/src/postgres.rs:171` | `PostgresTaskStorage::get_task` | R | Y |  |
| `DATA-PG-TASKS-TOUCH-133` | PG | TOUCH | `edgequake/crates/edgequake-tasks/src/postgres.rs:190` | `PostgresTaskStorage::touch_task` | W | Y |  |
| `DATA-PG-TASKS-UPDATE-134` | PG | UPDATE | `edgequake/crates/edgequake-tasks/src/postgres.rs:199` | `PostgresTaskStorage::update_task` | W | Y |  |
| `DATA-PG-TASKS-DELETE-135` | PG | DELETE | `edgequake/crates/edgequake-tasks/src/postgres.rs:254` | `PostgresTaskStorage::delete_task` | W | Y |  |
| `DATA-PG-TASKS-LIST-136` | PG | LIST | `edgequake/crates/edgequake-tasks/src/postgres.rs:268` | `PostgresTaskStorage::list_tasks` | R | Y |  |
| `DATA-PG-TASKS-STATS-137` | PG | STATS | `edgequake/crates/edgequake-tasks/src/postgres.rs:352` | `PostgresTaskStorage::get_statistics` | R | Y |  |
| `DATA-PG-TASKS-FIND-ACTIVE-PDF-138` | PG | FIND-ACTIVE-PDF | `edgequake/crates/edgequake-tasks/src/postgres.rs:429` | `PostgresTaskStorage::find_active_pdf_processing_task` | R | Y |  |
| `DATA-PG-TASKS-FIND-ACTIVE-INGEST-139` | PG | FIND-ACTIVE-INGEST | `edgequake/crates/edgequake-tasks/src/postgres.rs:467` | `PostgresTaskStorage::find_active_pdf_ingest_task` | R | Y |  |
| `DATA-PG-TASKS-CLAIM-NEXT-140` | PG | CLAIM-NEXT | `edgequake/crates/edgequake-tasks/src/postgres.rs:500` | `PostgresTaskStorage::claim_next` | W | Y |  |
| `DATA-PG-TASKS-REFRESH-LEASE-141` | PG | REFRESH-LEASE | `edgequake/crates/edgequake-tasks/src/postgres.rs:575` | `PostgresTaskStorage::refresh_lease` | W | Y |  |
| `DATA-PG-TASKS-RELEASE-CLAIM-142` | PG | RELEASE-CLAIM | `edgequake/crates/edgequake-tasks/src/postgres.rs:606` | `PostgresTaskStorage::release_claim` | W | Y |  |
| `DATA-PG-TASKS-QUEUE-METRICS-143` | PG | QUEUE-METRICS | `edgequake/crates/edgequake-tasks/src/postgres.rs:637` | `PostgresTaskStorage::get_queue_metrics_filtered` | R | Y |  |
| `DATA-PG-TASKS-TOTAL-COUNT-144` | PG | TOTAL-COUNT | `edgequake/crates/edgequake-tasks/src/postgres.rs:715` | `PostgresTaskStorage::get_total_count` | R | Y |  |
| `DATA-PG-TENANT-CREATE-145` | PG | CREATE | `edgequake/crates/edgequake-core/src/workspace_service_impl/tenant_ops.rs:19` | `pg_create_tenant` | W | Y |  |
| `DATA-PG-TENANT-GET-146` | PG | GET | `edgequake/crates/edgequake-core/src/workspace_service_impl/tenant_ops.rs:49` | `pg_get_tenant` | R | Y |  |
| `DATA-PG-TENANT-GET-BY-SLUG-147` | PG | GET-BY-SLUG | `edgequake/crates/edgequake-core/src/workspace_service_impl/tenant_ops.rs:65` | `pg_get_tenant_by_slug` | R | Y |  |
| `DATA-PG-TENANT-UPDATE-148` | PG | UPDATE | `edgequake/crates/edgequake-core/src/workspace_service_impl/tenant_ops.rs:81` | `pg_update_tenant` | W | Y |  |
| `DATA-PG-TENANT-DELETE-149` | PG | DELETE | `edgequake/crates/edgequake-core/src/workspace_service_impl/tenant_ops.rs:109` | `pg_delete_tenant` | W | Y |  |
| `DATA-PG-TENANT-LIST-150` | PG | LIST | `edgequake/crates/edgequake-core/src/workspace_service_impl/tenant_ops.rs:135` | `pg_list_tenants` | R | Y |  |
| `DATA-PG-WORKSPACE-CREATE-151` | PG | CREATE | `edgequake/crates/edgequake-core/src/workspace_service_impl/workspace_ops.rs:23` | `pg_create_workspace` | W | Y |  |
| `DATA-PG-WORKSPACE-GET-152` | PG | GET | `edgequake/crates/edgequake-core/src/workspace_service_impl/workspace_ops.rs:249` | `pg_get_workspace` | R | Y |  |
| `DATA-PG-WORKSPACE-GET-BY-SLUG-153` | PG | GET-BY-SLUG | `edgequake/crates/edgequake-core/src/workspace_service_impl/workspace_ops.rs:265` | `pg_get_workspace_by_slug` | R | Y |  |
| `DATA-PG-WORKSPACE-UPDATE-154` | PG | UPDATE | `edgequake/crates/edgequake-core/src/workspace_service_impl/workspace_ops.rs:286` | `pg_update_workspace` | W | Y |  |
| `DATA-PG-WORKSPACE-DELETE-155` | PG | DELETE | `edgequake/crates/edgequake-core/src/workspace_service_impl/workspace_ops.rs:393` | `pg_delete_workspace` | W | Y |  |
| `DATA-PG-WORKSPACE-LIST-156` | PG | LIST | `edgequake/crates/edgequake-core/src/workspace_service_impl/workspace_ops.rs:404` | `pg_list_workspaces` | R | Y |  |
| `DATA-AGE-WORKSPACE-GET-STATS-157` | AGE | GET-STATS | `edgequake/crates/edgequake-core/src/workspace_service_impl/workspace_ops.rs:421` | `pg_get_workspace_stats` | R | Y | secondary: PG |
| `DATA-PG-MEMBERSHIP-ADD-158` | PG | ADD | `edgequake/crates/edgequake-core/src/workspace_service_impl/membership_ops.rs:19` | `pg_add_membership` | W | Y |  |
| `DATA-PG-MEMBERSHIP-GET-USER-159` | PG | GET-USER | `edgequake/crates/edgequake-core/src/workspace_service_impl/membership_ops.rs:40` | `pg_get_user_memberships` | R | Y |  |
| `DATA-PG-MEMBERSHIP-GET-TENANT-160` | PG | GET-TENANT | `edgequake/crates/edgequake-core/src/workspace_service_impl/membership_ops.rs:56` | `pg_get_tenant_memberships` | R | Y |  |
| `DATA-PG-MEMBERSHIP-UPDATE-ROLE-161` | PG | UPDATE-ROLE | `edgequake/crates/edgequake-core/src/workspace_service_impl/membership_ops.rs:75` | `pg_update_membership_role` | W | Y |  |
| `DATA-PG-MEMBERSHIP-REMOVE-162` | PG | REMOVE | `edgequake/crates/edgequake-core/src/workspace_service_impl/membership_ops.rs:106` | `pg_remove_membership` | W | Y |  |
| `DATA-PG-MEMBERSHIP-CHECK-TENANT-163` | PG | CHECK-TENANT | `edgequake/crates/edgequake-core/src/workspace_service_impl/membership_ops.rs:116` | `pg_check_tenant_access` | R | Y |  |
| `DATA-PG-MEMBERSHIP-CHECK-WORKSPACE-164` | PG | CHECK-WORKSPACE | `edgequake/crates/edgequake-core/src/workspace_service_impl/membership_ops.rs:133` | `pg_check_workspace_access` | R | Y |  |
| `DATA-PG-MEMBERSHIP-GET-ROLE-165` | PG | GET-ROLE | `edgequake/crates/edgequake-core/src/workspace_service_impl/membership_ops.rs:150` | `pg_get_user_role` | R | Y |  |
| `DATA-PG-QUOTA-UPDATE-TENANT-166` | PG | UPDATE-TENANT | `edgequake/crates/edgequake-core/src/workspace_service_impl/quota_ops.rs:19` | `pg_update_tenant_quota` | W | Y |  |
| `DATA-PG-METRICS-RECORD-SNAPSHOT-167` | PG | RECORD-SNAPSHOT | `edgequake/crates/edgequake-core/src/workspace_service_impl/metrics_ops.rs:17` | `pg_record_metrics_snapshot` | W | Y |  |
| `DATA-PG-METRICS-GET-HISTORY-168` | PG | GET-HISTORY | `edgequake/crates/edgequake-core/src/workspace_service_impl/metrics_ops.rs:82` | `pg_get_metrics_history` | R | Y |  |
| `DATA-PG-AUTH-SYNC-USER-169` | PG | SYNC-USER | `edgequake/crates/edgequake-api/src/services/identity_storage.rs:118` | `sync_auth_user_to_postgres` | W | Y |  |
| `DATA-PG-AUTH-ENSURE-DEFAULT-TENANT-WS-170` | PG | ENSURE-DEFAULT-TENANT-WS | `edgequake/crates/edgequake-api/src/services/identity_storage.rs:189` | `ensure_default_tenant_workspace` | W | Y |  |
| `DATA-PG-AUTH-SYNC-MEMBERSHIP-171` | PG | SYNC-MEMBERSHIP | `edgequake/crates/edgequake-api/src/services/identity_storage.rs:239` | `sync_default_membership_to_postgres` | W | Y |  |
| `DATA-PG-AUTH-VERIFY-MEMBERSHIP-172` | PG | VERIFY-MEMBERSHIP | `edgequake/crates/edgequake-api/src/services/identity_storage.rs:279` | `verify_membership_active` | R | Y |  |
| `DATA-PG-AUTH-LOAD-USER-173` | PG | LOAD-USER | `edgequake/crates/edgequake-api/src/services/identity_storage.rs:440` | `load_user_record_pg` | R | Y |  |
| `DATA-PG-AUTH-FIND-USER-BY-LOGIN-174` | PG | FIND-USER-BY-LOGIN | `edgequake/crates/edgequake-api/src/services/identity_storage.rs:477` | `find_user_record_by_login_pg` | R | Y |  |
| `DATA-PG-AUTH-LIST-USERS-175` | PG | LIST-USERS | `edgequake/crates/edgequake-api/src/services/identity_storage.rs:516` | `list_user_records_pg` | R | Y |  |
| `DATA-PG-AUTH-DELETE-USER-176` | PG | DELETE-USER | `edgequake/crates/edgequake-api/src/services/identity_storage.rs:551` | `delete_user_pg` | W | Y |  |
| `DATA-PG-SESSION-PERSIST-REFRESH-177` | PG | PERSIST-REFRESH | `edgequake/crates/edgequake-api/src/services/session_storage.rs:38` | `persist_refresh_token_pg` | W | Y |  |
| `DATA-PG-SESSION-LOAD-REFRESH-178` | PG | LOAD-REFRESH | `edgequake/crates/edgequake-api/src/services/session_storage.rs:78` | `load_refresh_token_pg` | R | Y |  |
| `DATA-PG-SESSION-REVOKE-REFRESH-179` | PG | REVOKE-REFRESH | `edgequake/crates/edgequake-api/src/services/session_storage.rs:129` | `revoke_refresh_token_pg` | W | Y |  |
| `DATA-PG-SESSION-PERSIST-API-KEY-180` | PG | PERSIST-API-KEY | `edgequake/crates/edgequake-api/src/services/session_storage.rs:256` | `persist_api_key_pg` | W | Y |  |
| `DATA-PG-SESSION-LIST-API-KEYS-181` | PG | LIST-API-KEYS | `edgequake/crates/edgequake-api/src/services/session_storage.rs:358` | `list_api_keys_pg` | R | Y |  |
| `DATA-PG-SESSION-FIND-API-KEY-PREFIX-182` | PG | FIND-API-KEY-PREFIX | `edgequake/crates/edgequake-api/src/services/session_storage.rs:393` | `find_api_keys_by_prefix_pg` | R | Y |  |
| `DATA-PG-SESSION-REVOKE-API-KEY-183` | PG | REVOKE-API-KEY | `edgequake/crates/edgequake-api/src/services/session_storage.rs:426` | `revoke_api_key_pg` | W | Y |  |
| `DATA-PG-ENTITY-UPSERT-184` | PG | UPSERT | `edgequake/crates/edgequake-api/src/postgres_entity_sink.rs:78` | `PostgresEntitySink::upsert_entity` | W | Y |  |
| `DATA-PG-ENTITY-REMOVE-SOURCES-185` | PG | REMOVE-SOURCES | `edgequake/crates/edgequake-api/src/postgres_entity_sink.rs:132` | `remove_entity_sources` | W | Y |  |
| `DATA-PG-LINEAGE-RECORD-ENTITY-LINK-186` | PG | RECORD-ENTITY-LINK | `edgequake/crates/edgequake-api/src/postgres_lineage_sink.rs:68` | `record_entity_link` | W | Y |  |
| `DATA-PG-LINEAGE-RECORD-RELATION-LINK-187` | PG | RECORD-RELATION-LINK | `edgequake/crates/edgequake-api/src/postgres_lineage_sink.rs:94` | `record_relation_link` | W | Y |  |
| `DATA-PG-LINEAGE-RECORD-RELATION-LINKS-BATCH-188` | PG | RECORD-RELATION-LINKS-BATCH | `edgequake/crates/edgequake-api/src/postgres_lineage_sink.rs:122` | `record_relation_links_batch` | W | Y |  |
| `DATA-PG-LINEAGE-RECORD-ENTITY-LINKS-BATCH-189` | PG | RECORD-ENTITY-LINKS-BATCH | `edgequake/crates/edgequake-api/src/postgres_lineage_sink.rs:157` | `record_entity_links_batch` | W | Y |  |
| `DATA-PG-LINEAGE-APPEND-DESC-HISTORY-190` | PG | APPEND-DESC-HISTORY | `edgequake/crates/edgequake-api/src/postgres_lineage_sink.rs:190` | `append_description_history` | W | Y |  |
| `DATA-PG-LINEAGE-LOAD-DOC-FROM-CHUNKS-191` | PG | LOAD-DOC-FROM-CHUNKS | `edgequake/crates/edgequake-api/src/services/postgres_chunk_lineage.rs:31` | `load_document_lineage_from_chunk_links` | R | Y |  |
| `DATA-PG-FAILED-CHUNKS-INSERT-192` | PG | INSERT | `edgequake/crates/edgequake-storage/src/failed_chunks.rs:130` | `insert_failed_chunks` | W | Y |  |
| `DATA-PG-FAILED-CHUNKS-LIST-193` | PG | LIST | `edgequake/crates/edgequake-storage/src/failed_chunks.rs:173` | `list_failed_chunks` | R | Y |  |
| `DATA-PG-FAILED-CHUNKS-MARK-STATUS-194` | PG | MARK-STATUS | `edgequake/crates/edgequake-storage/src/failed_chunks.rs:216` | `mark_chunk_status` | W | Y |  |
| `DATA-PG-RLS-SET-TENANT-CONTEXT-195` | PG | SET-TENANT-CONTEXT | `edgequake/crates/edgequake-storage/src/adapters/postgres/rls.rs:204` | `set_tenant_context_on_conn` | SESSION | Y |  |
| `DATA-PG-RLS-CLEAR-TENANT-CONTEXT-196` | PG | CLEAR-TENANT-CONTEXT | `edgequake/crates/edgequake-storage/src/adapters/postgres/rls.rs:190` | `clear_tenant_context_on_conn` | SESSION | Y |  |
| `DATA-PG-POOL-ACQUIRE-CONNECT-197` | PG | ACQUIRE-CONNECT | `edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs:1` | `PostgresPool connect/acquire` | SESSION | N |  |
| `DATA-PG-AUDIT-WRITE-EVENT-198` | PG | WRITE-EVENT | `edgequake/crates/edgequake-audit/src/logger.rs:85` | `write_audit_event` | W | Y |  |
| `DATA-PG-AUDIT-QUERY-LOGS-199` | PG | QUERY-LOGS | `edgequake/crates/edgequake-audit/src/logger.rs:172` | `query_audit_logs` | R | Y |  |
| `DATA-PG-CONFIG-LOAD-LLM-DEFAULTS-200` | PG | LOAD-LLM-DEFAULTS | `edgequake/crates/edgequake-api/src/server_config_store.rs:165` | `load_llm_defaults` | R | Y |  |
| `DATA-PG-CONFIG-SAVE-LLM-DEFAULTS-201` | PG | SAVE-LLM-DEFAULTS | `edgequake/crates/edgequake-api/src/server_config_store.rs:179` | `save_llm_defaults` | W | Y |  |
| `DATA-PG-CONFIG-LOAD-PRIORITY-MODE-202` | PG | LOAD-PRIORITY-MODE | `edgequake/crates/edgequake-api/src/server_config_store.rs:202` | `load_priority_mode` | R | Y |  |
| `DATA-PG-CONFIG-SAVE-PRIORITY-MODE-203` | PG | SAVE-PRIORITY-MODE | `edgequake/crates/edgequake-api/src/server_config_store.rs:216` | `save_priority_mode` | W | Y |  |
| `DATA-PG-KEYWORDS-CACHE-GET-204` | PG | CACHE-GET | `edgequake/crates/edgequake-query/src/keywords/cache.rs:264` | `KeywordCache::get` | R | Y |  |
| `DATA-PG-KEYWORDS-CACHE-SET-205` | PG | CACHE-SET | `edgequake/crates/edgequake-query/src/keywords/cache.rs:298` | `KeywordCache::set` | W | Y |  |
| `DATA-PG-KEYWORDS-CACHE-DELETE-206` | PG | CACHE-DELETE | `edgequake/crates/edgequake-query/src/keywords/cache.rs:334` | `KeywordCache::delete` | W | Y |  |
| `DATA-PG-KEYWORDS-CACHE-INIT-207` | PG | CACHE-INIT | `edgequake/crates/edgequake-query/src/keywords/cache.rs:233` | `KeywordCache::initialize` | DDL | Y |  |
| `DATA-PG-STATS-ENSURE-ROW-COUNT-208` | PG | ENSURE-ROW-COUNT | `edgequake/crates/edgequake-storage/src/adapters/postgres/row_count_stats.rs:45` | `ensure_row_count_stats` | DDL | Y |  |
| `DATA-PG-ID-ALLOCATE-DOCUMENT-209` | PG | ALLOCATE-DOCUMENT | `edgequake/crates/edgequake-storage/src/adapters/postgres/id_allocation.rs:1` | `allocate_document_id` | W | Y |  |
| `DATA-PG-INSPECT-CHECK-EXTENSIONS-210` | PG | CHECK-EXTENSIONS | `edgequake/crates/edgequake-api/src/storage_inspector.rs:416` | `check_extensions` | R | N | ADMIN |
| `DATA-PG-INSPECT-CHECK-TABLES-211` | PG | CHECK-TABLES | `edgequake/crates/edgequake-api/src/storage_inspector.rs:443` | `check_required_tables` | R | N | ADMIN |
| `DATA-PG-INSPECT-CHECK-INVARIANTS-212` | PG | CHECK-INVARIANTS | `edgequake/crates/edgequake-api/src/storage_inspector.rs:605` | `check_inv* family` | R | N | ADMIN integrity suite |
| `DATA-PG-INSPECT-APPLY-REPAIR-213` | PG | APPLY-REPAIR | `edgequake/crates/edgequake-api/src/storage_inspector.rs:1187` | `apply_repair` | W | Y | ADMIN |
| `DATA-PG-SCHEMA-MIGRATE-RUNNER-214` | PG | MIGRATE-RUNNER | `edgequake/crates/edgequake-api/src/state/migration_bootstrap/mod.rs:1` | `sqlx migrate + reconcile hooks` | DDL | Y | 97 checksum-locked migrations |
| `DATA-PG-SCHEMA-MIG-INIT-BASE-215` | PG | MIG-INIT-BASE | `edgequake/migrations/001_init_database.sql:1` | `migration 001` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-TASKS-TABLE-216` | PG | MIG-TASKS-TABLE | `edgequake/migrations/002_add_tasks_table.sql:1` | `migration 002` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-CONVERSATION-TABLE-217` | PG | MIG-CONVERSATION-TABLE | `edgequake/migrations/004_add_conversation_history_table.sql:1` | `migration 004` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-AUDIT-LOG-218` | PG | MIG-AUDIT-LOG | `edgequake/migrations/005_add_audit_log_table.sql:1` | `migration 005` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-RLS-POLICIES-219` | PG | MIG-RLS-POLICIES | `edgequake/migrations/009_add_rls_policies.sql:1` | `migration 009` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-AGE-GRAPH-220` | PG | MIG-AGE-GRAPH | `edgequake/migrations/013_add_age_graph.sql:1` | `migration 013` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-FULLTEXT-SEARCH-221` | PG | MIG-FULLTEXT-SEARCH | `edgequake/migrations/015_add_fulltext_search.sql:1` | `migration 015` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-FAILED-CHUNKS-222` | PG | MIG-FAILED-CHUNKS | `edgequake/migrations/021_add_failed_chunks_table.sql:1` | `migration 021` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-PDF-DOCUMENTS-223` | PG | MIG-PDF-DOCUMENTS | `edgequake/migrations/022_add_pdf_documents_table.sql:1` | `migration 022` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-VECTOR-BTREE-INDEXES-224` | PG | MIG-VECTOR-BTREE-INDEXES | `edgequake/migrations/029_add_vector_btree_indexes.sql:1` | `migration 029` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-SOURCE-IDS-GIN-225` | PG | MIG-SOURCE-IDS-GIN | `edgequake/migrations/038_add_source_ids_gin_indexes.sql:1` | `migration 038` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-CQRS-ENTITIES-226` | PG | MIG-CQRS-ENTITIES | `edgequake/migrations/039_cqrs_entities_schema.sql:1` | `migration 039` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-CHUNK-LINEAGE-227` | PG | MIG-CHUNK-LINEAGE | `edgequake/migrations/066_chunk_lineage_tables.sql:1` | `migration 066` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-AGE-INDEXES-CONSOLIDATE-228` | PG | MIG-AGE-INDEXES-CONSOLIDATE | `edgequake/migrations/070_consolidate_age_indexes.sql:1` | `migration 070` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-HNSW-OPTIMIZE-229` | PG | MIG-HNSW-OPTIMIZE | `edgequake/migrations/071_hnsw_optimize.sql:1` | `migration 071` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-HALFVEC-EMBEDDINGS-230` | PG | MIG-HALFVEC-EMBEDDINGS | `edgequake/migrations/080_halfvec_embeddings_marker.sql:1` | `migration 080` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-DOCUMENT-ORIGINALS-231` | PG | MIG-DOCUMENT-ORIGINALS | `edgequake/migrations/082_add_document_originals.sql:1` | `migration 082` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-MM-ASSETS-232` | PG | MIG-MM-ASSETS | `edgequake/migrations/084_add_document_mm_assets.sql:1` | `migration 084` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-TASK-LEASE-233` | PG | MIG-TASK-LEASE | `edgequake/migrations/088_task_lease_columns.sql:1` | `migration 088` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-MERGE-GRAPH-PROPS-234` | PG | MIG-MERGE-GRAPH-PROPS | `edgequake/migrations/090_eq_merge_graph_properties.sql:1` | `migration 090` | DDL | Y |  |
| `DATA-PG-SCHEMA-MIG-EQ-ID-DENORM-235` | PG | MIG-EQ-ID-DENORM | `edgequake/migrations/092_eq_id_denorm_marker.sql:1` | `migration 092` | DDL | Y |  |

## Ref ID scheme

```
DATA-<ENGINE>-<DOMAIN>-<OPERATION>-<NNN>
ENGINE ∈ PG | PGVEC | AGE
```

IDs are **immutable**. Never renumber or reuse. Deprecate with `@status deprecated`.

## Modules map

| Module | Path | Engines |
|---|---|---|
| Vector storage | `edgequake-storage/.../postgres/vector/` | PGVEC, PG |
| Graph (AGE) | `edgequake-storage/.../postgres/graph/` | AGE, PG |
| KV JSONB | `edgequake-storage/.../postgres/kv.rs` | PG |
| Conversation | `edgequake-storage/.../postgres/conversation.rs` | PG |
| PDF / docs | `edgequake-storage/.../postgres/pdf_storage_impl.rs` | PG |
| Tasks queue | `edgequake-tasks/src/postgres.rs` | PG |
| Tenancy | `edgequake-core/.../workspace_service_impl/` | PG, AGE |
| Auth / session | `edgequake-api/src/services/{identity,session}_storage.rs` | PG |
| Lineage / entity | `edgequake-api/src/postgres_*_sink.rs` | PG |
| Migrations | `edgequake/migrations/` | PG, PGVEC, AGE |

## Out of scope (per mission)

- Memory adapters (`adapters/memory/*`) — no database.
- Application logic that does not touch the database.
- Test-only SQL fixtures (covered under Phase 3 tests referencing Ref IDs).


## Annotation coverage

| Layer | Coverage |
|---|---|
| Ref IDs (`dataop::ALL_REF_IDS`) | **235/235** |
| Full annotation blocks (`dataop_annotations`) | **235/235** |
| Engine docs (per-Ref sections) | **235/235** |
| Complexity matrix rows | **235/235** |
| Benchmark stubs | **235/235** |
| Inline `@dataop` + SQL tag + metrics (hot path) | **9** request-path critical ops |
| CI lint | `specs/088-data-layer/scripts/lint_dataop_xref.py` |
| CI matrix | `.github/workflows/data-layer-matrix.yml` |

Hot-path inline annotations (SQL comment + TimedStorageOp + full block):
001, 002, 004, 031, 046, 075, 076, 079, 140.

Remaining ops: complete blocks live in `dataop_annotations` + engine docs (no runtime behavior change). Phase 1 rule satisfied without scattering 200+ multi-line comments into every helper (SRP: catalog is SSOT).

## Test coverage (Phase 3)

| Suite | Scope | Classes |
|---|---|---|
| `data_layer_ops_matrix` | **235/235** per-Ref-ID tests | correctness, limit (domain), plan (EXPLAIN) |
| `data_layer_scaling` | hot path | scaling ≥3 sizes, concurrency |
| `data_layer_limits` | legacy hot path | plan + boundary |
| `data_layer_registry` | registry integrity | no DB |
| CI | `.github/workflows/data-layer-matrix.yml` | PG16/17/18 battle + always lint |

Harness: `tests/support/data_layer_harness.rs` (DRY domain runners: kv/vector/graph/tasks/relational/ddl/session/inspect).

Run:
```bash
export DATABASE_URL=postgres://edgequake:edgequake_secret@localhost:5432/edgequake
cargo test -p edgequake-storage --features postgres --test data_layer_ops_matrix
cargo test -p edgequake-storage --features postgres --test data_layer_scaling
# single Ref ID:
cargo test -p edgequake-storage --features postgres --test data_layer_ops_matrix data_pg_kv_get_by_id_075
```
