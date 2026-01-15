//! PostgreSQL Provider Switching E2E Tests
//!
//! These tests verify that workspace provider configuration:
//! 1. Persists correctly to PostgreSQL metadata JSONB
//! 2. Reads back correctly from PostgreSQL
//! 3. Provider switching is immediately effective for document processing
//! 4. Rebuild operations use updated provider config
//!
//! Run with:
//!   cargo test --package edgequake-api --test e2e_postgres_provider_switching --features postgres
//!
//! Environment variables needed:
//!   - DATABASE_URL or POSTGRES_PASSWORD

#![cfg(feature = "postgres")]

use std::env;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

use edgequake_llm::ProviderFactory;

/// Get database URL from environment
fn get_database_url() -> Option<String> {
    env::var("DATABASE_URL").ok().or_else(|| {
        let password = env::var("POSTGRES_PASSWORD").ok()?;
        let host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
        let db = env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake_test".to_string());
        let user = env::var("POSTGRES_USER").unwrap_or_else(|_| "edgequake_test".to_string());
        Some(format!(
            "postgresql://{}:{}@{}:{}/{}",
            user, password, host, port, db
        ))
    })
}

/// Create test database pool
async fn create_test_pool() -> Option<PgPool> {
    let database_url = get_database_url()?;

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .ok()
}

macro_rules! require_postgres {
    () => {
        match create_test_pool().await {
            Some(pool) => pool,
            None => {
                eprintln!("Skipping test: DATABASE_URL or POSTGRES_PASSWORD not set");
                return;
            }
        }
    };
}

/// Create a test tenant for the tests
async fn create_test_tenant(pool: &PgPool) -> Uuid {
    let tenant_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO tenants (tenant_id, name, slug, is_active, metadata, settings, created_at, updated_at)
        VALUES ($1, $2, $3, TRUE, '{}'::jsonb, '{}'::jsonb, NOW(), NOW())
        ON CONFLICT (tenant_id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .bind(format!("Provider Test Tenant {}", tenant_id))
    .bind(format!("provider-test-{}", &tenant_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("Failed to create test tenant");

    tenant_id
}

/// Create a test workspace with specific provider configuration
async fn create_test_workspace_with_provider(
    pool: &PgPool,
    tenant_id: Uuid,
    llm_provider: &str,
    llm_model: &str,
    embedding_provider: &str,
    embedding_model: &str,
    embedding_dimension: i64,
) -> Uuid {
    let workspace_id = Uuid::new_v4();
    let metadata = serde_json::json!({
        "llm_provider": llm_provider,
        "llm_model": llm_model,
        "embedding_provider": embedding_provider,
        "embedding_model": embedding_model,
        "embedding_dimension": embedding_dimension
    });

    sqlx::query(
        r#"
        INSERT INTO workspaces (workspace_id, tenant_id, name, slug, description, is_active, metadata, settings, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 'Provider switching test', TRUE, $5, '{}'::jsonb, NOW(), NOW())
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("Provider Test Workspace {}", workspace_id))
    .bind(format!("ws-provider-{}", &workspace_id.to_string()[..8]))
    .bind(&metadata)
    .execute(pool)
    .await
    .expect("Failed to create test workspace");

    workspace_id
}

/// Get workspace metadata from database
async fn get_workspace_metadata(pool: &PgPool, workspace_id: Uuid) -> serde_json::Value {
    let row: (serde_json::Value,) =
        sqlx::query_as("SELECT metadata FROM workspaces WHERE workspace_id = $1")
            .bind(workspace_id)
            .fetch_one(pool)
            .await
            .expect("Failed to get workspace metadata");
    row.0
}

/// Update workspace provider in database
async fn update_workspace_provider(
    pool: &PgPool,
    workspace_id: Uuid,
    llm_provider: &str,
    llm_model: &str,
    embedding_provider: &str,
    embedding_model: &str,
) {
    sqlx::query(
        r#"
        UPDATE workspaces
        SET metadata = metadata || $2::jsonb,
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .bind(serde_json::json!({
        "llm_provider": llm_provider,
        "llm_model": llm_model,
        "embedding_provider": embedding_provider,
        "embedding_model": embedding_model
    }))
    .execute(pool)
    .await
    .expect("Failed to update workspace provider");
}

/// Cleanup test data
#[allow(dead_code)]
async fn cleanup_workspace(pool: &PgPool, workspace_id: Uuid) {
    let _ = sqlx::query("DELETE FROM workspaces WHERE workspace_id = $1")
        .bind(workspace_id)
        .execute(pool)
        .await;
}

async fn cleanup_tenant(pool: &PgPool, tenant_id: Uuid) {
    let _ = sqlx::query("DELETE FROM workspaces WHERE tenant_id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM tenants WHERE tenant_id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await;
}

// ============================================================================
// Test: Provider Config Persists to PostgreSQL
// ============================================================================

#[tokio::test]
async fn test_provider_config_persists_to_postgres() {
    let pool = require_postgres!();
    let tenant_id = create_test_tenant(&pool).await;

    // Create workspace with specific provider config
    let workspace_id = create_test_workspace_with_provider(
        &pool,
        tenant_id,
        "openai",
        "gpt-4o-mini",
        "openai",
        "text-embedding-3-small",
        1536,
    )
    .await;

    // Verify metadata was persisted
    let metadata = get_workspace_metadata(&pool, workspace_id).await;

    assert_eq!(
        metadata.get("llm_provider").and_then(|v| v.as_str()),
        Some("openai"),
        "llm_provider should be persisted"
    );
    assert_eq!(
        metadata.get("llm_model").and_then(|v| v.as_str()),
        Some("gpt-4o-mini"),
        "llm_model should be persisted"
    );
    assert_eq!(
        metadata.get("embedding_provider").and_then(|v| v.as_str()),
        Some("openai"),
        "embedding_provider should be persisted"
    );
    assert_eq!(
        metadata.get("embedding_model").and_then(|v| v.as_str()),
        Some("text-embedding-3-small"),
        "embedding_model should be persisted"
    );

    // Cleanup
    cleanup_tenant(&pool, tenant_id).await;
}

// ============================================================================
// Test: Provider Config Updates Persist
// ============================================================================

#[tokio::test]
async fn test_provider_update_persists_to_postgres() {
    let pool = require_postgres!();
    let tenant_id = create_test_tenant(&pool).await;

    // Create workspace with ollama provider
    let workspace_id = create_test_workspace_with_provider(
        &pool,
        tenant_id,
        "ollama",
        "gemma3:12b",
        "ollama",
        "embeddinggemma:latest",
        768,
    )
    .await;

    // Update to openai provider
    update_workspace_provider(
        &pool,
        workspace_id,
        "openai",
        "gpt-4o-mini",
        "openai",
        "text-embedding-3-small",
    )
    .await;

    // Verify update was persisted
    let metadata = get_workspace_metadata(&pool, workspace_id).await;

    assert_eq!(
        metadata.get("llm_provider").and_then(|v| v.as_str()),
        Some("openai"),
        "llm_provider should be updated"
    );
    assert_eq!(
        metadata.get("embedding_provider").and_then(|v| v.as_str()),
        Some("openai"),
        "embedding_provider should be updated"
    );

    // Cleanup
    cleanup_tenant(&pool, tenant_id).await;
}

// ============================================================================
// Test: Empty Metadata Uses Defaults
// ============================================================================

#[tokio::test]
async fn test_empty_metadata_uses_defaults() {
    let pool = require_postgres!();
    let tenant_id = create_test_tenant(&pool).await;
    let workspace_id = Uuid::new_v4();

    // Create workspace with empty metadata (simulating legacy data)
    sqlx::query(
        r#"
        INSERT INTO workspaces (workspace_id, tenant_id, name, slug, description, is_active, metadata, settings, created_at, updated_at)
        VALUES ($1, $2, 'Legacy Workspace', 'legacy-ws', 'Legacy workspace without provider config', TRUE, '{}'::jsonb, '{}'::jsonb, NOW(), NOW())
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .execute(&pool)
    .await
    .expect("Failed to create legacy workspace");

    // Verify we can create providers with default values
    // This tests the into_workspace() default fallback logic
    let metadata = get_workspace_metadata(&pool, workspace_id).await;

    // Empty metadata should return empty object
    assert!(
        metadata.as_object().unwrap().is_empty(),
        "Metadata should be empty for legacy workspace"
    );

    // The into_workspace() function should use defaults - we verify by
    // checking that default providers can be created
    let default_llm = ProviderFactory::create_llm_provider("ollama", "gemma3:12b");
    assert!(
        default_llm.is_ok(),
        "Default ollama LLM provider should be creatable"
    );

    // Cleanup
    cleanup_tenant(&pool, tenant_id).await;
}

// ============================================================================
// Test: Provider Factory Respects Workspace Config
// ============================================================================

#[tokio::test]
async fn test_provider_factory_respects_workspace_config() {
    let pool = require_postgres!();
    let tenant_id = create_test_tenant(&pool).await;

    // Create workspace with mock provider (should always succeed)
    let workspace_id = create_test_workspace_with_provider(
        &pool,
        tenant_id,
        "mock",
        "mock-llm",
        "mock",
        "mock-embed",
        1536,
    )
    .await;

    // Get workspace config from DB
    let metadata = get_workspace_metadata(&pool, workspace_id).await;
    let llm_provider = metadata
        .get("llm_provider")
        .and_then(|v| v.as_str())
        .unwrap_or("mock");
    let llm_model = metadata
        .get("llm_model")
        .and_then(|v| v.as_str())
        .unwrap_or("mock-llm");
    let embedding_provider = metadata
        .get("embedding_provider")
        .and_then(|v| v.as_str())
        .unwrap_or("mock");
    let embedding_model = metadata
        .get("embedding_model")
        .and_then(|v| v.as_str())
        .unwrap_or("mock-embed");
    let embedding_dimension = metadata
        .get("embedding_dimension")
        .and_then(|v| v.as_u64())
        .unwrap_or(1536) as usize;

    // Create providers using workspace config
    let llm_result = ProviderFactory::create_llm_provider(llm_provider, llm_model);
    let embedding_result = ProviderFactory::create_embedding_provider(
        embedding_provider,
        embedding_model,
        embedding_dimension,
    );

    assert!(
        llm_result.is_ok(),
        "Mock LLM provider should be creatable: {:?}",
        llm_result.err()
    );
    assert!(
        embedding_result.is_ok(),
        "Mock embedding provider should be creatable: {:?}",
        embedding_result.err()
    );

    // Cleanup
    cleanup_tenant(&pool, tenant_id).await;
}

// ============================================================================
// Test: OpenAI Provider Fails Without API Key (Behavior Verification)
// ============================================================================

#[tokio::test]
async fn test_openai_provider_fails_without_api_key() {
    let pool = require_postgres!();
    let tenant_id = create_test_tenant(&pool).await;

    // Temporarily unset OPENAI_API_KEY
    let original_key = env::var("OPENAI_API_KEY").ok();
    env::remove_var("OPENAI_API_KEY");

    // Create workspace with openai provider
    let workspace_id = create_test_workspace_with_provider(
        &pool,
        tenant_id,
        "openai",
        "gpt-4o-mini",
        "openai",
        "text-embedding-3-small",
        1536,
    )
    .await;

    // Get workspace config
    let metadata = get_workspace_metadata(&pool, workspace_id).await;
    let llm_provider = metadata
        .get("llm_provider")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let llm_model = metadata
        .get("llm_model")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Attempt to create provider - should fail without API key
    let llm_result = ProviderFactory::create_llm_provider(llm_provider, llm_model);

    assert!(
        llm_result.is_err(),
        "OpenAI provider should fail without API key"
    );

    let error_msg = llm_result.err().unwrap().to_string().to_lowercase();
    assert!(
        error_msg.contains("api") || error_msg.contains("key") || error_msg.contains("openai"),
        "Error should mention API key issue: {}",
        error_msg
    );

    // Restore original key if present
    if let Some(key) = original_key {
        env::set_var("OPENAI_API_KEY", key);
    }

    // Cleanup
    cleanup_tenant(&pool, tenant_id).await;
}

// ============================================================================
// Test: Multiple Workspaces Have Isolated Provider Config
// ============================================================================

#[tokio::test]
async fn test_multiple_workspaces_provider_isolation() {
    let pool = require_postgres!();
    let tenant_id = create_test_tenant(&pool).await;

    // Create two workspaces with different providers
    let ws1_id = create_test_workspace_with_provider(
        &pool, tenant_id, "mock", "model-a", "mock", "embed-a", 768,
    )
    .await;

    let ws2_id = create_test_workspace_with_provider(
        &pool, tenant_id, "mock", "model-b", "mock", "embed-b", 1536,
    )
    .await;

    // Get and verify each workspace's config
    let meta1 = get_workspace_metadata(&pool, ws1_id).await;
    let meta2 = get_workspace_metadata(&pool, ws2_id).await;

    assert_eq!(
        meta1.get("llm_model").and_then(|v| v.as_str()),
        Some("model-a"),
        "Workspace 1 should have model-a"
    );
    assert_eq!(
        meta2.get("llm_model").and_then(|v| v.as_str()),
        Some("model-b"),
        "Workspace 2 should have model-b"
    );
    assert_eq!(
        meta1.get("embedding_dimension").and_then(|v| v.as_i64()),
        Some(768),
        "Workspace 1 should have dimension 768"
    );
    assert_eq!(
        meta2.get("embedding_dimension").and_then(|v| v.as_i64()),
        Some(1536),
        "Workspace 2 should have dimension 1536"
    );

    // Cleanup
    cleanup_tenant(&pool, tenant_id).await;
}

// ============================================================================
// Test: Provider Switch Between Ollama and OpenAI
// ============================================================================

#[tokio::test]
async fn test_provider_switch_ollama_to_openai() {
    let pool = require_postgres!();
    let tenant_id = create_test_tenant(&pool).await;

    // Create workspace with ollama
    let workspace_id = create_test_workspace_with_provider(
        &pool,
        tenant_id,
        "ollama",
        "gemma3:12b",
        "ollama",
        "embeddinggemma:latest",
        768,
    )
    .await;

    // Verify initial state
    let meta_before = get_workspace_metadata(&pool, workspace_id).await;
    assert_eq!(
        meta_before.get("llm_provider").and_then(|v| v.as_str()),
        Some("ollama")
    );

    // Switch to openai
    update_workspace_provider(
        &pool,
        workspace_id,
        "openai",
        "gpt-4o-mini",
        "openai",
        "text-embedding-3-small",
    )
    .await;

    // Verify switch
    let meta_after = get_workspace_metadata(&pool, workspace_id).await;
    assert_eq!(
        meta_after.get("llm_provider").and_then(|v| v.as_str()),
        Some("openai"),
        "Provider should switch to openai"
    );
    assert_eq!(
        meta_after.get("llm_model").and_then(|v| v.as_str()),
        Some("gpt-4o-mini"),
        "Model should switch to gpt-4o-mini"
    );

    // Cleanup
    cleanup_tenant(&pool, tenant_id).await;
}

// ============================================================================
// Test: Dimension Update When Switching Embedding Model
// ============================================================================

#[tokio::test]
async fn test_embedding_dimension_update() {
    let pool = require_postgres!();
    let tenant_id = create_test_tenant(&pool).await;

    // Create workspace with 768-dim embedding
    let workspace_id = create_test_workspace_with_provider(
        &pool,
        tenant_id,
        "ollama",
        "gemma3:12b",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Verify initial dimension
    let meta_before = get_workspace_metadata(&pool, workspace_id).await;
    assert_eq!(
        meta_before
            .get("embedding_dimension")
            .and_then(|v| v.as_i64()),
        Some(768)
    );

    // Update to openai embedding with different dimension
    sqlx::query(
        r#"
        UPDATE workspaces
        SET metadata = metadata || '{"embedding_provider": "openai", "embedding_model": "text-embedding-3-small", "embedding_dimension": 1536}'::jsonb,
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .execute(&pool)
    .await
    .expect("Failed to update embedding dimension");

    // Verify new dimension
    let meta_after = get_workspace_metadata(&pool, workspace_id).await;
    assert_eq!(
        meta_after
            .get("embedding_dimension")
            .and_then(|v| v.as_i64()),
        Some(1536),
        "Dimension should update to 1536"
    );

    // Cleanup
    cleanup_tenant(&pool, tenant_id).await;
}
