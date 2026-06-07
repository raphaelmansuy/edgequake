//! Backend contract tests for graph workspace stats and batch upsert (SPEC-017).

#[path = "support/graph_workspace_contract.rs"]
mod graph_workspace_contract;

#[path = "support/graph_batch_contract.rs"]
mod graph_batch_contract;

use edgequake_storage::{GraphStorage, MemoryGraphStorage};

#[tokio::test]
async fn memory_backend_workspace_graph_stats_contract() {
    let storage = MemoryGraphStorage::new("contract-memory");
    storage.initialize().await.expect("initialize graph");
    graph_workspace_contract::assert_workspace_graph_stats(&storage).await;
}

#[tokio::test]
async fn memory_backend_graph_batch_upsert_contract() {
    let storage = MemoryGraphStorage::new("contract-memory-batch");
    storage.initialize().await.expect("initialize graph");
    graph_batch_contract::assert_graph_batch_upsert(&storage).await;
}

#[cfg(feature = "postgres")]
#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

#[cfg(feature = "postgres")]
mod postgres_contract {
    use super::{graph_batch_contract, graph_workspace_contract, postgres_test_config};
    use edgequake_storage::{GraphStorage, PostgresAGEGraphStorage, PostgresConfig};

    fn init_graph(config: PostgresConfig) -> PostgresAGEGraphStorage {
        let storage = PostgresAGEGraphStorage::new(config);
        // Caller must initialize before contract assertions.
        storage
    }

    #[tokio::test]
    async fn postgres_backend_workspace_graph_stats_contract() {
        let Some(config) = postgres_test_config::contract_postgres_config("graph_contract") else {
            eprintln!("Skipping postgres contract: POSTGRES_PASSWORD not set");
            return;
        };

        let storage = init_graph(config);
        storage.initialize().await.expect("initialize graph");
        graph_workspace_contract::assert_workspace_graph_stats(&storage).await;
    }

    #[tokio::test]
    async fn postgres_backend_graph_batch_upsert_contract() {
        let Some(config) = postgres_test_config::contract_postgres_config("graph_batch") else {
            eprintln!("Skipping postgres batch contract: POSTGRES_PASSWORD not set");
            return;
        };

        let storage = init_graph(config);
        storage.initialize().await.expect("initialize graph");
        graph_batch_contract::assert_graph_batch_upsert(&storage).await;
    }
}
