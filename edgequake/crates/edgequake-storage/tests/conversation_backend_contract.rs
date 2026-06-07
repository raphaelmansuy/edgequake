//! Conversation adapter contract tests (SPEC-017 P1).

#[path = "support/conversation_contract.rs"]
mod conversation_contract;

use edgequake_storage::MemoryConversationStorage;

#[tokio::test]
async fn memory_conversation_crud_contract() {
    let storage = MemoryConversationStorage::new();
    conversation_contract::assert_conversation_crud_contract(&storage).await;
}

#[cfg(feature = "postgres")]
#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

#[cfg(feature = "postgres")]
mod postgres_conversation_contract {
    use super::{conversation_contract, postgres_test_config};
    use edgequake_storage::PostgresConversationStorage;
    use uuid::Uuid;

    #[tokio::test]
    async fn postgres_conversation_crud_contract() {
        let Some(config) = postgres_test_config::contract_postgres_config("conv_contract") else {
            eprintln!("Skipping postgres conversation contract: POSTGRES_PASSWORD not set");
            return;
        };

        let pool = postgres_test_config::contract_pg_pool(&config).await;
        let tenant = Uuid::new_v4();
        let user = Uuid::new_v4();
        postgres_test_config::seed_tenant_and_user(&pool, tenant, user)
            .await
            .expect("seed tenant/user");

        let storage = PostgresConversationStorage::new(pool);
        conversation_contract::assert_conversation_crud_contract_with_ids(&storage, tenant, user)
            .await;
    }
}
