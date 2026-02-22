#![cfg(feature = "pipeline")]

//! Comprehensive End-to-End Workspace Service Tests
//!
//! This module provides comprehensive coverage for workspace service:
//! - Tenant CRUD operations
//! - Workspace CRUD operations
//! - Membership management
//! - Access control
//! - Context building
//! - Concurrent operations
//! - Error handling edge cases
//!
//! Run with: `cargo test --package edgequake-core --test e2e_workspace_service`

use uuid::Uuid;

use edgequake_core::types::{
    CreateWorkspaceRequest, Membership, MembershipRole, Tenant, TenantPlan, UpdateWorkspaceRequest,
};
use edgequake_core::workspace_service::{InMemoryWorkspaceService, WorkspaceService};

// ============================================================================
// Tenant CRUD Tests
// ============================================================================

mod tenant_crud_tests {
    use super::*;

    #[tokio::test]
    async fn test_create_tenant_basic() {
        let service = InMemoryWorkspaceService::new();

        let tenant = Tenant::new("Acme Corp", "acme-corp").with_plan(TenantPlan::Basic);

        let created = service.create_tenant(tenant).await.unwrap();

        assert_eq!(created.name, "Acme Corp");
        assert_eq!(created.slug, "acme-corp");
        assert_eq!(created.plan, TenantPlan::Basic);
        assert!(created.is_active);
    }

    #[tokio::test]
    async fn test_create_tenant_all_plans() {
        let service = InMemoryWorkspaceService::new();

        let plans = vec![
            TenantPlan::Free,
            TenantPlan::Basic,
            TenantPlan::Pro,
            TenantPlan::Enterprise,
        ];

        for (i, plan) in plans.into_iter().enumerate() {
            let tenant =
                Tenant::new(&format!("Tenant {}", i), &format!("tenant-{}", i)).with_plan(plan);

            let created = service.create_tenant(tenant).await.unwrap();
            assert_eq!(created.plan, plan);
        }
    }

    #[tokio::test]
    async fn test_create_tenant_duplicate_slug_fails() {
        let service = InMemoryWorkspaceService::new();

        let tenant1 = Tenant::new("First Tenant", "duplicate-slug");
        service.create_tenant(tenant1).await.unwrap();

        let tenant2 = Tenant::new("Second Tenant", "duplicate-slug");
        let result = service.create_tenant(tenant2).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_get_tenant_by_id() {
        let service = InMemoryWorkspaceService::new();

        let tenant = Tenant::new("Test Tenant", "test");
        let created = service.create_tenant(tenant).await.unwrap();

        let retrieved = service.get_tenant(created.tenant_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Tenant");
    }

    #[tokio::test]
    async fn test_get_tenant_by_slug() {
        let service = InMemoryWorkspaceService::new();

        let tenant = Tenant::new("Slug Test", "unique-slug");
        service.create_tenant(tenant).await.unwrap();

        let retrieved = service.get_tenant_by_slug("unique-slug").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Slug Test");
    }

    #[tokio::test]
    async fn test_get_nonexistent_tenant() {
        let service = InMemoryWorkspaceService::new();

        let result = service.get_tenant(Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_tenant() {
        let service = InMemoryWorkspaceService::new();

        let mut tenant = Tenant::new("Original Name", "original");
        tenant = service.create_tenant(tenant).await.unwrap();

        tenant.name = "Updated Name".to_string();
        let updated = service.update_tenant(tenant).await.unwrap();

        assert_eq!(updated.name, "Updated Name");
    }

    #[tokio::test]
    async fn test_update_nonexistent_tenant_fails() {
        let service = InMemoryWorkspaceService::new();

        let tenant = Tenant::new("Ghost", "ghost");
        let result = service.update_tenant(tenant).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_tenant() {
        let service = InMemoryWorkspaceService::new();

        let tenant = Tenant::new("To Delete", "delete-me");
        let created = service.create_tenant(tenant).await.unwrap();

        // Create a workspace
        let request = CreateWorkspaceRequest::new("Workspace");
        service
            .create_workspace(created.tenant_id, request)
            .await
            .unwrap();

        // Delete tenant (should cascade to workspaces)
        service.delete_tenant(created.tenant_id).await.unwrap();

        // Verify tenant is gone
        let result = service.get_tenant(created.tenant_id).await.unwrap();
        assert!(result.is_none());

        // Verify workspaces are gone
        let workspaces = service.list_workspaces(created.tenant_id).await.unwrap();
        assert!(workspaces.is_empty());
    }

    #[tokio::test]
    async fn test_list_tenants_with_pagination() {
        let service = InMemoryWorkspaceService::new();

        // Create 10 tenants
        for i in 0..10 {
            let tenant = Tenant::new(&format!("Tenant {}", i), &format!("tenant-{}", i));
            service.create_tenant(tenant).await.unwrap();
        }

        // Get first page
        let page1 = service.list_tenants(5, 0).await.unwrap();
        assert_eq!(page1.len(), 5);

        // Get second page
        let page2 = service.list_tenants(5, 5).await.unwrap();
        assert_eq!(page2.len(), 5);

        // Verify no overlap
        let page1_ids: Vec<_> = page1.iter().map(|t| t.tenant_id).collect();
        let page2_ids: Vec<_> = page2.iter().map(|t| t.tenant_id).collect();
        assert!(page1_ids.iter().all(|id| !page2_ids.contains(id)));
    }
}

// ============================================================================
// Workspace CRUD Tests
// ============================================================================

mod workspace_crud_tests {
    use super::*;

    async fn create_tenant(service: &InMemoryWorkspaceService) -> Tenant {
        let tenant = Tenant::new("Test Tenant", &format!("test-{}", Uuid::new_v4()));
        service.create_tenant(tenant).await.unwrap()
    }

    #[tokio::test]
    async fn test_create_workspace_basic() {
        let service = InMemoryWorkspaceService::new();
        let tenant = create_tenant(&service).await;

        let request = CreateWorkspaceRequest {
            name: "Knowledge Base".to_string(),
            slug: Some("kb".to_string()),
            description: Some("Main knowledge base".to_string()),
            max_documents: Some(1000),
            llm_model: None,
            llm_provider: None,
            embedding_model: None,
            embedding_provider: None,
            embedding_dimension: None,

            vision_provider: None,
            vision_model: None,
        };

        let workspace = service
            .create_workspace(tenant.tenant_id, request)
            .await
            .unwrap();

        assert_eq!(workspace.name, "Knowledge Base");
        assert_eq!(workspace.slug, "kb");
        assert_eq!(
            workspace.description,
            Some("Main knowledge base".to_string())
        );
        assert!(workspace.is_active);
    }

    #[tokio::test]
    async fn test_create_workspace_auto_generate_slug() {
        let service = InMemoryWorkspaceService::new();
        let tenant = create_tenant(&service).await;

        let request = CreateWorkspaceRequest {
            name: "My Amazing Workspace!".to_string(),
            slug: None, // Should auto-generate
            description: None,
            max_documents: None,
            llm_model: None,
            llm_provider: None,
            embedding_model: None,
            embedding_provider: None,
            embedding_dimension: None,

            vision_provider: None,
            vision_model: None,
        };

        let workspace = service
            .create_workspace(tenant.tenant_id, request)
            .await
            .unwrap();

        // Slug should be generated from name
        assert!(!workspace.slug.is_empty());
        assert!(!workspace.slug.contains(' '));
        assert!(!workspace.slug.contains('!'));
    }

    #[tokio::test]
    async fn test_create_workspace_duplicate_slug_same_tenant_fails() {
        let service = InMemoryWorkspaceService::new();
        let tenant = create_tenant(&service).await;

        let request1 = CreateWorkspaceRequest {
            name: "Workspace 1".to_string(),
            slug: Some("same-slug".to_string()),
            description: None,
            max_documents: None,
            llm_model: None,
            llm_provider: None,
            embedding_model: None,
            embedding_provider: None,
            embedding_dimension: None,

            vision_provider: None,
            vision_model: None,
        };
        service
            .create_workspace(tenant.tenant_id, request1)
            .await
            .unwrap();

        let request2 = CreateWorkspaceRequest {
            name: "Workspace 2".to_string(),
            slug: Some("same-slug".to_string()),
            description: None,
            max_documents: None,
            llm_model: None,
            llm_provider: None,
            embedding_model: None,
            embedding_provider: None,
            embedding_dimension: None,

            vision_provider: None,
            vision_model: None,
        };
        let result = service.create_workspace(tenant.tenant_id, request2).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_workspace_same_slug_different_tenant_succeeds() {
        let service = InMemoryWorkspaceService::new();

        let tenant1 = Tenant::new("Tenant 1", "tenant-1");
        let tenant1 = service.create_tenant(tenant1).await.unwrap();

        let tenant2 = Tenant::new("Tenant 2", "tenant-2");
        let tenant2 = service.create_tenant(tenant2).await.unwrap();

        let request = CreateWorkspaceRequest {
            name: "Main KB".to_string(),
            slug: Some("main".to_string()),
            description: None,
            max_documents: None,
            llm_model: None,
            llm_provider: None,
            embedding_model: None,
            embedding_provider: None,
            embedding_dimension: None,

            vision_provider: None,
            vision_model: None,
        };

        // Same slug in different tenants should work
        service
            .create_workspace(tenant1.tenant_id, request.clone())
            .await
            .unwrap();
        service
            .create_workspace(tenant2.tenant_id, request)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_workspace_limit_enforcement() {
        let service = InMemoryWorkspaceService::new();

        let mut tenant = Tenant::new("Limited", "limited");
        tenant.max_workspaces = 1;
        let tenant = service.create_tenant(tenant).await.unwrap();

        // First workspace succeeds
        let request1 = CreateWorkspaceRequest::new("Workspace 1");
        service
            .create_workspace(tenant.tenant_id, request1)
            .await
            .unwrap();

        // Second workspace fails
        let request2 = CreateWorkspaceRequest::new("Workspace 2");
        let result = service.create_workspace(tenant.tenant_id, request2).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("limit"));
    }

    #[tokio::test]
    async fn test_get_workspace_by_id() {
        let service = InMemoryWorkspaceService::new();
        let tenant = create_tenant(&service).await;

        let request = CreateWorkspaceRequest {
            name: "Test WS".to_string(),
            slug: Some("test".to_string()),
            description: None,
            max_documents: None,
            llm_model: None,
            llm_provider: None,
            embedding_model: None,
            embedding_provider: None,
            embedding_dimension: None,

            vision_provider: None,
            vision_model: None,
        };
        let created = service
            .create_workspace(tenant.tenant_id, request)
            .await
            .unwrap();

        let retrieved = service.get_workspace(created.workspace_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test WS");
    }

    #[tokio::test]
    async fn test_get_workspace_by_slug() {
        let service = InMemoryWorkspaceService::new();
        let tenant = create_tenant(&service).await;

        let request = CreateWorkspaceRequest {
            name: "Slug Test WS".to_string(),
            slug: Some("slug-test".to_string()),
            description: None,
            max_documents: None,
            llm_model: None,
            llm_provider: None,
            embedding_model: None,
            embedding_provider: None,
            embedding_dimension: None,

            vision_provider: None,
            vision_model: None,
        };
        service
            .create_workspace(tenant.tenant_id, request)
            .await
            .unwrap();

        let retrieved = service
            .get_workspace_by_slug(tenant.tenant_id, "slug-test")
            .await
            .unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_update_workspace() {
        let service = InMemoryWorkspaceService::new();
        let tenant = create_tenant(&service).await;

        let request = CreateWorkspaceRequest::new("Original");
        let created = service
            .create_workspace(tenant.tenant_id, request)
            .await
            .unwrap();

        let update = UpdateWorkspaceRequest {
            name: Some("Updated Name".to_string()),
            description: Some("Updated description".to_string()),
            is_active: Some(false),
            max_documents: Some(500),
            ..Default::default()
        };
        let updated = service
            .update_workspace(created.workspace_id, update)
            .await
            .unwrap();

        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.description, Some("Updated description".to_string()));
        assert!(!updated.is_active);
    }

    #[tokio::test]
    async fn test_delete_workspace() {
        let service = InMemoryWorkspaceService::new();
        let tenant = create_tenant(&service).await;

        let request = CreateWorkspaceRequest::new("To Delete");
        let created = service
            .create_workspace(tenant.tenant_id, request)
            .await
            .unwrap();

        service
            .delete_workspace(created.workspace_id)
            .await
            .unwrap();

        let result = service.get_workspace(created.workspace_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_list_workspaces() {
        let service = InMemoryWorkspaceService::new();
        let tenant = create_tenant(&service).await;

        // Create multiple workspaces
        for i in 0..5 {
            let request = CreateWorkspaceRequest {
                name: format!("Workspace {}", i),
                slug: Some(format!("ws-{}", i)),
                description: None,
                max_documents: None,
                llm_model: None,
                llm_provider: None,
                embedding_model: None,
                embedding_provider: None,
                embedding_dimension: None,

                vision_provider: None,
                vision_model: None,
            };
            service
                .create_workspace(tenant.tenant_id, request)
                .await
                .unwrap();
        }

        let workspaces = service.list_workspaces(tenant.tenant_id).await.unwrap();
        assert_eq!(workspaces.len(), 5);
    }

    #[tokio::test]
    async fn test_get_workspace_stats() {
        let service = InMemoryWorkspaceService::new();
        let tenant = create_tenant(&service).await;

        let request = CreateWorkspaceRequest::new("Stats Test");
        let workspace = service
            .create_workspace(tenant.tenant_id, request)
            .await
            .unwrap();

        let stats = service
            .get_workspace_stats(workspace.workspace_id)
            .await
            .unwrap();
        assert_eq!(stats.workspace_id, workspace.workspace_id);
        // In-memory returns zeros
        assert_eq!(stats.document_count, 0);
    }
}

// ============================================================================
// Membership Tests
// ============================================================================

mod membership_tests {
    use super::*;

    async fn setup() -> (InMemoryWorkspaceService, Tenant, Uuid) {
        let service = InMemoryWorkspaceService::new();
        let tenant = Tenant::new("Test", &format!("test-{}", Uuid::new_v4()));
        let tenant = service.create_tenant(tenant).await.unwrap();
        let user_id = Uuid::new_v4();
        (service, tenant, user_id)
    }

    #[tokio::test]
    async fn test_add_membership() {
        let (service, tenant, user_id) = setup().await;

        let membership = Membership::new(user_id, tenant.tenant_id, MembershipRole::Member);
        let created = service.add_membership(membership).await.unwrap();

        assert_eq!(created.user_id, user_id);
        assert_eq!(created.tenant_id, tenant.tenant_id);
        assert_eq!(created.role, MembershipRole::Member);
        assert!(created.is_active);
    }

    #[tokio::test]
    async fn test_add_duplicate_membership_fails() {
        let (service, tenant, user_id) = setup().await;

        let membership1 = Membership::new(user_id, tenant.tenant_id, MembershipRole::Member);
        service.add_membership(membership1).await.unwrap();

        let membership2 = Membership::new(user_id, tenant.tenant_id, MembershipRole::Admin);
        let result = service.add_membership(membership2).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_membership_all_roles() {
        let service = InMemoryWorkspaceService::new();
        let tenant = Tenant::new("Multi Role", "multi-role");
        let tenant = service.create_tenant(tenant).await.unwrap();

        let roles = vec![
            MembershipRole::Readonly,
            MembershipRole::Member,
            MembershipRole::Admin,
            MembershipRole::Owner,
        ];

        for role in roles {
            let user_id = Uuid::new_v4();
            let membership = Membership::new(user_id, tenant.tenant_id, role);
            let created = service.add_membership(membership).await.unwrap();
            assert_eq!(created.role, role);
        }
    }

    #[tokio::test]
    async fn test_get_user_memberships() {
        let service = InMemoryWorkspaceService::new();
        let user_id = Uuid::new_v4();

        // Create multiple tenants and memberships
        for i in 0..3 {
            let tenant = Tenant::new(&format!("Tenant {}", i), &format!("tenant-{}", i));
            let tenant = service.create_tenant(tenant).await.unwrap();

            let membership = Membership::new(user_id, tenant.tenant_id, MembershipRole::Member);
            service.add_membership(membership).await.unwrap();
        }

        let memberships = service.get_user_memberships(user_id).await.unwrap();
        assert_eq!(memberships.len(), 3);
    }

    #[tokio::test]
    async fn test_get_tenant_memberships() {
        let (service, tenant, _) = setup().await;

        // Add multiple users
        for _ in 0..4 {
            let user_id = Uuid::new_v4();
            let membership = Membership::new(user_id, tenant.tenant_id, MembershipRole::Member);
            service.add_membership(membership).await.unwrap();
        }

        let memberships = service
            .get_tenant_memberships(tenant.tenant_id)
            .await
            .unwrap();
        assert_eq!(memberships.len(), 4);
    }

    #[tokio::test]
    async fn test_update_membership_role() {
        let (service, tenant, user_id) = setup().await;

        let membership = Membership::new(user_id, tenant.tenant_id, MembershipRole::Member);
        let created = service.add_membership(membership).await.unwrap();

        let updated = service
            .update_membership_role(created.membership_id, MembershipRole::Admin)
            .await
            .unwrap();

        assert_eq!(updated.role, MembershipRole::Admin);
    }

    #[tokio::test]
    async fn test_remove_membership() {
        let (service, tenant, user_id) = setup().await;

        let membership = Membership::new(user_id, tenant.tenant_id, MembershipRole::Member);
        let created = service.add_membership(membership).await.unwrap();

        service
            .remove_membership(created.membership_id)
            .await
            .unwrap();

        // User should no longer have access
        let has_access = service
            .check_tenant_access(user_id, tenant.tenant_id)
            .await
            .unwrap();
        assert!(!has_access);
    }

    #[tokio::test]
    async fn test_check_tenant_access() {
        let (service, tenant, user_id) = setup().await;

        // No access initially
        let access = service
            .check_tenant_access(user_id, tenant.tenant_id)
            .await
            .unwrap();
        assert!(!access);

        // Grant access
        let membership = Membership::new(user_id, tenant.tenant_id, MembershipRole::Member);
        service.add_membership(membership).await.unwrap();

        // Now has access
        let access = service
            .check_tenant_access(user_id, tenant.tenant_id)
            .await
            .unwrap();
        assert!(access);
    }

    #[tokio::test]
    async fn test_check_workspace_access() {
        let (service, tenant, user_id) = setup().await;

        // Create workspace
        let request = CreateWorkspaceRequest::new("Test WS");
        let workspace = service
            .create_workspace(tenant.tenant_id, request)
            .await
            .unwrap();

        // No access without membership
        let access = service
            .check_workspace_access(user_id, workspace.workspace_id)
            .await
            .unwrap();
        assert!(!access);

        // Grant tenant access
        let membership = Membership::new(user_id, tenant.tenant_id, MembershipRole::Member);
        service.add_membership(membership).await.unwrap();

        // Now should have workspace access
        let access = service
            .check_workspace_access(user_id, workspace.workspace_id)
            .await
            .unwrap();
        assert!(access);
    }

    #[tokio::test]
    async fn test_get_user_role() {
        let (service, tenant, user_id) = setup().await;

        // No role initially
        let role = service
            .get_user_role(user_id, tenant.tenant_id)
            .await
            .unwrap();
        assert!(role.is_none());

        // Add membership with admin role
        let membership = Membership::new(user_id, tenant.tenant_id, MembershipRole::Admin);
        service.add_membership(membership).await.unwrap();

        // Should return Admin
        let role = service
            .get_user_role(user_id, tenant.tenant_id)
            .await
            .unwrap();
        assert_eq!(role, Some(MembershipRole::Admin));
    }
}

// ============================================================================
// Context Building Tests
// ============================================================================

mod context_tests {
    use super::*;

    #[tokio::test]
    async fn test_build_context_basic() {
        let service = InMemoryWorkspaceService::new();

        let tenant = Tenant::new("Context Test", "context-test");
        let tenant = service.create_tenant(tenant).await.unwrap();

        let user_id = Uuid::new_v4();
        let membership = Membership::new(user_id, tenant.tenant_id, MembershipRole::Admin);
        service.add_membership(membership).await.unwrap();

        let context = service
            .build_context(user_id, tenant.tenant_id, None)
            .await
            .unwrap();

        assert!(context.is_valid());
        assert_eq!(context.tenant_id, Some(tenant.tenant_id));
        assert!(context.can_write());
    }

    #[tokio::test]
    async fn test_build_context_with_workspace() {
        let service = InMemoryWorkspaceService::new();

        let tenant = Tenant::new("Context WS Test", "context-ws-test");
        let tenant = service.create_tenant(tenant).await.unwrap();

        let request = CreateWorkspaceRequest::new("Test WS");
        let workspace = service
            .create_workspace(tenant.tenant_id, request)
            .await
            .unwrap();

        let user_id = Uuid::new_v4();
        let membership = Membership::new(user_id, tenant.tenant_id, MembershipRole::Member);
        service.add_membership(membership).await.unwrap();

        let context = service
            .build_context(user_id, tenant.tenant_id, Some(workspace.workspace_id))
            .await
            .unwrap();

        assert!(context.is_valid());
        assert_eq!(context.workspace_id, Some(workspace.workspace_id));
    }

    #[tokio::test]
    async fn test_build_context_without_access_fails() {
        let service = InMemoryWorkspaceService::new();

        let tenant = Tenant::new("No Access", "no-access");
        let tenant = service.create_tenant(tenant).await.unwrap();

        let user_id = Uuid::new_v4();
        // No membership

        let result = service.build_context(user_id, tenant.tenant_id, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_build_context_readonly_role() {
        let service = InMemoryWorkspaceService::new();

        let tenant = Tenant::new("Readonly Test", "readonly-test");
        let tenant = service.create_tenant(tenant).await.unwrap();

        let user_id = Uuid::new_v4();
        let membership = Membership::new(user_id, tenant.tenant_id, MembershipRole::Readonly);
        service.add_membership(membership).await.unwrap();

        let context = service
            .build_context(user_id, tenant.tenant_id, None)
            .await
            .unwrap();

        assert!(context.is_valid());
        // Readonly should not have write access
        assert!(!context.can_write());
    }
}

// ============================================================================
// Concurrent Operation Tests
// ============================================================================

mod concurrent_tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Barrier;

    #[tokio::test]
    async fn test_concurrent_tenant_creation() {
        let service = Arc::new(InMemoryWorkspaceService::new());
        let barrier = Arc::new(Barrier::new(10));

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let service = Arc::clone(&service);
                let barrier = Arc::clone(&barrier);

                tokio::spawn(async move {
                    barrier.wait().await;
                    let tenant = Tenant::new(
                        &format!("Concurrent Tenant {}", i),
                        &format!("concurrent-{}", i),
                    );
                    service.create_tenant(tenant).await
                })
            })
            .collect();

        // Await all handles
        let mut all_ok = true;
        for handle in handles {
            if let Ok(result) = handle.await {
                if result.is_err() {
                    all_ok = false;
                }
            } else {
                all_ok = false;
            }
        }
        assert!(all_ok);
    }

    #[tokio::test]
    async fn test_concurrent_workspace_creation() {
        let service = Arc::new(InMemoryWorkspaceService::new());

        let mut tenant = Tenant::new("Concurrent WS", "concurrent-ws");
        tenant.max_workspaces = 100;
        let tenant = service.create_tenant(tenant).await.unwrap();

        let barrier = Arc::new(Barrier::new(10));

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let service = Arc::clone(&service);
                let barrier = Arc::clone(&barrier);
                let tenant_id = tenant.tenant_id;

                tokio::spawn(async move {
                    barrier.wait().await;
                    let request = CreateWorkspaceRequest {
                        name: format!("Workspace {}", i),
                        slug: Some(format!("ws-{}", i)),
                        description: None,
                        max_documents: None,
                        llm_model: None,
                        llm_provider: None,
                        embedding_model: None,
                        embedding_provider: None,
                        embedding_dimension: None,

                        vision_provider: None,
                        vision_model: None,
                    };
                    service.create_workspace(tenant_id, request).await
                })
            })
            .collect();

        // Await all handles
        let mut all_ok = true;
        for handle in handles {
            if let Ok(result) = handle.await {
                if result.is_err() {
                    all_ok = false;
                }
            } else {
                all_ok = false;
            }
        }
        assert!(all_ok);

        // Verify all 10 workspaces exist
        let workspaces = service.list_workspaces(tenant.tenant_id).await.unwrap();
        assert_eq!(workspaces.len(), 10);
    }

    #[tokio::test]
    async fn test_concurrent_membership_operations() {
        let service = Arc::new(InMemoryWorkspaceService::new());

        let tenant = Tenant::new("Membership Test", "membership-test");
        let tenant = service.create_tenant(tenant).await.unwrap();

        let barrier = Arc::new(Barrier::new(20));

        let handles: Vec<_> = (0..20)
            .map(|_| {
                let service = Arc::clone(&service);
                let barrier = Arc::clone(&barrier);
                let tenant_id = tenant.tenant_id;

                tokio::spawn(async move {
                    barrier.wait().await;
                    let user_id = Uuid::new_v4();
                    let membership = Membership::new(user_id, tenant_id, MembershipRole::Member);
                    service.add_membership(membership).await
                })
            })
            .collect();

        // Await all handles
        let mut all_ok = true;
        for handle in handles {
            if let Ok(result) = handle.await {
                if result.is_err() {
                    all_ok = false;
                }
            } else {
                all_ok = false;
            }
        }
        assert!(all_ok);

        // Verify all memberships
        let memberships = service
            .get_tenant_memberships(tenant.tenant_id)
            .await
            .unwrap();
        assert_eq!(memberships.len(), 20);
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_tenant_name() {
        let service = InMemoryWorkspaceService::new();

        let tenant = Tenant::new("", "empty-name");
        // Should work (validation may be at API layer)
        let result = service.create_tenant(tenant).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_special_characters_in_slug() {
        let service = InMemoryWorkspaceService::new();

        let tenant = Tenant::new("Special", "special-chars-123");
        let result = service.create_tenant(tenant).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_max_workspaces_zero() {
        let service = InMemoryWorkspaceService::new();

        let mut tenant = Tenant::new("Zero Max", "zero-max");
        tenant.max_workspaces = 0;
        let tenant = service.create_tenant(tenant).await.unwrap();

        let request = CreateWorkspaceRequest::new("Test");
        let result = service.create_workspace(tenant.tenant_id, request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_nonexistent_workspace() {
        let service = InMemoryWorkspaceService::new();

        let update = UpdateWorkspaceRequest {
            name: Some("New Name".to_string()),
            description: None,
            is_active: None,
            max_documents: None,
            ..Default::default()
        };

        let result = service.update_workspace(Uuid::new_v4(), update).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_nonexistent_membership() {
        let service = InMemoryWorkspaceService::new();

        let result = service
            .update_membership_role(Uuid::new_v4(), MembershipRole::Admin)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_check_access_nonexistent_workspace() {
        let service = InMemoryWorkspaceService::new();

        let result = service
            .check_workspace_access(Uuid::new_v4(), Uuid::new_v4())
            .await
            .unwrap();

        assert!(!result);
    }
}
