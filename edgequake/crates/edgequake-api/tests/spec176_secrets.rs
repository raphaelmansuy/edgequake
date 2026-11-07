//! SPEC-176 secret manager. Database is the default. Infisical is optional:
//! a dead Infisical host fails that tenant only, and does not write the key
//! into `managed_secrets`.
#![cfg(feature = "postgres")]

#[path = "../../edgequake-storage/tests/support/postgres_access_pool.rs"]
#[allow(dead_code)]
mod postgres_access_pool;

use edgequake_api::handlers::connections::{connection_list_sql, connection_update_sql};
use edgequake_api::providers::secret_manager::SecretService;
use edgequake_secrets::{encrypt, SecretRef};
use sqlx::Row;
use uuid::Uuid;

fn install_key() -> ([u8; 32], String) {
    let key = [9u8; 32];
    let hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
    std::env::set_var("EDGEQUAKE_SECRETS_KEY", &hex);
    std::env::set_var("EDGEQUAKE_SECRETS_KEY_ID", "spec176");
    (key, "spec176".into())
}

#[tokio::test]
async fn database_round_trip_legacy_fallback_and_isolated_infisical_failure() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(std::time::Duration::from_secs(15))
            .await
    else {
        return;
    };
    let (key, key_id) = install_key();
    let service = SecretService::new(pool.clone());
    let database_tenant = Uuid::new_v4();
    let infisical_tenant = Uuid::new_v4();

    let stored = service
        .put(Some(database_tenant), "eq-conn-db", "sk-database")
        .await
        .expect("database put");
    let SecretRef::Database { id } = &stored else {
        panic!("default backend must be the database, got {stored:?}");
    };
    let revealed = edgequake_api::providers::secret_manager::reveal_connection_key(
        &pool,
        Some(database_tenant),
        Some(serde_json::to_value(&stored).unwrap()),
        None,
        None,
        None,
    )
    .await
    .expect("reveal ref");
    assert_eq!(revealed.as_deref(), Some("sk-database"));

    let legacy = encrypt(b"sk-legacy", &key, &key_id).unwrap();
    let legacy_value = edgequake_api::providers::secret_manager::reveal_connection_key(
        &pool,
        Some(database_tenant),
        None,
        Some(legacy.ciphertext),
        Some(legacy.nonce),
        Some(legacy.key_id),
    )
    .await
    .expect("legacy");
    assert_eq!(legacy_value.as_deref(), Some("sk-legacy"));

    let client = encrypt(b"infisical-client-secret", &key, &key_id).unwrap();
    sqlx::query(
        r#"INSERT INTO secret_backend_configs (
               tenant_id, kind, infisical_base_url, infisical_client_id, infisical_project_id,
               infisical_environment, infisical_secret_path, allow_private_network,
               client_secret_ciphertext, client_secret_nonce, client_secret_key_id
           ) VALUES ($1, 'infisical', 'http://127.0.0.1:1', 'client', 'project', 'dev', '/', true, $2, $3, $4)"#,
    )
    .bind(infisical_tenant)
    .bind(&client.ciphertext)
    .bind(&client.nonce)
    .bind(&client.key_id)
    .execute(&pool)
    .await
    .expect("infisical config");

    let failed = service
        .put(
            Some(infisical_tenant),
            "eq-conn-remote",
            "sk-should-stay-out",
        )
        .await
        .expect_err("unreachable infisical");
    let message = failed.to_string();
    assert!(
        message.to_ascii_lowercase().contains("unreachable")
            || message.to_ascii_lowercase().contains("infisical"),
        "{message}"
    );
    assert!(!message.contains("sk-should-stay-out"));
    let leaked: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM managed_secrets WHERE tenant_id = $1 AND name = 'eq-conn-remote'",
    )
    .bind(infisical_tenant)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        leaked, 0,
        "a failed Infisical write must not fall back to the database"
    );

    let still: String =
        sqlx::query_scalar("SELECT name FROM managed_secrets WHERE id = $1 AND tenant_id = $2")
            .bind(Uuid::parse_str(id).unwrap())
            .bind(database_tenant)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(still, "eq-conn-db");

    let slug = format!("spec176-{database_tenant}");
    sqlx::query(
        r#"INSERT INTO provider_connections (
               tenant_id, slug, display_name, api_shape, locality, base_url, secret_ref
           ) VALUES ($1, $2, $2, 'openai_chat', 'cloud', 'https://api.openai.com/v1', $3)"#,
    )
    .bind(database_tenant)
    .bind(&slug)
    .bind(serde_json::to_value(&stored).unwrap())
    .execute(&pool)
    .await
    .expect("connection row");
    let listed = sqlx::query(&connection_list_sql(true))
        .bind(database_tenant)
        .fetch_all(&pool)
        .await
        .expect("connection list sql");
    let row = listed
        .iter()
        .find(|row| row.try_get::<String, _>("slug").ok().as_deref() == Some(slug.as_str()))
        .expect("saved connection");
    assert!(row.try_get::<bool, _>("key_configured").unwrap());
    assert_eq!(
        row.try_get::<Option<String>, _>("secret_backend")
            .unwrap()
            .as_deref(),
        Some("database")
    );
    let ciphertext: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT api_key_ciphertext FROM provider_connections WHERE slug = $1")
            .bind(&slug)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        ciphertext.is_none(),
        "a referenced key is not stored inline"
    );

    let other_tenant = Uuid::new_v4();
    let crossed = edgequake_api::providers::secret_manager::reveal_connection_key(
        &pool,
        Some(other_tenant),
        Some(serde_json::to_value(&stored).unwrap()),
        None,
        None,
        None,
    )
    .await;
    assert!(
        crossed.is_err(),
        "a tenant cannot read another tenant's secret"
    );

    let connection_id: Uuid =
        sqlx::query_scalar("SELECT id FROM provider_connections WHERE slug = $1")
            .bind(&slug)
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query(&connection_update_sql(true))
        .bind(connection_id)
        .bind(&slug)
        .bind(&slug)
        .bind("openai_chat")
        .bind("cloud")
        .bind("https://api.openai.com/v1")
        .bind("bearer")
        .bind(None::<Vec<u8>>)
        .bind(None::<Vec<u8>>)
        .bind(None::<String>)
        .bind(Some("fp"))
        .bind(120_i32)
        .bind(false)
        .bind(database_tenant)
        .bind(true)
        .bind(serde_json::to_value(&stored).unwrap())
        .bind(None::<String>)
        .bind(false)
        .execute(&pool)
        .await
        .expect("update sql");

    service
        .delete_ref(Some(database_tenant), &stored)
        .await
        .expect("delete secret with the connection");
    let removed: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM managed_secrets WHERE id = $1")
        .bind(Uuid::parse_str(id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(removed, 0);

    std::env::remove_var(edgequake_secrets::ENV_SECRETS_KEY);
    let refused = service
        .put(Some(database_tenant), "eq-conn-nokey", "sk-refused")
        .await
        .expect_err("missing key");
    assert!(refused.to_string().contains("EDGEQUAKE_SECRETS_KEY"));

    sqlx::query("DELETE FROM provider_connections WHERE slug = $1")
        .bind(&slug)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM managed_secrets WHERE tenant_id = ANY($1)")
        .bind(vec![database_tenant, infisical_tenant])
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM secret_backend_configs WHERE tenant_id = $1")
        .bind(infisical_tenant)
        .execute(&pool)
        .await
        .ok();
}
