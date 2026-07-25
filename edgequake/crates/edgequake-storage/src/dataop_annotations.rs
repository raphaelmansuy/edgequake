//! Complete @dataop annotation blocks for every Ref ID (SPEC-088).
//! Inline code annotations on hot paths; this catalog is SSOT for the rest.
//! Lint verifies every inventory Ref has an entry here.

/// Full annotation block text for a Ref ID, if registered.
pub fn annotation_block(ref_id: &str) -> Option<&'static str> {
    ANNOTATIONS
        .iter()
        .find(|(id, _)| *id == ref_id)
        .map(|(_, b)| *b)
}

/// Number of catalogued annotation blocks.
pub fn annotation_count() -> usize {
    ANNOTATIONS.len()
}

const ANNOTATIONS: &[(&str, &str)] = &[
    (
        "DATA-PGVEC-VECTORS-ANN-QUERY-001",
        r###"/**
 * @dataop      DATA-PGVEC-VECTORS-ANN-QUERY-001
 * @engine      pgvector 0.8.x
 * @intent      ANN-QUERY via VectorStorage::query
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/001.md
 * @limits      type=R; transactional=Y; unfiltered HNSW/IVF
 * @scaling     see specs/088-data-layer/benchmarks/001.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/pgvector.md#data-pgvec-vectors-ann-query-001
 */"###,
    ),
    (
        "DATA-PGVEC-VECTORS-ANN-QUERY-FILTERED-002",
        r###"/**
 * @dataop      DATA-PGVEC-VECTORS-ANN-QUERY-FILTERED-002
 * @engine      pgvector 0.8.x
 * @intent      ANN-QUERY-FILTERED via VectorStorage::query_filtered
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/002.md
 * @limits      type=R; transactional=Y; tenant/ws/doc + iterative_scan
 * @scaling     see specs/088-data-layer/benchmarks/002.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/pgvector.md#data-pgvec-vectors-ann-query-filtered-002
 */"###,
    ),
    (
        "DATA-PG-VECTORS-TEXT-SEARCH-FILTERED-003",
        r###"/**
 * @dataop      DATA-PG-VECTORS-TEXT-SEARCH-FILTERED-003
 * @engine      postgres
 * @intent      TEXT-SEARCH-FILTERED via VectorStorage::text_search_filtered
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/003.md
 * @limits      type=R; transactional=Y; FTS GIN
 * @scaling     see specs/088-data-layer/benchmarks/003.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-vectors-text-search-filtered-003
 */"###,
    ),
    (
        "DATA-PGVEC-VECTORS-UPSERT-BATCH-004",
        r###"/**
 * @dataop      DATA-PGVEC-VECTORS-UPSERT-BATCH-004
 * @engine      pgvector 0.8.x
 * @intent      UPSERT-BATCH via VectorStorage::upsert_report_created
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/004.md
 * @limits      type=W; transactional=Y; UNNEST ON CONFLICT
 * @scaling     see specs/088-data-layer/benchmarks/004.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/pgvector.md#data-pgvec-vectors-upsert-batch-004
 */"###,
    ),
    (
        "DATA-PG-VECTORS-DELETE-BY-ID-005",
        r###"/**
 * @dataop      DATA-PG-VECTORS-DELETE-BY-ID-005
 * @engine      postgres
 * @intent      DELETE-BY-ID via VectorStorage::delete
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/005.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/005.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-vectors-delete-by-id-005
 */"###,
    ),
    (
        "DATA-PG-VECTORS-DELETE-ENTITY-006",
        r###"/**
 * @dataop      DATA-PG-VECTORS-DELETE-ENTITY-006
 * @engine      postgres
 * @intent      DELETE-ENTITY via VectorStorage::delete_entity
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/006.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/006.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-vectors-delete-entity-006
 */"###,
    ),
    (
        "DATA-PG-VECTORS-DELETE-ENTITIES-BATCH-007",
        r###"/**
 * @dataop      DATA-PG-VECTORS-DELETE-ENTITIES-BATCH-007
 * @engine      postgres
 * @intent      DELETE-ENTITIES-BATCH via VectorStorage::delete_entities_batch
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/007.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/007.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-vectors-delete-entities-batch-007
 */"###,
    ),
    (
        "DATA-PG-VECTORS-DELETE-ENTITY-RELATIONS-008",
        r###"/**
 * @dataop      DATA-PG-VECTORS-DELETE-ENTITY-RELATIONS-008
 * @engine      postgres
 * @intent      DELETE-ENTITY-RELATIONS via VectorStorage::delete_entity_relations
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/008.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/008.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-vectors-delete-entity-relations-008
 */"###,
    ),
    (
        "DATA-PG-VECTORS-GET-BY-ID-009",
        r###"/**
 * @dataop      DATA-PG-VECTORS-GET-BY-ID-009
 * @engine      postgres
 * @intent      GET-BY-ID via VectorStorage::get_by_id
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/009.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/009.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-vectors-get-by-id-009
 */"###,
    ),
    (
        "DATA-PG-VECTORS-GET-BY-IDS-010",
        r###"/**
 * @dataop      DATA-PG-VECTORS-GET-BY-IDS-010
 * @engine      postgres
 * @intent      GET-BY-IDS via VectorStorage::get_by_ids
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/010.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/010.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-vectors-get-by-ids-010
 */"###,
    ),
    (
        "DATA-PG-VECTORS-COUNT-011",
        r###"/**
 * @dataop      DATA-PG-VECTORS-COUNT-011
 * @engine      postgres
 * @intent      COUNT via VectorStorage::count
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/011.md
 * @limits      type=R; transactional=Y; stats O(1) or COUNT*
 * @scaling     see specs/088-data-layer/benchmarks/011.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-vectors-count-011
 */"###,
    ),
    (
        "DATA-PG-VECTORS-IS-EMPTY-012",
        r###"/**
 * @dataop      DATA-PG-VECTORS-IS-EMPTY-012
 * @engine      postgres
 * @intent      IS-EMPTY via VectorStorage::is_empty
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/012.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/012.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-vectors-is-empty-012
 */"###,
    ),
    (
        "DATA-PG-VECTORS-PING-013",
        r###"/**
 * @dataop      DATA-PG-VECTORS-PING-013
 * @engine      postgres
 * @intent      PING via VectorStorage::ping
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/013.md
 * @limits      type=R; transactional=N; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/013.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-vectors-ping-013
 */"###,
    ),
    (
        "DATA-PG-VECTORS-CLEAR-014",
        r###"/**
 * @dataop      DATA-PG-VECTORS-CLEAR-014
 * @engine      postgres
 * @intent      CLEAR via VectorStorage::clear
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/014.md
 * @limits      type=W; transactional=Y; ADMIN
 * @scaling     see specs/088-data-layer/benchmarks/014.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-vectors-clear-014
 */"###,
    ),
    (
        "DATA-PG-VECTORS-CLEAR-WORKSPACE-015",
        r###"/**
 * @dataop      DATA-PG-VECTORS-CLEAR-WORKSPACE-015
 * @engine      postgres
 * @intent      CLEAR-WORKSPACE via VectorStorage::clear_workspace
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/015.md
 * @limits      type=W; transactional=Y; ADMIN
 * @scaling     see specs/088-data-layer/benchmarks/015.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-vectors-clear-workspace-015
 */"###,
    ),
    (
        "DATA-PG-VECTORS-DELETE-BY-DOCUMENT-016",
        r###"/**
 * @dataop      DATA-PG-VECTORS-DELETE-BY-DOCUMENT-016
 * @engine      postgres
 * @intent      DELETE-BY-DOCUMENT via VectorStorage::delete_by_document
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/016.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/016.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-vectors-delete-by-document-016
 */"###,
    ),
    (
        "DATA-PGVEC-VECTORS-WARMUP-ANN-017",
        r###"/**
 * @dataop      DATA-PGVEC-VECTORS-WARMUP-ANN-017
 * @engine      pgvector 0.8.x
 * @intent      WARMUP-ANN via VectorStorage::warmup_workspace_ann
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/017.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/017.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/pgvector.md#data-pgvec-vectors-warmup-ann-017
 */"###,
    ),
    (
        "DATA-PGVEC-VECTORS-DDL-CREATE-TABLE-018",
        r###"/**
 * @dataop      DATA-PGVEC-VECTORS-DDL-CREATE-TABLE-018
 * @engine      pgvector 0.8.x
 * @intent      DDL-CREATE-TABLE via create_table
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/018.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/018.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/pgvector.md#data-pgvec-vectors-ddl-create-table-018
 */"###,
    ),
    (
        "DATA-PGVEC-VECTORS-DDL-ENSURE-ANN-INDEX-019",
        r###"/**
 * @dataop      DATA-PGVEC-VECTORS-DDL-ENSURE-ANN-INDEX-019
 * @engine      pgvector 0.8.x
 * @intent      DDL-ENSURE-ANN-INDEX via ensure_ann_index
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/019.md
 * @limits      type=DDL; transactional=N; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/019.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/pgvector.md#data-pgvec-vectors-ddl-ensure-ann-index-019
 */"###,
    ),
    (
        "DATA-PGVEC-VECTORS-DDL-PARTIAL-HNSW-020",
        r###"/**
 * @dataop      DATA-PGVEC-VECTORS-DDL-PARTIAL-HNSW-020
 * @engine      pgvector 0.8.x
 * @intent      DDL-PARTIAL-HNSW via ensure_partial_hnsw_for_workspace
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/020.md
 * @limits      type=DDL; transactional=N; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/020.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/pgvector.md#data-pgvec-vectors-ddl-partial-hnsw-020
 */"###,
    ),
    (
        "DATA-PG-VECTORS-DDL-ENSURE-FTS-021",
        r###"/**
 * @dataop      DATA-PG-VECTORS-DDL-ENSURE-FTS-021
 * @engine      postgres
 * @intent      DDL-ENSURE-FTS via ensure_content_fts
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/021.md
 * @limits      type=DDL; transactional=N; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/021.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-vectors-ddl-ensure-fts-021
 */"###,
    ),
    (
        "DATA-PGVEC-VECTORS-SESSION-SEARCH-TUNING-022",
        r###"/**
 * @dataop      DATA-PGVEC-VECTORS-SESSION-SEARCH-TUNING-022
 * @engine      pgvector 0.8.x
 * @intent      SESSION-SEARCH-TUNING via search_tuning_statements
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/022.md
 * @limits      type=SESSION; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/022.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/pgvector.md#data-pgvec-vectors-session-search-tuning-022
 */"###,
    ),
    (
        "DATA-PG-VECTORS-WS-DROP-TABLE-023",
        r###"/**
 * @dataop      DATA-PG-VECTORS-WS-DROP-TABLE-023
 * @engine      postgres
 * @intent      WS-DROP-TABLE via PgWorkspaceVectorRegistry::drop_workspace_table
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/023.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/023.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-vectors-ws-drop-table-023
 */"###,
    ),
    (
        "DATA-PGVEC-VECTORS-DIM-RECONCILE-024",
        r###"/**
 * @dataop      DATA-PGVEC-VECTORS-DIM-RECONCILE-024
 * @engine      pgvector 0.8.x
 * @intent      DIM-RECONCILE via reconcile_dimension
 * @tables      domain VECTORS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/024.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/024.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/pgvector.md#data-pgvec-vectors-dim-reconcile-024
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-HAS-NODE-025",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-HAS-NODE-025
 * @engine      apache_age 1.8
 * @intent      HAS-NODE via GraphStorage::has_node
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/025.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/025.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-has-node-025
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-GET-NODE-026",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-GET-NODE-026
 * @engine      apache_age 1.8
 * @intent      GET-NODE via GraphStorage::get_node
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/026.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/026.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-get-node-026
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-NODE-DEGREE-027",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-NODE-DEGREE-027
 * @engine      apache_age 1.8
 * @intent      NODE-DEGREE via GraphStorage::node_degree
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/027.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/027.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-node-degree-027
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-NODE-DEGREES-BATCH-028",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-NODE-DEGREES-BATCH-028
 * @engine      apache_age 1.8
 * @intent      NODE-DEGREES-BATCH via GraphStorage::node_degrees_batch
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/028.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/028.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-node-degrees-batch-028
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-GET-ALL-NODES-029",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-GET-ALL-NODES-029
 * @engine      apache_age 1.8
 * @intent      GET-ALL-NODES via GraphStorage::get_all_nodes
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/029.md
 * @limits      type=R; transactional=Y; FORBIDDEN request path
 * @scaling     see specs/088-data-layer/benchmarks/029.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-get-all-nodes-029
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-GET-NODES-BY-IDS-030",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-GET-NODES-BY-IDS-030
 * @engine      apache_age 1.8
 * @intent      GET-NODES-BY-IDS via GraphStorage::get_nodes_by_ids
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/030.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/030.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-get-nodes-by-ids-030
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-GET-NODES-BATCH-031",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-GET-NODES-BATCH-031
 * @engine      apache_age 1.8
 * @intent      GET-NODES-BATCH via GraphStorage::get_nodes_batch
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/031.md
 * @limits      type=R; transactional=Y; native SQL preferred
 * @scaling     see specs/088-data-layer/benchmarks/031.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-get-nodes-batch-031
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-GET-EDGES-FOR-NODES-BATCH-032",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-GET-EDGES-FOR-NODES-BATCH-032
 * @engine      apache_age 1.8
 * @intent      GET-EDGES-FOR-NODES-BATCH via GraphStorage::get_edges_for_nodes_batch
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/032.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/032.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-get-edges-for-nodes-batch-032
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-HAS-EDGE-033",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-HAS-EDGE-033
 * @engine      apache_age 1.8
 * @intent      HAS-EDGE via GraphStorage::has_edge
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/033.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/033.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-has-edge-033
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-GET-EDGE-034",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-GET-EDGE-034
 * @engine      apache_age 1.8
 * @intent      GET-EDGE via GraphStorage::get_edge
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/034.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/034.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-get-edge-034
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-GET-NODE-EDGES-035",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-GET-NODE-EDGES-035
 * @engine      apache_age 1.8
 * @intent      GET-NODE-EDGES via GraphStorage::get_node_edges
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/035.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/035.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-get-node-edges-035
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-GET-INCIDENT-EDGES-BATCH-036",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-GET-INCIDENT-EDGES-BATCH-036
 * @engine      apache_age 1.8
 * @intent      GET-INCIDENT-EDGES-BATCH via GraphStorage::get_incident_edges_batch
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/036.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/036.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-get-incident-edges-batch-036
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-GET-ALL-EDGES-037",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-GET-ALL-EDGES-037
 * @engine      apache_age 1.8
 * @intent      GET-ALL-EDGES via GraphStorage::get_all_edges
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/037.md
 * @limits      type=R; transactional=Y; FORBIDDEN request path
 * @scaling     see specs/088-data-layer/benchmarks/037.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-get-all-edges-037
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-GET-KNOWLEDGE-GRAPH-038",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-GET-KNOWLEDGE-GRAPH-038
 * @engine      apache_age 1.8
 * @intent      GET-KNOWLEDGE-GRAPH via GraphStorage::get_knowledge_graph
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/038.md
 * @limits      type=R; transactional=Y; bounded expand
 * @scaling     see specs/088-data-layer/benchmarks/038.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-get-knowledge-graph-038
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-GET-POPULAR-LABELS-039",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-GET-POPULAR-LABELS-039
 * @engine      apache_age 1.8
 * @intent      GET-POPULAR-LABELS via GraphStorage::get_popular_labels
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/039.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/039.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-get-popular-labels-039
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-SEARCH-LABELS-040",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-SEARCH-LABELS-040
 * @engine      apache_age 1.8
 * @intent      SEARCH-LABELS via GraphStorage::search_labels
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/040.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/040.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-search-labels-040
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-SEARCH-NODES-041",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-SEARCH-NODES-041
 * @engine      apache_age 1.8
 * @intent      SEARCH-NODES via GraphStorage::search_nodes
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/041.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/041.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-search-nodes-041
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-GET-NEIGHBORS-042",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-GET-NEIGHBORS-042
 * @engine      apache_age 1.8
 * @intent      GET-NEIGHBORS via GraphStorage::get_neighbors
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/042.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/042.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-get-neighbors-042
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-GET-POPULAR-NODES-DEGREE-043",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-GET-POPULAR-NODES-DEGREE-043
 * @engine      apache_age 1.8
 * @intent      GET-POPULAR-NODES-DEGREE via GraphStorage::get_popular_nodes_with_degree
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/043.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/043.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-get-popular-nodes-degree-043
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-GET-EDGES-FOR-NODE-SET-044",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-GET-EDGES-FOR-NODE-SET-044
 * @engine      apache_age 1.8
 * @intent      GET-EDGES-FOR-NODE-SET via GraphStorage::get_edges_for_node_set
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/044.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/044.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-get-edges-for-node-set-044
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-UPSERT-NODE-045",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-UPSERT-NODE-045
 * @engine      apache_age 1.8
 * @intent      UPSERT-NODE via GraphStorage::upsert_node
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/045.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/045.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-upsert-node-045
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-UPSERT-NODES-BATCH-046",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-UPSERT-NODES-BATCH-046
 * @engine      apache_age 1.8
 * @intent      UPSERT-NODES-BATCH via GraphStorage::upsert_nodes_batch
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/046.md
 * @limits      type=W; transactional=Y; native ON CONFLICT
 * @scaling     see specs/088-data-layer/benchmarks/046.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-upsert-nodes-batch-046
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-DELETE-NODE-047",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-DELETE-NODE-047
 * @engine      apache_age 1.8
 * @intent      DELETE-NODE via GraphStorage::delete_node
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/047.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/047.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-delete-node-047
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-DELETE-NODES-BATCH-048",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-DELETE-NODES-BATCH-048
 * @engine      apache_age 1.8
 * @intent      DELETE-NODES-BATCH via GraphStorage::delete_nodes_batch
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/048.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/048.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-delete-nodes-batch-048
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-DELETE-NODE-SCOPED-049",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-DELETE-NODE-SCOPED-049
 * @engine      apache_age 1.8
 * @intent      DELETE-NODE-SCOPED via GraphStorage::delete_node_scoped
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/049.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/049.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-delete-node-scoped-049
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-UPSERT-EDGE-050",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-UPSERT-EDGE-050
 * @engine      apache_age 1.8
 * @intent      UPSERT-EDGE via GraphStorage::upsert_edge
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/050.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/050.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-upsert-edge-050
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-UPSERT-EDGES-BATCH-051",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-UPSERT-EDGES-BATCH-051
 * @engine      apache_age 1.8
 * @intent      UPSERT-EDGES-BATCH via GraphStorage::upsert_edges_batch
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/051.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/051.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-upsert-edges-batch-051
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-DELETE-EDGE-052",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-DELETE-EDGE-052
 * @engine      apache_age 1.8
 * @intent      DELETE-EDGE via GraphStorage::delete_edge
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/052.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/052.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-delete-edge-052
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-DELETE-EDGES-BATCH-053",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-DELETE-EDGES-BATCH-053
 * @engine      apache_age 1.8
 * @intent      DELETE-EDGES-BATCH via GraphStorage::delete_edges_batch
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/053.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/053.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-delete-edges-batch-053
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-DELETE-EDGE-SCOPED-054",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-DELETE-EDGE-SCOPED-054
 * @engine      apache_age 1.8
 * @intent      DELETE-EDGE-SCOPED via GraphStorage::delete_edge_scoped
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/054.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/054.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-delete-edge-scoped-054
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-CLEAR-055",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-CLEAR-055
 * @engine      apache_age 1.8
 * @intent      CLEAR via GraphStorage::clear
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/055.md
 * @limits      type=W; transactional=Y; ADMIN
 * @scaling     see specs/088-data-layer/benchmarks/055.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-clear-055
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-CLEAR-WORKSPACE-056",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-CLEAR-WORKSPACE-056
 * @engine      apache_age 1.8
 * @intent      CLEAR-WORKSPACE via GraphStorage::clear_workspace
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/056.md
 * @limits      type=W; transactional=Y; ADMIN
 * @scaling     see specs/088-data-layer/benchmarks/056.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-clear-workspace-056
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-NODE-COUNT-057",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-NODE-COUNT-057
 * @engine      apache_age 1.8
 * @intent      NODE-COUNT via GraphStorage::node_count
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/057.md
 * @limits      type=R; transactional=Y; O(N) exact
 * @scaling     see specs/088-data-layer/benchmarks/057.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-node-count-057
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-EDGE-COUNT-058",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-EDGE-COUNT-058
 * @engine      apache_age 1.8
 * @intent      EDGE-COUNT via GraphStorage::edge_count
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/058.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/058.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-edge-count-058
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-NODE-COUNT-FAST-059",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-NODE-COUNT-FAST-059
 * @engine      apache_age 1.8
 * @intent      NODE-COUNT-FAST via GraphStorage::node_count_fast
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/059.md
 * @limits      type=R; transactional=Y; reltuples
 * @scaling     see specs/088-data-layer/benchmarks/059.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-node-count-fast-059
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-EDGE-COUNT-FAST-060",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-EDGE-COUNT-FAST-060
 * @engine      apache_age 1.8
 * @intent      EDGE-COUNT-FAST via GraphStorage::edge_count_fast
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/060.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/060.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-edge-count-fast-060
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-NODE-COUNT-BY-WORKSPACE-061",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-NODE-COUNT-BY-WORKSPACE-061
 * @engine      apache_age 1.8
 * @intent      NODE-COUNT-BY-WORKSPACE via GraphStorage::node_count_by_workspace
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/061.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/061.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-node-count-by-workspace-061
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-EDGE-COUNT-BY-WORKSPACE-062",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-EDGE-COUNT-BY-WORKSPACE-062
 * @engine      apache_age 1.8
 * @intent      EDGE-COUNT-BY-WORKSPACE via GraphStorage::edge_count_by_workspace
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/062.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/062.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-edge-count-by-workspace-062
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-DISTINCT-NODE-TYPE-COUNT-063",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-DISTINCT-NODE-TYPE-COUNT-063
 * @engine      apache_age 1.8
 * @intent      DISTINCT-NODE-TYPE-COUNT via GraphStorage::distinct_node_type_count_by_workspace
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/063.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/063.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-distinct-node-type-count-063
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-NODE-COUNT-BY-SOURCE-PREFIX-064",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-NODE-COUNT-BY-SOURCE-PREFIX-064
 * @engine      apache_age 1.8
 * @intent      NODE-COUNT-BY-SOURCE-PREFIX via GraphStorage::node_count_by_source_prefix
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/064.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/064.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-node-count-by-source-prefix-064
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-NODE-COUNTS-BY-SOURCE-PREFIXES-065",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-NODE-COUNTS-BY-SOURCE-PREFIXES-065
 * @engine      apache_age 1.8
 * @intent      NODE-COUNTS-BY-SOURCE-PREFIXES via GraphStorage::node_counts_by_source_prefixes
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/065.md
 * @limits      type=R; transactional=Y; batched list reconcile
 * @scaling     see specs/088-data-layer/benchmarks/065.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-node-counts-by-source-prefixes-065
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-LIST-NODES-FILTERED-066",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-LIST-NODES-FILTERED-066
 * @engine      apache_age 1.8
 * @intent      LIST-NODES-FILTERED via GraphStorage::list_nodes_filtered
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/066.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/066.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-list-nodes-filtered-066
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-LIST-EDGES-FILTERED-067",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-LIST-EDGES-FILTERED-067
 * @engine      apache_age 1.8
 * @intent      LIST-EDGES-FILTERED via GraphStorage::list_edges_filtered
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/067.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/067.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-list-edges-filtered-067
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-FIND-NODES-BY-SOURCE-PREFIXES-068",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-FIND-NODES-BY-SOURCE-PREFIXES-068
 * @engine      apache_age 1.8
 * @intent      FIND-NODES-BY-SOURCE-PREFIXES via GraphStorage::find_nodes_by_source_prefixes
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/068.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/068.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-find-nodes-by-source-prefixes-068
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-FIND-EDGES-BY-SOURCE-PREFIXES-069",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-FIND-EDGES-BY-SOURCE-PREFIXES-069
 * @engine      apache_age 1.8
 * @intent      FIND-EDGES-BY-SOURCE-PREFIXES via GraphStorage::find_edges_by_source_prefixes
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/069.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/069.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-find-edges-by-source-prefixes-069
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-FIND-EDGE-BY-RELATIONSHIP-ID-070",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-FIND-EDGE-BY-RELATIONSHIP-ID-070
 * @engine      apache_age 1.8
 * @intent      FIND-EDGE-BY-RELATIONSHIP-ID via GraphStorage::find_edge_by_relationship_id
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/070.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/070.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-find-edge-by-relationship-id-070
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-CYPHER-EXEC-071",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-CYPHER-EXEC-071
 * @engine      apache_age 1.8
 * @intent      CYPHER-EXEC via execute_cypher / cypher_query
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/071.md
 * @limits      type=R/W; transactional=Y; AGE session wrapper
 * @scaling     see specs/088-data-layer/benchmarks/071.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-cypher-exec-071
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-LIFECYCLE-ENSURE-INDEXES-072",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-LIFECYCLE-ENSURE-INDEXES-072
 * @engine      apache_age 1.8
 * @intent      LIFECYCLE-ENSURE-INDEXES via ensure_indexes
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/072.md
 * @limits      type=DDL; transactional=N; boot-time index reconcile
 * @scaling     see specs/088-data-layer/benchmarks/072.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-lifecycle-ensure-indexes-072
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-COPY-LOAD-VERTICES-073",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-COPY-LOAD-VERTICES-073
 * @engine      apache_age 1.8
 * @intent      COPY-LOAD-VERTICES via load_vertices_from_csv
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/073.md
 * @limits      type=W; transactional=Y; COPY bulk
 * @scaling     see specs/088-data-layer/benchmarks/073.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-copy-load-vertices-073
 */"###,
    ),
    (
        "DATA-AGE-GRAPH-SESSION-LOAD-AGE-074",
        r###"/**
 * @dataop      DATA-AGE-GRAPH-SESSION-LOAD-AGE-074
 * @engine      apache_age 1.8
 * @intent      SESSION-LOAD-AGE via set_age_session / search_path
 * @tables      domain GRAPH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/074.md
 * @limits      type=SESSION; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/074.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-graph-session-load-age-074
 */"###,
    ),
    (
        "DATA-PG-KV-GET-BY-ID-075",
        r###"/**
 * @dataop      DATA-PG-KV-GET-BY-ID-075
 * @engine      postgres
 * @intent      GET-BY-ID via KVStorage::get_by_id
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/075.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/075.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-get-by-id-075
 */"###,
    ),
    (
        "DATA-PG-KV-GET-BY-IDS-076",
        r###"/**
 * @dataop      DATA-PG-KV-GET-BY-IDS-076
 * @engine      postgres
 * @intent      GET-BY-IDS via KVStorage::get_by_ids
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/076.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/076.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-get-by-ids-076
 */"###,
    ),
    (
        "DATA-PG-KV-GET-BY-IDS-ORDERED-077",
        r###"/**
 * @dataop      DATA-PG-KV-GET-BY-IDS-ORDERED-077
 * @engine      postgres
 * @intent      GET-BY-IDS-ORDERED via KVStorage::get_by_ids_ordered
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/077.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/077.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-get-by-ids-ordered-077
 */"###,
    ),
    (
        "DATA-PG-KV-FILTER-KEYS-078",
        r###"/**
 * @dataop      DATA-PG-KV-FILTER-KEYS-078
 * @engine      postgres
 * @intent      FILTER-KEYS via KVStorage::filter_keys
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/078.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/078.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-filter-keys-078
 */"###,
    ),
    (
        "DATA-PG-KV-UPSERT-079",
        r###"/**
 * @dataop      DATA-PG-KV-UPSERT-079
 * @engine      postgres
 * @intent      UPSERT via KVStorage::upsert
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/079.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/079.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-upsert-079
 */"###,
    ),
    (
        "DATA-PG-KV-DELETE-080",
        r###"/**
 * @dataop      DATA-PG-KV-DELETE-080
 * @engine      postgres
 * @intent      DELETE via KVStorage::delete
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/080.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/080.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-delete-080
 */"###,
    ),
    (
        "DATA-PG-KV-COUNT-081",
        r###"/**
 * @dataop      DATA-PG-KV-COUNT-081
 * @engine      postgres
 * @intent      COUNT via KVStorage::count
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/081.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/081.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-count-081
 */"###,
    ),
    (
        "DATA-PG-KV-IS-EMPTY-082",
        r###"/**
 * @dataop      DATA-PG-KV-IS-EMPTY-082
 * @engine      postgres
 * @intent      IS-EMPTY via KVStorage::is_empty
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/082.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/082.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-is-empty-082
 */"###,
    ),
    (
        "DATA-PG-KV-PING-083",
        r###"/**
 * @dataop      DATA-PG-KV-PING-083
 * @engine      postgres
 * @intent      PING via KVStorage::ping
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/083.md
 * @limits      type=R; transactional=N; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/083.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-ping-083
 */"###,
    ),
    (
        "DATA-PG-KV-COUNT-EMBEDDED-CHUNKS-084",
        r###"/**
 * @dataop      DATA-PG-KV-COUNT-EMBEDDED-CHUNKS-084
 * @engine      postgres
 * @intent      COUNT-EMBEDDED-CHUNKS via KVStorage::count_embedded_chunks_for_docs
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/084.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/084.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-count-embedded-chunks-084
 */"###,
    ),
    (
        "DATA-PG-KV-KEYS-WITH-PREFIX-085",
        r###"/**
 * @dataop      DATA-PG-KV-KEYS-WITH-PREFIX-085
 * @engine      postgres
 * @intent      KEYS-WITH-PREFIX via KVStorage::keys_with_prefix
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/085.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/085.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-keys-with-prefix-085
 */"###,
    ),
    (
        "DATA-PG-KV-KEYS-WITH-PREFIX-LIMITED-086",
        r###"/**
 * @dataop      DATA-PG-KV-KEYS-WITH-PREFIX-LIMITED-086
 * @engine      postgres
 * @intent      KEYS-WITH-PREFIX-LIMITED via KVStorage::keys_with_prefix_limited
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/086.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/086.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-keys-with-prefix-limited-086
 */"###,
    ),
    (
        "DATA-PG-KV-KEYS-WITH-SUFFIX-087",
        r###"/**
 * @dataop      DATA-PG-KV-KEYS-WITH-SUFFIX-087
 * @engine      postgres
 * @intent      KEYS-WITH-SUFFIX via KVStorage::keys_with_suffix
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/087.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/087.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-keys-with-suffix-087
 */"###,
    ),
    (
        "DATA-PG-KV-KEYS-WITH-SUFFIX-LIMITED-088",
        r###"/**
 * @dataop      DATA-PG-KV-KEYS-WITH-SUFFIX-LIMITED-088
 * @engine      postgres
 * @intent      KEYS-WITH-SUFFIX-LIMITED via KVStorage::keys_with_suffix_limited
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/088.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/088.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-keys-with-suffix-limited-088
 */"###,
    ),
    (
        "DATA-PG-KV-KEYS-089",
        r###"/**
 * @dataop      DATA-PG-KV-KEYS-089
 * @engine      postgres
 * @intent      KEYS via KVStorage::keys
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/089.md
 * @limits      type=R; transactional=Y; ADMIN mid-wildcard
 * @scaling     see specs/088-data-layer/benchmarks/089.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-keys-089
 */"###,
    ),
    (
        "DATA-PG-KV-CLEAR-090",
        r###"/**
 * @dataop      DATA-PG-KV-CLEAR-090
 * @engine      postgres
 * @intent      CLEAR via KVStorage::clear
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/090.md
 * @limits      type=W; transactional=Y; ADMIN
 * @scaling     see specs/088-data-layer/benchmarks/090.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-clear-090
 */"###,
    ),
    (
        "DATA-PG-KV-TRANSITION-IF-STATUS-091",
        r###"/**
 * @dataop      DATA-PG-KV-TRANSITION-IF-STATUS-091
 * @engine      postgres
 * @intent      TRANSITION-IF-STATUS via KVStorage::transition_if_status
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/091.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/091.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-transition-if-status-091
 */"###,
    ),
    (
        "DATA-PG-KV-DDL-CREATE-TABLE-092",
        r###"/**
 * @dataop      DATA-PG-KV-DDL-CREATE-TABLE-092
 * @engine      postgres
 * @intent      DDL-CREATE-TABLE via PostgresKVStorage::create_table
 * @tables      domain KV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/092.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/092.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-kv-ddl-create-table-092
 */"###,
    ),
    (
        "DATA-PG-PDF-STORE-093",
        r###"/**
 * @dataop      DATA-PG-PDF-STORE-093
 * @engine      postgres
 * @intent      STORE via PdfStorage::store_pdf
 * @tables      domain PDF — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/093.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/093.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-pdf-store-093
 */"###,
    ),
    (
        "DATA-PG-PDF-GET-094",
        r###"/**
 * @dataop      DATA-PG-PDF-GET-094
 * @engine      postgres
 * @intent      GET via PdfStorage::get_pdf
 * @tables      domain PDF — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/094.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/094.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-pdf-get-094
 */"###,
    ),
    (
        "DATA-PG-PDF-UPDATE-MARKDOWN-095",
        r###"/**
 * @dataop      DATA-PG-PDF-UPDATE-MARKDOWN-095
 * @engine      postgres
 * @intent      UPDATE-MARKDOWN via PdfStorage::update_markdown
 * @tables      domain PDF — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/095.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/095.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-pdf-update-markdown-095
 */"###,
    ),
    (
        "DATA-PG-PDF-UPDATE-STATUS-096",
        r###"/**
 * @dataop      DATA-PG-PDF-UPDATE-STATUS-096
 * @engine      postgres
 * @intent      UPDATE-STATUS via PdfStorage::update_pdf_processing
 * @tables      domain PDF — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/096.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/096.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-pdf-update-status-096
 */"###,
    ),
    (
        "DATA-PG-PDF-LINK-TO-DOCUMENT-097",
        r###"/**
 * @dataop      DATA-PG-PDF-LINK-TO-DOCUMENT-097
 * @engine      postgres
 * @intent      LINK-TO-DOCUMENT via PdfStorage::link_pdf_to_document
 * @tables      domain PDF — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/097.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/097.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-pdf-link-to-document-097
 */"###,
    ),
    (
        "DATA-PG-PDF-LIST-098",
        r###"/**
 * @dataop      DATA-PG-PDF-LIST-098
 * @engine      postgres
 * @intent      LIST via PdfStorage::list_pdfs
 * @tables      domain PDF — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/098.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/098.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-pdf-list-098
 */"###,
    ),
    (
        "DATA-PG-PDF-DELETE-099",
        r###"/**
 * @dataop      DATA-PG-PDF-DELETE-099
 * @engine      postgres
 * @intent      DELETE via PdfStorage::delete_pdf
 * @tables      domain PDF — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/099.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/099.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-pdf-delete-099
 */"###,
    ),
    (
        "DATA-PG-PDF-CLEAR-MARKDOWN-100",
        r###"/**
 * @dataop      DATA-PG-PDF-CLEAR-MARKDOWN-100
 * @engine      postgres
 * @intent      CLEAR-MARKDOWN via PdfStorage::clear_markdown
 * @tables      domain PDF — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/100.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/100.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-pdf-clear-markdown-100
 */"###,
    ),
    (
        "DATA-PG-DOCS-ENSURE-RECORD-101",
        r###"/**
 * @dataop      DATA-PG-DOCS-ENSURE-RECORD-101
 * @engine      postgres
 * @intent      ENSURE-RECORD via ensure_document_record
 * @tables      domain DOCS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/101.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/101.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-docs-ensure-record-101
 */"###,
    ),
    (
        "DATA-PG-DOCS-UPDATE-STATS-102",
        r###"/**
 * @dataop      DATA-PG-DOCS-UPDATE-STATS-102
 * @engine      postgres
 * @intent      UPDATE-STATS via update_document_stats
 * @tables      domain DOCS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/102.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/102.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-docs-update-stats-102
 */"###,
    ),
    (
        "DATA-PG-DOCS-TOUCH-STATUS-103",
        r###"/**
 * @dataop      DATA-PG-DOCS-TOUCH-STATUS-103
 * @engine      postgres
 * @intent      TOUCH-STATUS via touch_document_status
 * @tables      domain DOCS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/103.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/103.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-docs-touch-status-103
 */"###,
    ),
    (
        "DATA-PG-DOCS-DELETE-RECORD-104",
        r###"/**
 * @dataop      DATA-PG-DOCS-DELETE-RECORD-104
 * @engine      postgres
 * @intent      DELETE-RECORD via delete_document_record
 * @tables      domain DOCS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/104.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/104.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-docs-delete-record-104
 */"###,
    ),
    (
        "DATA-PG-PDF-COUNT-105",
        r###"/**
 * @dataop      DATA-PG-PDF-COUNT-105
 * @engine      postgres
 * @intent      COUNT via count_pdfs
 * @tables      domain PDF — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/105.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/105.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-pdf-count-105
 */"###,
    ),
    (
        "DATA-PG-DOCS-LIST-SUMMARIES-106",
        r###"/**
 * @dataop      DATA-PG-DOCS-LIST-SUMMARIES-106
 * @engine      postgres
 * @intent      LIST-SUMMARIES via list_relational_document_summaries
 * @tables      domain DOCS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/106.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/106.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-docs-list-summaries-106
 */"###,
    ),
    (
        "DATA-PG-DOCS-DELETE-WORKSPACE-107",
        r###"/**
 * @dataop      DATA-PG-DOCS-DELETE-WORKSPACE-107
 * @engine      postgres
 * @intent      DELETE-WORKSPACE via delete_relational_documents_for_workspace
 * @tables      domain DOCS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/107.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/107.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-docs-delete-workspace-107
 */"###,
    ),
    (
        "DATA-PG-ORIGINAL-STORE-108",
        r###"/**
 * @dataop      DATA-PG-ORIGINAL-STORE-108
 * @engine      postgres
 * @intent      STORE via OriginalStorage store/get/delete
 * @tables      domain ORIGINAL — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/108.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/108.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-original-store-108
 */"###,
    ),
    (
        "DATA-PG-MM-ASSET-STORE-109",
        r###"/**
 * @dataop      DATA-PG-MM-ASSET-STORE-109
 * @engine      postgres
 * @intent      STORE via MmAssetStorage CRUD
 * @tables      domain MM-ASSET — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/109.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/109.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-mm-asset-store-109
 */"###,
    ),
    (
        "DATA-PG-CONV-CREATE-110",
        r###"/**
 * @dataop      DATA-PG-CONV-CREATE-110
 * @engine      postgres
 * @intent      CREATE via ConversationStorage::create_conversation
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/110.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/110.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-create-110
 */"###,
    ),
    (
        "DATA-PG-CONV-GET-111",
        r###"/**
 * @dataop      DATA-PG-CONV-GET-111
 * @engine      postgres
 * @intent      GET via ConversationStorage::get_conversation
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/111.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/111.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-get-111
 */"###,
    ),
    (
        "DATA-PG-CONV-UPDATE-112",
        r###"/**
 * @dataop      DATA-PG-CONV-UPDATE-112
 * @engine      postgres
 * @intent      UPDATE via ConversationStorage::update_conversation
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/112.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/112.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-update-112
 */"###,
    ),
    (
        "DATA-PG-CONV-DELETE-113",
        r###"/**
 * @dataop      DATA-PG-CONV-DELETE-113
 * @engine      postgres
 * @intent      DELETE via ConversationStorage::delete_conversation
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/113.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/113.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-delete-113
 */"###,
    ),
    (
        "DATA-PG-CONV-LIST-114",
        r###"/**
 * @dataop      DATA-PG-CONV-LIST-114
 * @engine      postgres
 * @intent      LIST via ConversationStorage::list_conversations
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/114.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/114.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-list-114
 */"###,
    ),
    (
        "DATA-PG-CONV-SHARE-115",
        r###"/**
 * @dataop      DATA-PG-CONV-SHARE-115
 * @engine      postgres
 * @intent      SHARE via ConversationStorage::share_conversation
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/115.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/115.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-share-115
 */"###,
    ),
    (
        "DATA-PG-CONV-UNSHARE-116",
        r###"/**
 * @dataop      DATA-PG-CONV-UNSHARE-116
 * @engine      postgres
 * @intent      UNSHARE via ConversationStorage::unshare_conversation
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/116.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/116.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-unshare-116
 */"###,
    ),
    (
        "DATA-PG-CONV-GET-SHARED-117",
        r###"/**
 * @dataop      DATA-PG-CONV-GET-SHARED-117
 * @engine      postgres
 * @intent      GET-SHARED via ConversationStorage::get_shared_conversation
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/117.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/117.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-get-shared-117
 */"###,
    ),
    (
        "DATA-PG-CONV-MSG-CREATE-118",
        r###"/**
 * @dataop      DATA-PG-CONV-MSG-CREATE-118
 * @engine      postgres
 * @intent      MSG-CREATE via ConversationStorage::create_message
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/118.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/118.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-msg-create-118
 */"###,
    ),
    (
        "DATA-PG-CONV-MSG-UPDATE-119",
        r###"/**
 * @dataop      DATA-PG-CONV-MSG-UPDATE-119
 * @engine      postgres
 * @intent      MSG-UPDATE via ConversationStorage::update_message
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/119.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/119.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-msg-update-119
 */"###,
    ),
    (
        "DATA-PG-CONV-MSG-GET-120",
        r###"/**
 * @dataop      DATA-PG-CONV-MSG-GET-120
 * @engine      postgres
 * @intent      MSG-GET via ConversationStorage::get_message
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/120.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/120.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-msg-get-120
 */"###,
    ),
    (
        "DATA-PG-CONV-MSG-DELETE-121",
        r###"/**
 * @dataop      DATA-PG-CONV-MSG-DELETE-121
 * @engine      postgres
 * @intent      MSG-DELETE via ConversationStorage::delete_message
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/121.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/121.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-msg-delete-121
 */"###,
    ),
    (
        "DATA-PG-CONV-MSG-LIST-122",
        r###"/**
 * @dataop      DATA-PG-CONV-MSG-LIST-122
 * @engine      postgres
 * @intent      MSG-LIST via ConversationStorage::list_messages
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/122.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/122.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-msg-list-122
 */"###,
    ),
    (
        "DATA-PG-CONV-FOLDER-CREATE-123",
        r###"/**
 * @dataop      DATA-PG-CONV-FOLDER-CREATE-123
 * @engine      postgres
 * @intent      FOLDER-CREATE via ConversationStorage::create_folder
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/123.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/123.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-folder-create-123
 */"###,
    ),
    (
        "DATA-PG-CONV-FOLDER-LIST-124",
        r###"/**
 * @dataop      DATA-PG-CONV-FOLDER-LIST-124
 * @engine      postgres
 * @intent      FOLDER-LIST via ConversationStorage::list_folders
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/124.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/124.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-folder-list-124
 */"###,
    ),
    (
        "DATA-PG-CONV-FOLDER-UPDATE-125",
        r###"/**
 * @dataop      DATA-PG-CONV-FOLDER-UPDATE-125
 * @engine      postgres
 * @intent      FOLDER-UPDATE via ConversationStorage::update_folder
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/125.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/125.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-folder-update-125
 */"###,
    ),
    (
        "DATA-PG-CONV-FOLDER-GET-126",
        r###"/**
 * @dataop      DATA-PG-CONV-FOLDER-GET-126
 * @engine      postgres
 * @intent      FOLDER-GET via ConversationStorage::get_folder
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/126.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/126.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-folder-get-126
 */"###,
    ),
    (
        "DATA-PG-CONV-FOLDER-DELETE-127",
        r###"/**
 * @dataop      DATA-PG-CONV-FOLDER-DELETE-127
 * @engine      postgres
 * @intent      FOLDER-DELETE via ConversationStorage::delete_folder
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/127.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/127.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-folder-delete-127
 */"###,
    ),
    (
        "DATA-PG-CONV-BULK-DELETE-128",
        r###"/**
 * @dataop      DATA-PG-CONV-BULK-DELETE-128
 * @engine      postgres
 * @intent      BULK-DELETE via ConversationStorage::bulk_delete
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/128.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/128.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-bulk-delete-128
 */"###,
    ),
    (
        "DATA-PG-CONV-BULK-ARCHIVE-129",
        r###"/**
 * @dataop      DATA-PG-CONV-BULK-ARCHIVE-129
 * @engine      postgres
 * @intent      BULK-ARCHIVE via ConversationStorage::bulk_archive
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/129.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/129.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-bulk-archive-129
 */"###,
    ),
    (
        "DATA-PG-CONV-BULK-MOVE-130",
        r###"/**
 * @dataop      DATA-PG-CONV-BULK-MOVE-130
 * @engine      postgres
 * @intent      BULK-MOVE via ConversationStorage::bulk_move_to_folder
 * @tables      domain CONV — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/130.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/130.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-conv-bulk-move-130
 */"###,
    ),
    (
        "DATA-PG-TASKS-CREATE-131",
        r###"/**
 * @dataop      DATA-PG-TASKS-CREATE-131
 * @engine      postgres
 * @intent      CREATE via PostgresTaskStorage::create_task
 * @tables      domain TASKS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/131.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/131.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tasks-create-131
 */"###,
    ),
    (
        "DATA-PG-TASKS-GET-132",
        r###"/**
 * @dataop      DATA-PG-TASKS-GET-132
 * @engine      postgres
 * @intent      GET via PostgresTaskStorage::get_task
 * @tables      domain TASKS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/132.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/132.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tasks-get-132
 */"###,
    ),
    (
        "DATA-PG-TASKS-TOUCH-133",
        r###"/**
 * @dataop      DATA-PG-TASKS-TOUCH-133
 * @engine      postgres
 * @intent      TOUCH via PostgresTaskStorage::touch_task
 * @tables      domain TASKS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/133.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/133.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tasks-touch-133
 */"###,
    ),
    (
        "DATA-PG-TASKS-UPDATE-134",
        r###"/**
 * @dataop      DATA-PG-TASKS-UPDATE-134
 * @engine      postgres
 * @intent      UPDATE via PostgresTaskStorage::update_task
 * @tables      domain TASKS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/134.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/134.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tasks-update-134
 */"###,
    ),
    (
        "DATA-PG-TASKS-DELETE-135",
        r###"/**
 * @dataop      DATA-PG-TASKS-DELETE-135
 * @engine      postgres
 * @intent      DELETE via PostgresTaskStorage::delete_task
 * @tables      domain TASKS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/135.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/135.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tasks-delete-135
 */"###,
    ),
    (
        "DATA-PG-TASKS-LIST-136",
        r###"/**
 * @dataop      DATA-PG-TASKS-LIST-136
 * @engine      postgres
 * @intent      LIST via PostgresTaskStorage::list_tasks
 * @tables      domain TASKS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/136.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/136.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tasks-list-136
 */"###,
    ),
    (
        "DATA-PG-TASKS-STATS-137",
        r###"/**
 * @dataop      DATA-PG-TASKS-STATS-137
 * @engine      postgres
 * @intent      STATS via PostgresTaskStorage::get_statistics
 * @tables      domain TASKS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/137.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/137.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tasks-stats-137
 */"###,
    ),
    (
        "DATA-PG-TASKS-FIND-ACTIVE-PDF-138",
        r###"/**
 * @dataop      DATA-PG-TASKS-FIND-ACTIVE-PDF-138
 * @engine      postgres
 * @intent      FIND-ACTIVE-PDF via PostgresTaskStorage::find_active_pdf_processing_task
 * @tables      domain TASKS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/138.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/138.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tasks-find-active-pdf-138
 */"###,
    ),
    (
        "DATA-PG-TASKS-FIND-ACTIVE-INGEST-139",
        r###"/**
 * @dataop      DATA-PG-TASKS-FIND-ACTIVE-INGEST-139
 * @engine      postgres
 * @intent      FIND-ACTIVE-INGEST via PostgresTaskStorage::find_active_pdf_ingest_task
 * @tables      domain TASKS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/139.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/139.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tasks-find-active-ingest-139
 */"###,
    ),
    (
        "DATA-PG-TASKS-CLAIM-NEXT-140",
        r###"/**
 * @dataop      DATA-PG-TASKS-CLAIM-NEXT-140
 * @engine      postgres
 * @intent      CLAIM-NEXT via PostgresTaskStorage::claim_next
 * @tables      domain TASKS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/140.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/140.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tasks-claim-next-140
 */"###,
    ),
    (
        "DATA-PG-TASKS-REFRESH-LEASE-141",
        r###"/**
 * @dataop      DATA-PG-TASKS-REFRESH-LEASE-141
 * @engine      postgres
 * @intent      REFRESH-LEASE via PostgresTaskStorage::refresh_lease
 * @tables      domain TASKS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/141.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/141.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tasks-refresh-lease-141
 */"###,
    ),
    (
        "DATA-PG-TASKS-RELEASE-CLAIM-142",
        r###"/**
 * @dataop      DATA-PG-TASKS-RELEASE-CLAIM-142
 * @engine      postgres
 * @intent      RELEASE-CLAIM via PostgresTaskStorage::release_claim
 * @tables      domain TASKS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/142.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/142.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tasks-release-claim-142
 */"###,
    ),
    (
        "DATA-PG-TASKS-QUEUE-METRICS-143",
        r###"/**
 * @dataop      DATA-PG-TASKS-QUEUE-METRICS-143
 * @engine      postgres
 * @intent      QUEUE-METRICS via PostgresTaskStorage::get_queue_metrics_filtered
 * @tables      domain TASKS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/143.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/143.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tasks-queue-metrics-143
 */"###,
    ),
    (
        "DATA-PG-TASKS-TOTAL-COUNT-144",
        r###"/**
 * @dataop      DATA-PG-TASKS-TOTAL-COUNT-144
 * @engine      postgres
 * @intent      TOTAL-COUNT via PostgresTaskStorage::get_total_count
 * @tables      domain TASKS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/144.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/144.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tasks-total-count-144
 */"###,
    ),
    (
        "DATA-PG-TENANT-CREATE-145",
        r###"/**
 * @dataop      DATA-PG-TENANT-CREATE-145
 * @engine      postgres
 * @intent      CREATE via pg_create_tenant
 * @tables      domain TENANT — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/145.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/145.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tenant-create-145
 */"###,
    ),
    (
        "DATA-PG-TENANT-GET-146",
        r###"/**
 * @dataop      DATA-PG-TENANT-GET-146
 * @engine      postgres
 * @intent      GET via pg_get_tenant
 * @tables      domain TENANT — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/146.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/146.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tenant-get-146
 */"###,
    ),
    (
        "DATA-PG-TENANT-GET-BY-SLUG-147",
        r###"/**
 * @dataop      DATA-PG-TENANT-GET-BY-SLUG-147
 * @engine      postgres
 * @intent      GET-BY-SLUG via pg_get_tenant_by_slug
 * @tables      domain TENANT — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/147.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/147.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tenant-get-by-slug-147
 */"###,
    ),
    (
        "DATA-PG-TENANT-UPDATE-148",
        r###"/**
 * @dataop      DATA-PG-TENANT-UPDATE-148
 * @engine      postgres
 * @intent      UPDATE via pg_update_tenant
 * @tables      domain TENANT — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/148.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/148.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tenant-update-148
 */"###,
    ),
    (
        "DATA-PG-TENANT-DELETE-149",
        r###"/**
 * @dataop      DATA-PG-TENANT-DELETE-149
 * @engine      postgres
 * @intent      DELETE via pg_delete_tenant
 * @tables      domain TENANT — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/149.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/149.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tenant-delete-149
 */"###,
    ),
    (
        "DATA-PG-TENANT-LIST-150",
        r###"/**
 * @dataop      DATA-PG-TENANT-LIST-150
 * @engine      postgres
 * @intent      LIST via pg_list_tenants
 * @tables      domain TENANT — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/150.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/150.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-tenant-list-150
 */"###,
    ),
    (
        "DATA-PG-WORKSPACE-CREATE-151",
        r###"/**
 * @dataop      DATA-PG-WORKSPACE-CREATE-151
 * @engine      postgres
 * @intent      CREATE via pg_create_workspace
 * @tables      domain WORKSPACE — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/151.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/151.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-workspace-create-151
 */"###,
    ),
    (
        "DATA-PG-WORKSPACE-GET-152",
        r###"/**
 * @dataop      DATA-PG-WORKSPACE-GET-152
 * @engine      postgres
 * @intent      GET via pg_get_workspace
 * @tables      domain WORKSPACE — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/152.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/152.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-workspace-get-152
 */"###,
    ),
    (
        "DATA-PG-WORKSPACE-GET-BY-SLUG-153",
        r###"/**
 * @dataop      DATA-PG-WORKSPACE-GET-BY-SLUG-153
 * @engine      postgres
 * @intent      GET-BY-SLUG via pg_get_workspace_by_slug
 * @tables      domain WORKSPACE — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/153.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/153.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-workspace-get-by-slug-153
 */"###,
    ),
    (
        "DATA-PG-WORKSPACE-UPDATE-154",
        r###"/**
 * @dataop      DATA-PG-WORKSPACE-UPDATE-154
 * @engine      postgres
 * @intent      UPDATE via pg_update_workspace
 * @tables      domain WORKSPACE — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/154.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/154.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-workspace-update-154
 */"###,
    ),
    (
        "DATA-PG-WORKSPACE-DELETE-155",
        r###"/**
 * @dataop      DATA-PG-WORKSPACE-DELETE-155
 * @engine      postgres
 * @intent      DELETE via pg_delete_workspace
 * @tables      domain WORKSPACE — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/155.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/155.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-workspace-delete-155
 */"###,
    ),
    (
        "DATA-PG-WORKSPACE-LIST-156",
        r###"/**
 * @dataop      DATA-PG-WORKSPACE-LIST-156
 * @engine      postgres
 * @intent      LIST via pg_list_workspaces
 * @tables      domain WORKSPACE — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/156.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/156.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-workspace-list-156
 */"###,
    ),
    (
        "DATA-AGE-WORKSPACE-GET-STATS-157",
        r###"/**
 * @dataop      DATA-AGE-WORKSPACE-GET-STATS-157
 * @engine      apache_age 1.8
 * @intent      GET-STATS via pg_get_workspace_stats
 * @tables      domain WORKSPACE — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/157.md
 * @limits      type=R; transactional=Y; secondary: PG
 * @scaling     see specs/088-data-layer/benchmarks/157.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/age.md#data-age-workspace-get-stats-157
 */"###,
    ),
    (
        "DATA-PG-MEMBERSHIP-ADD-158",
        r###"/**
 * @dataop      DATA-PG-MEMBERSHIP-ADD-158
 * @engine      postgres
 * @intent      ADD via pg_add_membership
 * @tables      domain MEMBERSHIP — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/158.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/158.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-membership-add-158
 */"###,
    ),
    (
        "DATA-PG-MEMBERSHIP-GET-USER-159",
        r###"/**
 * @dataop      DATA-PG-MEMBERSHIP-GET-USER-159
 * @engine      postgres
 * @intent      GET-USER via pg_get_user_memberships
 * @tables      domain MEMBERSHIP — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/159.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/159.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-membership-get-user-159
 */"###,
    ),
    (
        "DATA-PG-MEMBERSHIP-GET-TENANT-160",
        r###"/**
 * @dataop      DATA-PG-MEMBERSHIP-GET-TENANT-160
 * @engine      postgres
 * @intent      GET-TENANT via pg_get_tenant_memberships
 * @tables      domain MEMBERSHIP — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/160.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/160.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-membership-get-tenant-160
 */"###,
    ),
    (
        "DATA-PG-MEMBERSHIP-UPDATE-ROLE-161",
        r###"/**
 * @dataop      DATA-PG-MEMBERSHIP-UPDATE-ROLE-161
 * @engine      postgres
 * @intent      UPDATE-ROLE via pg_update_membership_role
 * @tables      domain MEMBERSHIP — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/161.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/161.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-membership-update-role-161
 */"###,
    ),
    (
        "DATA-PG-MEMBERSHIP-REMOVE-162",
        r###"/**
 * @dataop      DATA-PG-MEMBERSHIP-REMOVE-162
 * @engine      postgres
 * @intent      REMOVE via pg_remove_membership
 * @tables      domain MEMBERSHIP — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/162.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/162.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-membership-remove-162
 */"###,
    ),
    (
        "DATA-PG-MEMBERSHIP-CHECK-TENANT-163",
        r###"/**
 * @dataop      DATA-PG-MEMBERSHIP-CHECK-TENANT-163
 * @engine      postgres
 * @intent      CHECK-TENANT via pg_check_tenant_access
 * @tables      domain MEMBERSHIP — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/163.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/163.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-membership-check-tenant-163
 */"###,
    ),
    (
        "DATA-PG-MEMBERSHIP-CHECK-WORKSPACE-164",
        r###"/**
 * @dataop      DATA-PG-MEMBERSHIP-CHECK-WORKSPACE-164
 * @engine      postgres
 * @intent      CHECK-WORKSPACE via pg_check_workspace_access
 * @tables      domain MEMBERSHIP — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/164.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/164.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-membership-check-workspace-164
 */"###,
    ),
    (
        "DATA-PG-MEMBERSHIP-GET-ROLE-165",
        r###"/**
 * @dataop      DATA-PG-MEMBERSHIP-GET-ROLE-165
 * @engine      postgres
 * @intent      GET-ROLE via pg_get_user_role
 * @tables      domain MEMBERSHIP — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/165.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/165.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-membership-get-role-165
 */"###,
    ),
    (
        "DATA-PG-QUOTA-UPDATE-TENANT-166",
        r###"/**
 * @dataop      DATA-PG-QUOTA-UPDATE-TENANT-166
 * @engine      postgres
 * @intent      UPDATE-TENANT via pg_update_tenant_quota
 * @tables      domain QUOTA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/166.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/166.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-quota-update-tenant-166
 */"###,
    ),
    (
        "DATA-PG-METRICS-RECORD-SNAPSHOT-167",
        r###"/**
 * @dataop      DATA-PG-METRICS-RECORD-SNAPSHOT-167
 * @engine      postgres
 * @intent      RECORD-SNAPSHOT via pg_record_metrics_snapshot
 * @tables      domain METRICS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/167.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/167.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-metrics-record-snapshot-167
 */"###,
    ),
    (
        "DATA-PG-METRICS-GET-HISTORY-168",
        r###"/**
 * @dataop      DATA-PG-METRICS-GET-HISTORY-168
 * @engine      postgres
 * @intent      GET-HISTORY via pg_get_metrics_history
 * @tables      domain METRICS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/168.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/168.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-metrics-get-history-168
 */"###,
    ),
    (
        "DATA-PG-AUTH-SYNC-USER-169",
        r###"/**
 * @dataop      DATA-PG-AUTH-SYNC-USER-169
 * @engine      postgres
 * @intent      SYNC-USER via sync_auth_user_to_postgres
 * @tables      domain AUTH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/169.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/169.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-auth-sync-user-169
 */"###,
    ),
    (
        "DATA-PG-AUTH-ENSURE-DEFAULT-TENANT-WS-170",
        r###"/**
 * @dataop      DATA-PG-AUTH-ENSURE-DEFAULT-TENANT-WS-170
 * @engine      postgres
 * @intent      ENSURE-DEFAULT-TENANT-WS via ensure_default_tenant_workspace
 * @tables      domain AUTH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/170.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/170.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-auth-ensure-default-tenant-ws-170
 */"###,
    ),
    (
        "DATA-PG-AUTH-SYNC-MEMBERSHIP-171",
        r###"/**
 * @dataop      DATA-PG-AUTH-SYNC-MEMBERSHIP-171
 * @engine      postgres
 * @intent      SYNC-MEMBERSHIP via sync_default_membership_to_postgres
 * @tables      domain AUTH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/171.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/171.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-auth-sync-membership-171
 */"###,
    ),
    (
        "DATA-PG-AUTH-VERIFY-MEMBERSHIP-172",
        r###"/**
 * @dataop      DATA-PG-AUTH-VERIFY-MEMBERSHIP-172
 * @engine      postgres
 * @intent      VERIFY-MEMBERSHIP via verify_membership_active
 * @tables      domain AUTH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/172.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/172.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-auth-verify-membership-172
 */"###,
    ),
    (
        "DATA-PG-AUTH-LOAD-USER-173",
        r###"/**
 * @dataop      DATA-PG-AUTH-LOAD-USER-173
 * @engine      postgres
 * @intent      LOAD-USER via load_user_record_pg
 * @tables      domain AUTH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/173.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/173.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-auth-load-user-173
 */"###,
    ),
    (
        "DATA-PG-AUTH-FIND-USER-BY-LOGIN-174",
        r###"/**
 * @dataop      DATA-PG-AUTH-FIND-USER-BY-LOGIN-174
 * @engine      postgres
 * @intent      FIND-USER-BY-LOGIN via find_user_record_by_login_pg
 * @tables      domain AUTH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/174.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/174.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-auth-find-user-by-login-174
 */"###,
    ),
    (
        "DATA-PG-AUTH-LIST-USERS-175",
        r###"/**
 * @dataop      DATA-PG-AUTH-LIST-USERS-175
 * @engine      postgres
 * @intent      LIST-USERS via list_user_records_pg
 * @tables      domain AUTH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/175.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/175.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-auth-list-users-175
 */"###,
    ),
    (
        "DATA-PG-AUTH-DELETE-USER-176",
        r###"/**
 * @dataop      DATA-PG-AUTH-DELETE-USER-176
 * @engine      postgres
 * @intent      DELETE-USER via delete_user_pg
 * @tables      domain AUTH — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/176.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/176.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-auth-delete-user-176
 */"###,
    ),
    (
        "DATA-PG-SESSION-PERSIST-REFRESH-177",
        r###"/**
 * @dataop      DATA-PG-SESSION-PERSIST-REFRESH-177
 * @engine      postgres
 * @intent      PERSIST-REFRESH via persist_refresh_token_pg
 * @tables      domain SESSION — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/177.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/177.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-session-persist-refresh-177
 */"###,
    ),
    (
        "DATA-PG-SESSION-LOAD-REFRESH-178",
        r###"/**
 * @dataop      DATA-PG-SESSION-LOAD-REFRESH-178
 * @engine      postgres
 * @intent      LOAD-REFRESH via load_refresh_token_pg
 * @tables      domain SESSION — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/178.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/178.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-session-load-refresh-178
 */"###,
    ),
    (
        "DATA-PG-SESSION-REVOKE-REFRESH-179",
        r###"/**
 * @dataop      DATA-PG-SESSION-REVOKE-REFRESH-179
 * @engine      postgres
 * @intent      REVOKE-REFRESH via revoke_refresh_token_pg
 * @tables      domain SESSION — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/179.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/179.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-session-revoke-refresh-179
 */"###,
    ),
    (
        "DATA-PG-SESSION-PERSIST-API-KEY-180",
        r###"/**
 * @dataop      DATA-PG-SESSION-PERSIST-API-KEY-180
 * @engine      postgres
 * @intent      PERSIST-API-KEY via persist_api_key_pg
 * @tables      domain SESSION — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/180.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/180.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-session-persist-api-key-180
 */"###,
    ),
    (
        "DATA-PG-SESSION-LIST-API-KEYS-181",
        r###"/**
 * @dataop      DATA-PG-SESSION-LIST-API-KEYS-181
 * @engine      postgres
 * @intent      LIST-API-KEYS via list_api_keys_pg
 * @tables      domain SESSION — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/181.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/181.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-session-list-api-keys-181
 */"###,
    ),
    (
        "DATA-PG-SESSION-FIND-API-KEY-PREFIX-182",
        r###"/**
 * @dataop      DATA-PG-SESSION-FIND-API-KEY-PREFIX-182
 * @engine      postgres
 * @intent      FIND-API-KEY-PREFIX via find_api_keys_by_prefix_pg
 * @tables      domain SESSION — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/182.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/182.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-session-find-api-key-prefix-182
 */"###,
    ),
    (
        "DATA-PG-SESSION-REVOKE-API-KEY-183",
        r###"/**
 * @dataop      DATA-PG-SESSION-REVOKE-API-KEY-183
 * @engine      postgres
 * @intent      REVOKE-API-KEY via revoke_api_key_pg
 * @tables      domain SESSION — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/183.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/183.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-session-revoke-api-key-183
 */"###,
    ),
    (
        "DATA-PG-ENTITY-UPSERT-184",
        r###"/**
 * @dataop      DATA-PG-ENTITY-UPSERT-184
 * @engine      postgres
 * @intent      UPSERT via PostgresEntitySink::upsert_entity
 * @tables      domain ENTITY — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/184.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/184.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-entity-upsert-184
 */"###,
    ),
    (
        "DATA-PG-ENTITY-REMOVE-SOURCES-185",
        r###"/**
 * @dataop      DATA-PG-ENTITY-REMOVE-SOURCES-185
 * @engine      postgres
 * @intent      REMOVE-SOURCES via remove_entity_sources
 * @tables      domain ENTITY — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/185.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/185.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-entity-remove-sources-185
 */"###,
    ),
    (
        "DATA-PG-LINEAGE-RECORD-ENTITY-LINK-186",
        r###"/**
 * @dataop      DATA-PG-LINEAGE-RECORD-ENTITY-LINK-186
 * @engine      postgres
 * @intent      RECORD-ENTITY-LINK via record_entity_link
 * @tables      domain LINEAGE — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/186.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/186.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-lineage-record-entity-link-186
 */"###,
    ),
    (
        "DATA-PG-LINEAGE-RECORD-RELATION-LINK-187",
        r###"/**
 * @dataop      DATA-PG-LINEAGE-RECORD-RELATION-LINK-187
 * @engine      postgres
 * @intent      RECORD-RELATION-LINK via record_relation_link
 * @tables      domain LINEAGE — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/187.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/187.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-lineage-record-relation-link-187
 */"###,
    ),
    (
        "DATA-PG-LINEAGE-RECORD-RELATION-LINKS-BATCH-188",
        r###"/**
 * @dataop      DATA-PG-LINEAGE-RECORD-RELATION-LINKS-BATCH-188
 * @engine      postgres
 * @intent      RECORD-RELATION-LINKS-BATCH via record_relation_links_batch
 * @tables      domain LINEAGE — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/188.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/188.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-lineage-record-relation-links-batch-188
 */"###,
    ),
    (
        "DATA-PG-LINEAGE-RECORD-ENTITY-LINKS-BATCH-189",
        r###"/**
 * @dataop      DATA-PG-LINEAGE-RECORD-ENTITY-LINKS-BATCH-189
 * @engine      postgres
 * @intent      RECORD-ENTITY-LINKS-BATCH via record_entity_links_batch
 * @tables      domain LINEAGE — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/189.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/189.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-lineage-record-entity-links-batch-189
 */"###,
    ),
    (
        "DATA-PG-LINEAGE-APPEND-DESC-HISTORY-190",
        r###"/**
 * @dataop      DATA-PG-LINEAGE-APPEND-DESC-HISTORY-190
 * @engine      postgres
 * @intent      APPEND-DESC-HISTORY via append_description_history
 * @tables      domain LINEAGE — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/190.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/190.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-lineage-append-desc-history-190
 */"###,
    ),
    (
        "DATA-PG-LINEAGE-LOAD-DOC-FROM-CHUNKS-191",
        r###"/**
 * @dataop      DATA-PG-LINEAGE-LOAD-DOC-FROM-CHUNKS-191
 * @engine      postgres
 * @intent      LOAD-DOC-FROM-CHUNKS via load_document_lineage_from_chunk_links
 * @tables      domain LINEAGE — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/191.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/191.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-lineage-load-doc-from-chunks-191
 */"###,
    ),
    (
        "DATA-PG-FAILED-CHUNKS-INSERT-192",
        r###"/**
 * @dataop      DATA-PG-FAILED-CHUNKS-INSERT-192
 * @engine      postgres
 * @intent      INSERT via insert_failed_chunks
 * @tables      domain FAILED-CHUNKS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/192.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/192.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-failed-chunks-insert-192
 */"###,
    ),
    (
        "DATA-PG-FAILED-CHUNKS-LIST-193",
        r###"/**
 * @dataop      DATA-PG-FAILED-CHUNKS-LIST-193
 * @engine      postgres
 * @intent      LIST via list_failed_chunks
 * @tables      domain FAILED-CHUNKS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/193.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/193.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-failed-chunks-list-193
 */"###,
    ),
    (
        "DATA-PG-FAILED-CHUNKS-MARK-STATUS-194",
        r###"/**
 * @dataop      DATA-PG-FAILED-CHUNKS-MARK-STATUS-194
 * @engine      postgres
 * @intent      MARK-STATUS via mark_chunk_status
 * @tables      domain FAILED-CHUNKS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/194.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/194.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-failed-chunks-mark-status-194
 */"###,
    ),
    (
        "DATA-PG-RLS-SET-TENANT-CONTEXT-195",
        r###"/**
 * @dataop      DATA-PG-RLS-SET-TENANT-CONTEXT-195
 * @engine      postgres
 * @intent      SET-TENANT-CONTEXT via set_tenant_context_on_conn
 * @tables      domain RLS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/195.md
 * @limits      type=SESSION; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/195.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-rls-set-tenant-context-195
 */"###,
    ),
    (
        "DATA-PG-RLS-CLEAR-TENANT-CONTEXT-196",
        r###"/**
 * @dataop      DATA-PG-RLS-CLEAR-TENANT-CONTEXT-196
 * @engine      postgres
 * @intent      CLEAR-TENANT-CONTEXT via clear_tenant_context_on_conn
 * @tables      domain RLS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/196.md
 * @limits      type=SESSION; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/196.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-rls-clear-tenant-context-196
 */"###,
    ),
    (
        "DATA-PG-POOL-ACQUIRE-CONNECT-197",
        r###"/**
 * @dataop      DATA-PG-POOL-ACQUIRE-CONNECT-197
 * @engine      postgres
 * @intent      ACQUIRE-CONNECT via PostgresPool connect/acquire
 * @tables      domain POOL — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/197.md
 * @limits      type=SESSION; transactional=N; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/197.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-pool-acquire-connect-197
 */"###,
    ),
    (
        "DATA-PG-AUDIT-WRITE-EVENT-198",
        r###"/**
 * @dataop      DATA-PG-AUDIT-WRITE-EVENT-198
 * @engine      postgres
 * @intent      WRITE-EVENT via write_audit_event
 * @tables      domain AUDIT — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/198.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/198.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-audit-write-event-198
 */"###,
    ),
    (
        "DATA-PG-AUDIT-QUERY-LOGS-199",
        r###"/**
 * @dataop      DATA-PG-AUDIT-QUERY-LOGS-199
 * @engine      postgres
 * @intent      QUERY-LOGS via query_audit_logs
 * @tables      domain AUDIT — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/199.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/199.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-audit-query-logs-199
 */"###,
    ),
    (
        "DATA-PG-CONFIG-LOAD-LLM-DEFAULTS-200",
        r###"/**
 * @dataop      DATA-PG-CONFIG-LOAD-LLM-DEFAULTS-200
 * @engine      postgres
 * @intent      LOAD-LLM-DEFAULTS via load_llm_defaults
 * @tables      domain CONFIG — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/200.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/200.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-config-load-llm-defaults-200
 */"###,
    ),
    (
        "DATA-PG-CONFIG-SAVE-LLM-DEFAULTS-201",
        r###"/**
 * @dataop      DATA-PG-CONFIG-SAVE-LLM-DEFAULTS-201
 * @engine      postgres
 * @intent      SAVE-LLM-DEFAULTS via save_llm_defaults
 * @tables      domain CONFIG — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/201.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/201.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-config-save-llm-defaults-201
 */"###,
    ),
    (
        "DATA-PG-CONFIG-LOAD-PRIORITY-MODE-202",
        r###"/**
 * @dataop      DATA-PG-CONFIG-LOAD-PRIORITY-MODE-202
 * @engine      postgres
 * @intent      LOAD-PRIORITY-MODE via load_priority_mode
 * @tables      domain CONFIG — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/202.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/202.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-config-load-priority-mode-202
 */"###,
    ),
    (
        "DATA-PG-CONFIG-SAVE-PRIORITY-MODE-203",
        r###"/**
 * @dataop      DATA-PG-CONFIG-SAVE-PRIORITY-MODE-203
 * @engine      postgres
 * @intent      SAVE-PRIORITY-MODE via save_priority_mode
 * @tables      domain CONFIG — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/203.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/203.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-config-save-priority-mode-203
 */"###,
    ),
    (
        "DATA-PG-KEYWORDS-CACHE-GET-204",
        r###"/**
 * @dataop      DATA-PG-KEYWORDS-CACHE-GET-204
 * @engine      postgres
 * @intent      CACHE-GET via KeywordCache::get
 * @tables      domain KEYWORDS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/204.md
 * @limits      type=R; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/204.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-keywords-cache-get-204
 */"###,
    ),
    (
        "DATA-PG-KEYWORDS-CACHE-SET-205",
        r###"/**
 * @dataop      DATA-PG-KEYWORDS-CACHE-SET-205
 * @engine      postgres
 * @intent      CACHE-SET via KeywordCache::set
 * @tables      domain KEYWORDS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/205.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/205.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-keywords-cache-set-205
 */"###,
    ),
    (
        "DATA-PG-KEYWORDS-CACHE-DELETE-206",
        r###"/**
 * @dataop      DATA-PG-KEYWORDS-CACHE-DELETE-206
 * @engine      postgres
 * @intent      CACHE-DELETE via KeywordCache::delete
 * @tables      domain KEYWORDS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/206.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/206.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-keywords-cache-delete-206
 */"###,
    ),
    (
        "DATA-PG-KEYWORDS-CACHE-INIT-207",
        r###"/**
 * @dataop      DATA-PG-KEYWORDS-CACHE-INIT-207
 * @engine      postgres
 * @intent      CACHE-INIT via KeywordCache::initialize
 * @tables      domain KEYWORDS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/207.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/207.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-keywords-cache-init-207
 */"###,
    ),
    (
        "DATA-PG-STATS-ENSURE-ROW-COUNT-208",
        r###"/**
 * @dataop      DATA-PG-STATS-ENSURE-ROW-COUNT-208
 * @engine      postgres
 * @intent      ENSURE-ROW-COUNT via ensure_row_count_stats
 * @tables      domain STATS — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/208.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/208.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-stats-ensure-row-count-208
 */"###,
    ),
    (
        "DATA-PG-ID-ALLOCATE-DOCUMENT-209",
        r###"/**
 * @dataop      DATA-PG-ID-ALLOCATE-DOCUMENT-209
 * @engine      postgres
 * @intent      ALLOCATE-DOCUMENT via allocate_document_id
 * @tables      domain ID — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/209.md
 * @limits      type=W; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/209.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-id-allocate-document-209
 */"###,
    ),
    (
        "DATA-PG-INSPECT-CHECK-EXTENSIONS-210",
        r###"/**
 * @dataop      DATA-PG-INSPECT-CHECK-EXTENSIONS-210
 * @engine      postgres
 * @intent      CHECK-EXTENSIONS via check_extensions
 * @tables      domain INSPECT — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/210.md
 * @limits      type=R; transactional=N; ADMIN
 * @scaling     see specs/088-data-layer/benchmarks/210.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-inspect-check-extensions-210
 */"###,
    ),
    (
        "DATA-PG-INSPECT-CHECK-TABLES-211",
        r###"/**
 * @dataop      DATA-PG-INSPECT-CHECK-TABLES-211
 * @engine      postgres
 * @intent      CHECK-TABLES via check_required_tables
 * @tables      domain INSPECT — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/211.md
 * @limits      type=R; transactional=N; ADMIN
 * @scaling     see specs/088-data-layer/benchmarks/211.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-inspect-check-tables-211
 */"###,
    ),
    (
        "DATA-PG-INSPECT-CHECK-INVARIANTS-212",
        r###"/**
 * @dataop      DATA-PG-INSPECT-CHECK-INVARIANTS-212
 * @engine      postgres
 * @intent      CHECK-INVARIANTS via check_inv* family
 * @tables      domain INSPECT — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/212.md
 * @limits      type=R; transactional=N; ADMIN integrity suite
 * @scaling     see specs/088-data-layer/benchmarks/212.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-inspect-check-invariants-212
 */"###,
    ),
    (
        "DATA-PG-INSPECT-APPLY-REPAIR-213",
        r###"/**
 * @dataop      DATA-PG-INSPECT-APPLY-REPAIR-213
 * @engine      postgres
 * @intent      APPLY-REPAIR via apply_repair
 * @tables      domain INSPECT — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/213.md
 * @limits      type=W; transactional=Y; ADMIN
 * @scaling     see specs/088-data-layer/benchmarks/213.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-inspect-apply-repair-213
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIGRATE-RUNNER-214",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIGRATE-RUNNER-214
 * @engine      postgres
 * @intent      MIGRATE-RUNNER via sqlx migrate + reconcile hooks
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/214.md
 * @limits      type=DDL; transactional=Y; 97 checksum-locked migrations
 * @scaling     see specs/088-data-layer/benchmarks/214.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-migrate-runner-214
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-INIT-BASE-215",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-INIT-BASE-215
 * @engine      postgres
 * @intent      MIG-INIT-BASE via migration 001
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/215.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/215.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-init-base-215
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-TASKS-TABLE-216",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-TASKS-TABLE-216
 * @engine      postgres
 * @intent      MIG-TASKS-TABLE via migration 002
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/216.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/216.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-tasks-table-216
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-CONVERSATION-TABLE-217",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-CONVERSATION-TABLE-217
 * @engine      postgres
 * @intent      MIG-CONVERSATION-TABLE via migration 004
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/217.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/217.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-conversation-table-217
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-AUDIT-LOG-218",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-AUDIT-LOG-218
 * @engine      postgres
 * @intent      MIG-AUDIT-LOG via migration 005
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/218.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/218.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-audit-log-218
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-RLS-POLICIES-219",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-RLS-POLICIES-219
 * @engine      postgres
 * @intent      MIG-RLS-POLICIES via migration 009
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/219.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/219.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-rls-policies-219
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-AGE-GRAPH-220",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-AGE-GRAPH-220
 * @engine      postgres
 * @intent      MIG-AGE-GRAPH via migration 013
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/220.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/220.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-age-graph-220
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-FULLTEXT-SEARCH-221",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-FULLTEXT-SEARCH-221
 * @engine      postgres
 * @intent      MIG-FULLTEXT-SEARCH via migration 015
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/221.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/221.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-fulltext-search-221
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-FAILED-CHUNKS-222",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-FAILED-CHUNKS-222
 * @engine      postgres
 * @intent      MIG-FAILED-CHUNKS via migration 021
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/222.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/222.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-failed-chunks-222
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-PDF-DOCUMENTS-223",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-PDF-DOCUMENTS-223
 * @engine      postgres
 * @intent      MIG-PDF-DOCUMENTS via migration 022
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/223.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/223.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-pdf-documents-223
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-VECTOR-BTREE-INDEXES-224",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-VECTOR-BTREE-INDEXES-224
 * @engine      postgres
 * @intent      MIG-VECTOR-BTREE-INDEXES via migration 029
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/224.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/224.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-vector-btree-indexes-224
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-SOURCE-IDS-GIN-225",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-SOURCE-IDS-GIN-225
 * @engine      postgres
 * @intent      MIG-SOURCE-IDS-GIN via migration 038
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/225.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/225.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-source-ids-gin-225
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-CQRS-ENTITIES-226",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-CQRS-ENTITIES-226
 * @engine      postgres
 * @intent      MIG-CQRS-ENTITIES via migration 039
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/226.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/226.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-cqrs-entities-226
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-CHUNK-LINEAGE-227",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-CHUNK-LINEAGE-227
 * @engine      postgres
 * @intent      MIG-CHUNK-LINEAGE via migration 066
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/227.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/227.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-chunk-lineage-227
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-AGE-INDEXES-CONSOLIDATE-228",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-AGE-INDEXES-CONSOLIDATE-228
 * @engine      postgres
 * @intent      MIG-AGE-INDEXES-CONSOLIDATE via migration 070
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/228.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/228.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-age-indexes-consolidate-228
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-HNSW-OPTIMIZE-229",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-HNSW-OPTIMIZE-229
 * @engine      postgres
 * @intent      MIG-HNSW-OPTIMIZE via migration 071
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/229.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/229.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-hnsw-optimize-229
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-HALFVEC-EMBEDDINGS-230",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-HALFVEC-EMBEDDINGS-230
 * @engine      postgres
 * @intent      MIG-HALFVEC-EMBEDDINGS via migration 080
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/230.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/230.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-halfvec-embeddings-230
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-DOCUMENT-ORIGINALS-231",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-DOCUMENT-ORIGINALS-231
 * @engine      postgres
 * @intent      MIG-DOCUMENT-ORIGINALS via migration 082
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/231.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/231.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-document-originals-231
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-MM-ASSETS-232",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-MM-ASSETS-232
 * @engine      postgres
 * @intent      MIG-MM-ASSETS via migration 084
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/232.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/232.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-mm-assets-232
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-TASK-LEASE-233",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-TASK-LEASE-233
 * @engine      postgres
 * @intent      MIG-TASK-LEASE via migration 088
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/233.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/233.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-task-lease-233
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-MERGE-GRAPH-PROPS-234",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-MERGE-GRAPH-PROPS-234
 * @engine      postgres
 * @intent      MIG-MERGE-GRAPH-PROPS via migration 090
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/234.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/234.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-merge-graph-props-234
 */"###,
    ),
    (
        "DATA-PG-SCHEMA-MIG-EQ-ID-DENORM-235",
        r###"/**
 * @dataop      DATA-PG-SCHEMA-MIG-EQ-ID-DENORM-235
 * @engine      postgres
 * @intent      MIG-EQ-ID-DENORM via migration 092
 * @tables      domain SCHEMA — see indexes.md / migrations
 * @indexes     see specs/088-data-layer/indexes.md
 * @complexity  see complexity-matrix.md / benchmarks/235.md
 * @limits      type=DDL; transactional=Y; see engine doc
 * @scaling     see specs/088-data-layer/benchmarks/235.md
 * @tests       cargo test -p edgequake-storage --test data_layer_limits; e2e_spec061*
 * @pgversions  16: ok | 17: ok | 18: ok (matrix CI)
 * @docs        specs/088-data-layer/postgres.md#data-pg-schema-mig-eq-id-denorm-235
 */"###,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataop::all_ref_ids;

    #[test]
    fn every_ref_has_annotation_block() {
        for id in all_ref_ids() {
            assert!(
                annotation_block(id).is_some(),
                "missing annotation for {id}"
            );
            let b = annotation_block(id).unwrap();
            assert!(b.contains("@dataop"), "{id}");
            assert!(b.contains("@complexity"), "{id}");
            assert!(b.contains("@limits"), "{id}");
            assert!(b.contains("@tests"), "{id}");
        }
        assert_eq!(annotation_count(), all_ref_ids().len());
    }
}
