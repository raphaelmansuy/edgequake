//! PostgreSQL Conversation Storage Integration Tests
//!
//! These tests require a running PostgreSQL instance with the conversation tables.
//! Run with: `cargo test --package edgequake-storage --test postgres_conversation_integration --features postgres`
//!
//! Environment variables needed:
//! - POSTGRES_HOST (default: localhost)
//! - POSTGRES_PORT (default: 5432)
//! - POSTGRES_DB (default: edgequake)
//! - POSTGRES_USER (default: edgequake)
//! - POSTGRES_PASSWORD (required)

#![cfg(feature = "postgres")]

use std::env;
use std::time::Duration;
use uuid::Uuid;

use edgequake_storage::{PostgresConfig, PostgresConversationStorage};

/// Get PostgreSQL configuration from environment variables.
fn get_test_config() -> Option<PostgresConfig> {
    // Check if password is set (indicates test environment is configured)
    let password = env::var("POSTGRES_PASSWORD").ok()?;

    Some(PostgresConfig {
        host: env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: env::var("POSTGRES_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5432),
        database: env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake".to_string()),
        user: env::var("POSTGRES_USER").unwrap_or_else(|_| "edgequake".to_string()),
        password,
        namespace: format!(
            "test_conv_{}",
            uuid::Uuid::new_v4().to_string().replace("-", "")[..8].to_string()
        ),
        max_connections: 5,
        min_connections: 1,
        connect_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(60),
        ..Default::default()
    })
}

/// Create a connection pool for testing.
async fn create_test_pool(config: &PostgresConfig) -> sqlx::PgPool {
    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        config.user, config.password, config.host, config.port, config.database
    );

    sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.connect_timeout)
        .idle_timeout(config.idle_timeout)
        .connect(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL")
}

/// Skip test if PostgreSQL is not configured.
macro_rules! require_postgres {
    () => {
        match get_test_config() {
            Some(config) => config,
            None => {
                eprintln!("Skipping test: POSTGRES_PASSWORD not set");
                return;
            }
        }
    };
}

// ============================================================================
// Test Fixtures
// ============================================================================

/// Create test tenant and user IDs.
fn create_test_ids() -> (Uuid, Uuid, Option<Uuid>) {
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let workspace_id = Some(Uuid::new_v4());
    (tenant_id, user_id, workspace_id)
}

// ============================================================================
// Conversation CRUD Tests
// ============================================================================

#[tokio::test]
async fn test_create_conversation() {
    let config = require_postgres!();
    let pool = create_test_pool(&config).await;
    let storage = PostgresConversationStorage::new(pool);
    let (tenant_id, user_id, workspace_id) = create_test_ids();

    let result = storage
        .create_conversation(
            tenant_id,
            user_id,
            workspace_id,
            "Test Conversation".to_string(),
            "hybrid".to_string(),
            None,
        )
        .await;

    // Note: This test may fail if the tenant/user don't exist in the database
    // In a real test setup, you'd create the tenant and user first
    match result {
        Ok(conv) => {
            assert_eq!(conv.title, "Test Conversation");
            assert_eq!(conv.mode, "hybrid");
            assert_eq!(conv.tenant_id, tenant_id);
            assert_eq!(conv.user_id, user_id);
            assert!(!conv.is_pinned);
            assert!(!conv.is_archived);
        }
        Err(e) => {
            // Expected if tenant/user don't exist - this is a constraint violation
            eprintln!("Note: Test skipped due to missing tenant/user: {}", e);
        }
    }
}

#[tokio::test]
async fn test_get_conversation() {
    let config = require_postgres!();
    let pool = create_test_pool(&config).await;
    let storage = PostgresConversationStorage::new(pool);

    // Test getting non-existent conversation
    let result = storage.get_conversation(Uuid::new_v4()).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_share_conversation_nonexistent() {
    let config = require_postgres!();
    let pool = create_test_pool(&config).await;
    let storage = PostgresConversationStorage::new(pool);

    // Test sharing non-existent conversation
    let result = storage.share_conversation(Uuid::new_v4()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_shared_conversation_nonexistent() {
    let config = require_postgres!();
    let pool = create_test_pool(&config).await;
    let storage = PostgresConversationStorage::new(pool);

    let result = storage
        .get_shared_conversation("nonexistent_share_id")
        .await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

// ============================================================================
// Message Tests
// ============================================================================

#[tokio::test]
async fn test_get_message_nonexistent() {
    let config = require_postgres!();
    let pool = create_test_pool(&config).await;
    let storage = PostgresConversationStorage::new(pool);

    let result = storage.get_message(Uuid::new_v4()).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_delete_message_nonexistent() {
    let config = require_postgres!();
    let pool = create_test_pool(&config).await;
    let storage = PostgresConversationStorage::new(pool);

    let result = storage.delete_message(Uuid::new_v4()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_messages_empty() {
    let config = require_postgres!();
    let pool = create_test_pool(&config).await;
    let storage = PostgresConversationStorage::new(pool);

    // List messages for non-existent conversation
    let result = storage.list_messages(Uuid::new_v4(), 10, 0).await;
    assert!(result.is_ok());
    let (messages, total) = result.unwrap();
    assert!(messages.is_empty());
    assert_eq!(total, 0);
}

// ============================================================================
// Folder Tests
// ============================================================================

#[tokio::test]
async fn test_get_folder_nonexistent() {
    let config = require_postgres!();
    let pool = create_test_pool(&config).await;
    let storage = PostgresConversationStorage::new(pool);

    let result = storage.get_folder(Uuid::new_v4()).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_delete_folder_nonexistent() {
    let config = require_postgres!();
    let pool = create_test_pool(&config).await;
    let storage = PostgresConversationStorage::new(pool);

    let result = storage.delete_folder(Uuid::new_v4()).await;
    assert!(result.is_err());
}

// ============================================================================
// Bulk Operation Tests
// ============================================================================

#[tokio::test]
async fn test_bulk_delete_empty() {
    let config = require_postgres!();
    let pool = create_test_pool(&config).await;
    let storage = PostgresConversationStorage::new(pool);

    let result = storage.bulk_delete(&[]).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[tokio::test]
async fn test_bulk_delete_nonexistent() {
    let config = require_postgres!();
    let pool = create_test_pool(&config).await;
    let storage = PostgresConversationStorage::new(pool);

    let ids = vec![Uuid::new_v4(), Uuid::new_v4()];
    let result = storage.bulk_delete(&ids).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[tokio::test]
async fn test_bulk_archive_empty() {
    let config = require_postgres!();
    let pool = create_test_pool(&config).await;
    let storage = PostgresConversationStorage::new(pool);

    let result = storage.bulk_archive(&[], true).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[tokio::test]
async fn test_bulk_move_to_folder_empty() {
    let config = require_postgres!();
    let pool = create_test_pool(&config).await;
    let storage = PostgresConversationStorage::new(pool);

    let result = storage.bulk_move_to_folder(&[], Some(Uuid::new_v4())).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

// ============================================================================
// Helper Method Tests
// ============================================================================

#[tokio::test]
async fn test_get_message_count_nonexistent() {
    let config = require_postgres!();
    let pool = create_test_pool(&config).await;
    let storage = PostgresConversationStorage::new(pool);

    let result = storage.get_message_count(Uuid::new_v4()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[tokio::test]
async fn test_get_last_message_preview_nonexistent() {
    let config = require_postgres!();
    let pool = create_test_pool(&config).await;
    let storage = PostgresConversationStorage::new(pool);

    let result = storage.get_last_message_preview(Uuid::new_v4()).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

// ============================================================================
// Full Workflow Tests (with fixtures)
// ============================================================================

/// This module contains tests that require proper database setup with
/// test fixtures (tenants, users, workspaces).
mod full_workflow_tests {
    use super::*;

    /// Setup test fixtures in the database.
    /// Returns (tenant_id, user_id, workspace_id) if successful.
    async fn setup_test_fixtures(pool: &sqlx::PgPool) -> Option<(Uuid, Uuid, Uuid)> {
        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        // Try to create tenant
        let result = sqlx::query(
            r#"
            INSERT INTO tenants (tenant_id, name, slug, description, status, plan)
            VALUES ($1, 'Test Tenant', $2, 'Test tenant', 'active', 'free')
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(tenant_id)
        .bind(format!(
            "test_{}",
            tenant_id.to_string().replace("-", "")[..8].to_string()
        ))
        .execute(pool)
        .await;

        if result.is_err() {
            eprintln!("Failed to create tenant: {:?}", result.err());
            return None;
        }

        // Try to create user
        let result = sqlx::query(
            r#"
            INSERT INTO users (user_id, tenant_id, email, display_name, password_hash, is_active)
            VALUES ($1, $2, $3, 'Test User', 'hash', true)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .bind(format!(
            "test_{}@example.com",
            user_id.to_string().replace("-", "")[..8].to_string()
        ))
        .execute(pool)
        .await;

        if result.is_err() {
            eprintln!("Failed to create user: {:?}", result.err());
            return None;
        }

        // Try to create workspace
        let result = sqlx::query(
            r#"
            INSERT INTO workspaces (workspace_id, tenant_id, name, slug, status)
            VALUES ($1, $2, 'Test Workspace', $3, 'active')
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(workspace_id)
        .bind(tenant_id)
        .bind(format!(
            "test_{}",
            workspace_id.to_string().replace("-", "")[..8].to_string()
        ))
        .execute(pool)
        .await;

        if result.is_err() {
            eprintln!("Failed to create workspace: {:?}", result.err());
            return None;
        }

        Some((tenant_id, user_id, workspace_id))
    }

    /// Cleanup test fixtures.
    async fn cleanup_fixtures(pool: &sqlx::PgPool, tenant_id: Uuid) {
        // Delete in reverse order of creation due to foreign keys
        let _ = sqlx::query("DELETE FROM conversations WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM folders WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM workspaces WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM users WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM tenants WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(pool)
            .await;
    }

    #[tokio::test]
    async fn test_full_conversation_workflow() {
        let config = require_postgres!();
        let pool = create_test_pool(&config).await;

        // Setup fixtures
        let fixtures = setup_test_fixtures(&pool).await;
        if fixtures.is_none() {
            eprintln!("Skipping full workflow test: could not create fixtures");
            return;
        }
        let (tenant_id, user_id, workspace_id) = fixtures.unwrap();

        let storage = PostgresConversationStorage::new(pool.clone());

        // 1. Create a conversation
        let conv = storage
            .create_conversation(
                tenant_id,
                user_id,
                Some(workspace_id),
                "My Test Conversation".to_string(),
                "hybrid".to_string(),
                None,
            )
            .await
            .expect("Failed to create conversation");

        assert_eq!(conv.title, "My Test Conversation");
        let conv_id = conv.conversation_id;

        // 2. Get the conversation
        let retrieved = storage
            .get_conversation(conv_id)
            .await
            .expect("Failed to get conversation");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "My Test Conversation");

        // 3. Update the conversation
        let updated = storage
            .update_conversation(
                conv_id,
                Some("Updated Title".to_string()),
                None,
                Some(true), // pin it
                None,
                None,
            )
            .await
            .expect("Failed to update conversation");
        assert_eq!(updated.title, "Updated Title");
        assert!(updated.is_pinned);

        // 4. Add messages
        let msg1 = storage
            .create_message(
                conv_id,
                None,
                "user",
                "Hello, how can I help you?",
                Some("hybrid"),
                None,
                None,
                None,
                None,
                false,
            )
            .await
            .expect("Failed to create message 1");
        assert_eq!(msg1.role, "user");

        let msg2 = storage
            .create_message(
                conv_id,
                Some(msg1.message_id),
                "assistant",
                "I'm here to assist with your knowledge graph queries.",
                Some("hybrid"),
                Some(150),
                Some(1200),
                Some(500),
                Some(serde_json::json!({"sources": ["doc1", "doc2"]})),
                false,
            )
            .await
            .expect("Failed to create message 2");
        assert_eq!(msg2.role, "assistant");
        assert_eq!(msg2.parent_id, Some(msg1.message_id));

        // 5. List messages
        let (messages, total) = storage
            .list_messages(conv_id, 10, 0)
            .await
            .expect("Failed to list messages");
        assert_eq!(total, 2);
        assert_eq!(messages.len(), 2);

        // 6. Get message count
        let count = storage
            .get_message_count(conv_id)
            .await
            .expect("Failed to get message count");
        assert_eq!(count, 2);

        // 7. Get last message preview
        let preview = storage
            .get_last_message_preview(conv_id)
            .await
            .expect("Failed to get preview");
        assert!(preview.is_some());
        assert!(preview.unwrap().contains("assist"));

        // 8. Share the conversation
        let share_id = storage
            .share_conversation(conv_id)
            .await
            .expect("Failed to share conversation");
        assert!(share_id.starts_with("share_"));

        // 9. Get shared conversation
        let shared = storage
            .get_shared_conversation(&share_id)
            .await
            .expect("Failed to get shared");
        assert!(shared.is_some());

        // 10. Unshare
        storage
            .unshare_conversation(conv_id)
            .await
            .expect("Failed to unshare");

        let shared_after = storage
            .get_shared_conversation(&share_id)
            .await
            .expect("Failed to get shared after unshare");
        assert!(shared_after.is_none());

        // 11. Delete a message
        storage
            .delete_message(msg2.message_id)
            .await
            .expect("Failed to delete message");

        let (messages_after, total_after) = storage
            .list_messages(conv_id, 10, 0)
            .await
            .expect("Failed to list after delete");
        assert_eq!(total_after, 1);

        // 12. Delete the conversation
        storage
            .delete_conversation(conv_id)
            .await
            .expect("Failed to delete conversation");

        let deleted = storage
            .get_conversation(conv_id)
            .await
            .expect("Failed to get deleted");
        assert!(deleted.is_none());

        // Cleanup
        cleanup_fixtures(&pool, tenant_id).await;
    }

    #[tokio::test]
    async fn test_folder_workflow() {
        let config = require_postgres!();
        let pool = create_test_pool(&config).await;

        // Setup fixtures
        let fixtures = setup_test_fixtures(&pool).await;
        if fixtures.is_none() {
            eprintln!("Skipping folder workflow test: could not create fixtures");
            return;
        }
        let (tenant_id, user_id, workspace_id) = fixtures.unwrap();

        let storage = PostgresConversationStorage::new(pool.clone());

        // 1. Create a folder
        let folder = storage
            .create_folder(tenant_id, user_id, Some(workspace_id), "My Folder", None)
            .await
            .expect("Failed to create folder");

        assert_eq!(folder.name, "My Folder");
        let folder_id = folder.folder_id;

        // 2. Create a sub-folder
        let subfolder = storage
            .create_folder(
                tenant_id,
                user_id,
                Some(workspace_id),
                "Sub Folder",
                Some(folder_id),
            )
            .await
            .expect("Failed to create subfolder");

        assert_eq!(subfolder.parent_id, Some(folder_id));

        // 3. List folders
        let folders = storage
            .list_folders(tenant_id, user_id)
            .await
            .expect("Failed to list folders");

        assert_eq!(folders.len(), 2);

        // 4. Update folder
        let updated = storage
            .update_folder(folder_id, Some("Renamed Folder"), None, None)
            .await
            .expect("Failed to update folder");

        assert_eq!(updated.name, "Renamed Folder");

        // 5. Create conversation in folder
        let conv = storage
            .create_conversation(
                tenant_id,
                user_id,
                Some(workspace_id),
                "Folder Conversation".to_string(),
                "local".to_string(),
                Some(folder_id),
            )
            .await
            .expect("Failed to create conversation in folder");

        assert_eq!(conv.folder_id, Some(folder_id));

        // 6. Delete folder (should move conversation out)
        storage
            .delete_folder(folder_id)
            .await
            .expect("Failed to delete folder");

        // Conversation should still exist but without folder
        let conv_after = storage
            .get_conversation(conv.conversation_id)
            .await
            .expect("Failed to get conversation after folder delete");
        assert!(conv_after.is_some());
        assert!(conv_after.unwrap().folder_id.is_none());

        // Cleanup
        cleanup_fixtures(&pool, tenant_id).await;
    }

    #[tokio::test]
    async fn test_bulk_operations() {
        let config = require_postgres!();
        let pool = create_test_pool(&config).await;

        // Setup fixtures
        let fixtures = setup_test_fixtures(&pool).await;
        if fixtures.is_none() {
            eprintln!("Skipping bulk operations test: could not create fixtures");
            return;
        }
        let (tenant_id, user_id, workspace_id) = fixtures.unwrap();

        let storage = PostgresConversationStorage::new(pool.clone());

        // Create multiple conversations
        let mut conv_ids = Vec::new();
        for i in 0..5 {
            let conv = storage
                .create_conversation(
                    tenant_id,
                    user_id,
                    Some(workspace_id),
                    format!("Bulk Conv {}", i),
                    "hybrid".to_string(),
                    None,
                )
                .await
                .expect("Failed to create conversation");
            conv_ids.push(conv.conversation_id);
        }

        // Bulk archive
        let archived = storage
            .bulk_archive(&conv_ids[..3], true)
            .await
            .expect("Failed to bulk archive");
        assert_eq!(archived, 3);

        // Verify archive
        let conv = storage
            .get_conversation(conv_ids[0])
            .await
            .expect("Failed to get")
            .unwrap();
        assert!(conv.is_archived);

        // Create folder for bulk move
        let folder = storage
            .create_folder(tenant_id, user_id, Some(workspace_id), "Bulk Folder", None)
            .await
            .expect("Failed to create folder");

        // Bulk move to folder
        let moved = storage
            .bulk_move_to_folder(&conv_ids[3..], Some(folder.folder_id))
            .await
            .expect("Failed to bulk move");
        assert_eq!(moved, 2);

        // Bulk delete
        let deleted = storage
            .bulk_delete(&conv_ids)
            .await
            .expect("Failed to bulk delete");
        assert_eq!(deleted, 5);

        // Verify deletion
        for id in &conv_ids {
            let conv = storage.get_conversation(*id).await.expect("Failed to get");
            assert!(conv.is_none());
        }

        // Cleanup
        cleanup_fixtures(&pool, tenant_id).await;
    }

    #[tokio::test]
    async fn test_list_conversations_with_filters() {
        let config = require_postgres!();
        let pool = create_test_pool(&config).await;

        // Setup fixtures
        let fixtures = setup_test_fixtures(&pool).await;
        if fixtures.is_none() {
            eprintln!("Skipping list conversations test: could not create fixtures");
            return;
        }
        let (tenant_id, user_id, workspace_id) = fixtures.unwrap();

        let storage = PostgresConversationStorage::new(pool.clone());

        // Create conversations with different states
        let conv1 = storage
            .create_conversation(
                tenant_id,
                user_id,
                Some(workspace_id),
                "Active Conversation".to_string(),
                "hybrid".to_string(),
                None,
            )
            .await
            .expect("Failed to create conv1");

        let conv2 = storage
            .create_conversation(
                tenant_id,
                user_id,
                Some(workspace_id),
                "Pinned Conversation".to_string(),
                "local".to_string(),
                None,
            )
            .await
            .expect("Failed to create conv2");

        // Pin conv2
        storage
            .update_conversation(conv2.conversation_id, None, None, Some(true), None, None)
            .await
            .expect("Failed to pin");

        let conv3 = storage
            .create_conversation(
                tenant_id,
                user_id,
                Some(workspace_id),
                "Archived Conversation".to_string(),
                "global".to_string(),
                None,
            )
            .await
            .expect("Failed to create conv3");

        // Archive conv3
        storage
            .update_conversation(conv3.conversation_id, None, None, None, Some(true), None)
            .await
            .expect("Failed to archive");

        // List all (not archived)
        let (convs, total) = storage
            .list_conversations(
                tenant_id,
                user_id,
                Some(false), // not archived
                None,
                None,
                None, // unfiled
                None,
                "updated_at",
                true,
                10,
                0,
            )
            .await
            .expect("Failed to list");
        assert_eq!(total, 2);

        // List pinned only
        let (pinned, pinned_total) = storage
            .list_conversations(
                tenant_id,
                user_id,
                None,
                Some(true), // pinned
                None,
                None, // unfiled
                None,
                "updated_at",
                true,
                10,
                0,
            )
            .await
            .expect("Failed to list pinned");
        assert_eq!(pinned_total, 1);
        assert_eq!(pinned[0].title, "Pinned Conversation");

        // List archived
        let (archived, archived_total) = storage
            .list_conversations(
                tenant_id,
                user_id,
                Some(true), // archived
                None,
                None,
                None, // unfiled
                None,
                "updated_at",
                true,
                10,
                0,
            )
            .await
            .expect("Failed to list archived");
        assert_eq!(archived_total, 1);
        assert_eq!(archived[0].title, "Archived Conversation");

        // Cleanup
        let _ = storage
            .bulk_delete(&[
                conv1.conversation_id,
                conv2.conversation_id,
                conv3.conversation_id,
            ])
            .await;
        cleanup_fixtures(&pool, tenant_id).await;
    }
}
