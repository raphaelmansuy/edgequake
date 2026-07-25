//! SPEC-088: data-layer Ref ID registry integrity (no DB required).

use edgequake_storage::dataop::{all_ref_ids, is_valid_ref_id, lookup, sql_comment};
use edgequake_storage::dataop::{
    DATA_AGE_GRAPH_GET_NODES_BATCH_031, DATA_PGVEC_VECTORS_ANN_QUERY_001,
    DATA_PGVEC_VECTORS_ANN_QUERY_FILTERED_002, DATA_PG_KV_GET_BY_ID_075,
    DATA_PG_TASKS_CLAIM_NEXT_140,
};

#[test]
fn data_layer_registry_unique_valid() {
    let mut seen = std::collections::HashSet::new();
    for id in all_ref_ids() {
        assert!(is_valid_ref_id(id), "invalid {id}");
        assert!(seen.insert(*id), "duplicate {id}");
    }
    assert!(all_ref_ids().len() >= 200, "inventory unexpectedly small");
}

#[test]
fn data_layer_hot_path_refs_resolvable() {
    for id in [
        DATA_PGVEC_VECTORS_ANN_QUERY_001,
        DATA_PGVEC_VECTORS_ANN_QUERY_FILTERED_002,
        DATA_PG_KV_GET_BY_ID_075,
        DATA_AGE_GRAPH_GET_NODES_BATCH_031,
        DATA_PG_TASKS_CLAIM_NEXT_140,
    ] {
        assert_eq!(lookup(id), Some(id));
        let tagged = sql_comment(id, "SELECT 1");
        assert!(tagged.contains(id));
    }
}
