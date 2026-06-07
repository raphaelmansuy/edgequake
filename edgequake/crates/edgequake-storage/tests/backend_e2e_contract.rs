//! Cross-backend E2E contracts (SPEC-017 P2-11).
//!
//! Single assertion modules run against memory (always) and postgres (when configured).
//! Replaces duplicated smoke tests in `e2e_storage_backends::postgres_tests`.

#[path = "support/kv_e2e_contract.rs"]
mod kv_e2e_contract;

#[path = "support/vector_e2e_contract.rs"]
mod vector_e2e_contract;

#[path = "support/graph_e2e_contract.rs"]
mod graph_e2e_contract;

#[path = "support/graph_batch_contract.rs"]
mod graph_batch_contract;

#[path = "support/e2e_fixtures.rs"]
mod e2e_fixtures;

use e2e_fixtures::generate_namespace;
use edgequake_storage::{
    GraphStorage, KVStorage, MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage,
    VectorStorage,
};

use vector_e2e_contract::CONTRACT_DIMENSION as VECTOR_DIM;

async fn memory_kv() -> MemoryKVStorage {
    let s = MemoryKVStorage::new(generate_namespace());
    s.initialize().await.unwrap();
    s
}

async fn memory_vector() -> MemoryVectorStorage {
    let s = MemoryVectorStorage::new(generate_namespace(), VECTOR_DIM);
    s.initialize().await.unwrap();
    s
}

async fn memory_graph() -> MemoryGraphStorage {
    let s = MemoryGraphStorage::new(generate_namespace());
    s.initialize().await.unwrap();
    s
}

macro_rules! memory_backend_contract {
    ($name:ident, $storage:expr, $assert:expr) => {
        #[tokio::test]
        async fn $name() {
            let storage = $storage().await;
            $assert(&storage).await;
        }
    };
}

memory_backend_contract!(
    memory_kv_basic_crud_e2e_contract,
    memory_kv,
    kv_e2e_contract::assert_kv_basic_crud
);
memory_backend_contract!(
    memory_kv_bulk_e2e_contract,
    memory_kv,
    kv_e2e_contract::assert_kv_bulk_operations
);
memory_backend_contract!(
    memory_kv_filter_keys_e2e_contract,
    memory_kv,
    kv_e2e_contract::assert_kv_filter_keys
);
memory_backend_contract!(
    memory_vector_basic_crud_e2e_contract,
    memory_vector,
    vector_e2e_contract::assert_vector_basic_crud
);
memory_backend_contract!(
    memory_vector_query_e2e_contract,
    memory_vector,
    vector_e2e_contract::assert_vector_query
);
memory_backend_contract!(
    memory_vector_bulk_count_e2e_contract,
    memory_vector,
    vector_e2e_contract::assert_vector_bulk_count
);
memory_backend_contract!(
    memory_graph_node_crud_e2e_contract,
    memory_graph,
    graph_e2e_contract::assert_graph_node_crud
);
memory_backend_contract!(
    memory_graph_edge_crud_e2e_contract,
    memory_graph,
    graph_e2e_contract::assert_graph_edge_crud
);
memory_backend_contract!(
    memory_graph_node_edges_e2e_contract,
    memory_graph,
    graph_e2e_contract::assert_graph_node_edges
);

#[tokio::test]
async fn memory_graph_batch_e2e_contract() {
    let storage = memory_graph().await;
    graph_batch_contract::assert_graph_batch_upsert(&storage).await;
}

#[cfg(feature = "postgres")]
#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

#[cfg(feature = "postgres")]
mod postgres_backend_contract {
    use super::{
        graph_batch_contract, graph_e2e_contract, kv_e2e_contract, postgres_test_config,
        vector_e2e_contract,
    };
    use edgequake_storage::{
        GraphStorage, GraphStorageMutateOps, KVStorage, PgVectorStorage, PostgresAGEGraphStorage,
        PostgresKVStorage, VectorStorage,
    };

    macro_rules! require_postgres_kv {
        () => {{
            let config = match postgres_test_config::contract_postgres_config("backend_kv") {
                Some(c) => c,
                None => {
                    eprintln!("Skipping: POSTGRES_PASSWORD not set");
                    return;
                }
            };
            let s = PostgresKVStorage::new(config);
            s.initialize().await.unwrap();
            s
        }};
    }

    macro_rules! require_postgres_vector {
        () => {{
            let config = match postgres_test_config::contract_postgres_config("backend_vector") {
                Some(c) => c,
                None => {
                    eprintln!("Skipping: POSTGRES_PASSWORD not set");
                    return;
                }
            };
            let s =
                PgVectorStorage::with_dimension(config, vector_e2e_contract::CONTRACT_DIMENSION);
            s.initialize().await.unwrap();
            s
        }};
    }

    macro_rules! require_postgres_graph {
        () => {{
            let config = match postgres_test_config::contract_postgres_config("backend_graph") {
                Some(c) => c,
                None => {
                    eprintln!("Skipping: POSTGRES_PASSWORD not set");
                    return;
                }
            };
            let s = PostgresAGEGraphStorage::new(config);
            s.initialize().await.unwrap();
            s
        }};
    }

    macro_rules! postgres_backend_contract {
        ($name:ident, $init:ident, $assert:expr) => {
            #[tokio::test]
            async fn $name() {
                let storage = $init!();
                $assert(&storage).await;
                let _ = storage.clear().await;
            }
        };
    }

    postgres_backend_contract!(
        postgres_kv_basic_crud_e2e_contract,
        require_postgres_kv,
        kv_e2e_contract::assert_kv_basic_crud
    );
    postgres_backend_contract!(
        postgres_kv_bulk_e2e_contract,
        require_postgres_kv,
        kv_e2e_contract::assert_kv_bulk_operations
    );
    postgres_backend_contract!(
        postgres_vector_basic_crud_e2e_contract,
        require_postgres_vector,
        vector_e2e_contract::assert_vector_basic_crud
    );
    postgres_backend_contract!(
        postgres_vector_query_e2e_contract,
        require_postgres_vector,
        vector_e2e_contract::assert_vector_query
    );
    postgres_backend_contract!(
        postgres_graph_node_crud_e2e_contract,
        require_postgres_graph,
        graph_e2e_contract::assert_graph_node_crud
    );
    postgres_backend_contract!(
        postgres_graph_edge_crud_e2e_contract,
        require_postgres_graph,
        graph_e2e_contract::assert_graph_edge_crud
    );

    #[tokio::test]
    async fn postgres_graph_batch_e2e_contract() {
        let storage = require_postgres_graph!();
        graph_batch_contract::assert_graph_batch_upsert(&storage).await;
        let _ = storage.clear().await;
    }
}
