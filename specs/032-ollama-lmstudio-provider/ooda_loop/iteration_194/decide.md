# OODA 194: Decide - PostgreSQL Provider Switching Test Design

**Date**: 2025-01-15
**Focus**: Designing E2E tests for PostgreSQL provider switching

## Test File Structure

Create `/crates/edgequake-api/tests/e2e_postgres_provider_switching.rs`

## Test Cases

### 1. Provider Config Persistence (test_provider_config_persists_to_postgres)

```
Given: A workspace created with mock provider
When: Workspace updated with openai provider
Then: DB query shows provider fields in metadata
```

### 2. Provider Config Read (test_provider_config_reads_from_postgres)

```
Given: Workspace metadata contains custom provider config
When: get_workspace() is called
Then: Workspace struct has correct llm_provider, embedding_provider
```

### 3. Provider Switching Flow (test_provider_switch_affects_processing)

```
Given: Workspace with ollama provider config
When: Update to openai, then process document
Then: Processing attempts to use openai (fails without key, but attempt is logged)
```

### 4. Empty Metadata Defaults (test_empty_metadata_uses_defaults)

```
Given: Workspace created with empty metadata (legacy)
When: into_workspace() converts row
Then: Default providers are used (ollama)
```

### 5. Rebuild After Switch (test_rebuild_uses_updated_provider)

```
Given: Workspace with ollama provider
When: Switch to openai, trigger rebuild_embeddings
Then: Rebuild attempts to use openai embedding provider
```

### 6. Concurrent Updates (test_concurrent_provider_updates)

```
Given: Two concurrent update requests for same workspace
When: Both try to update provider fields
Then: Final state is consistent (last write wins)
```

## Implementation Strategy

1. Use existing `require_postgres!()` macro pattern
2. Test at WorkspaceService layer (DB persistence)
3. Test at DocumentTaskProcessor layer (provider usage)
4. Use mock provider for success cases
5. Use openai without key to verify error handling

## Required Fixtures

```rust
async fn create_test_tenant(pool: &PgPool) -> Uuid {
    let tenant_id = Uuid::new_v4();
    sqlx::query(r#"
        INSERT INTO tenants (tenant_id, name, slug, is_active, metadata, settings, created_at, updated_at)
        VALUES ($1, $2, $3, TRUE, '{}'::jsonb, '{}'::jsonb, NOW(), NOW())
    "#)
    .bind(tenant_id)
    .bind(format!("Test Tenant {}", tenant_id))
    .bind(format!("test-{}", &tenant_id.to_string()[..8]))
    .execute(pool)
    .await.unwrap();
    tenant_id
}

async fn create_test_workspace_with_provider(
    pool: &PgPool,
    tenant_id: Uuid,
    llm_provider: &str,
    embedding_provider: &str,
) -> Uuid {
    let workspace_id = Uuid::new_v4();
    let metadata = serde_json::json!({
        "llm_provider": llm_provider,
        "llm_model": "test-model",
        "embedding_provider": embedding_provider,
        "embedding_model": "test-embed",
        "embedding_dimension": 1536
    });
    sqlx::query(r#"
        INSERT INTO workspaces (workspace_id, tenant_id, name, slug, description, is_active, metadata, settings, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 'Test', TRUE, $5, '{}'::jsonb, NOW(), NOW())
    "#)
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("Provider Test Workspace {}", workspace_id))
    .bind(format!("ws-{}", &workspace_id.to_string()[..8]))
    .bind(metadata)
    .execute(pool)
    .await.unwrap();
    workspace_id
}
```

## Success Criteria

- All 6 tests pass with PostgreSQL backend
- Provider config correctly roundtrips through DB
- Updates to provider config are immediately visible
- Rebuild operations use updated provider config

## Next Step

OODA 195: Act - Implement PostgreSQL provider switching tests
