//! Role-Based Access Control (RBAC) service.
//!
//! ## Implements
//!
//! - **FEAT0860**: Role-to-permission mapping
//! - **FEAT0861**: Permission checking for operations
//! - **FEAT0862**: Hierarchical role support
//!
//! ## Use Cases
//!
//! - **UC2510**: System checks user permission before operation
//! - **UC2511**: Admin grants role to user
//! - **UC2512**: System denies access for insufficient permissions
//!
//! ## Enforces
//!
//! - **BR0860**: Each role has fixed permission set
//! - **BR0861**: Admin role has all permissions

use std::str::FromStr;

use crate::error::AuthError;
use crate::types::Role;

/// Available permissions in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission {
    // Document permissions
    DocumentRead,
    DocumentCreate,
    DocumentUpdate,
    DocumentDelete,

    // Entity permissions
    EntityRead,
    EntityCreate,
    EntityUpdate,
    EntityDelete,

    // Relationship permissions
    RelationshipRead,
    RelationshipCreate,
    RelationshipUpdate,
    RelationshipDelete,

    // Query permissions
    QueryExecute,
    QueryAdvanced,

    // Workspace permissions
    WorkspaceRead,
    WorkspaceCreate,
    WorkspaceUpdate,
    WorkspaceDelete,
    WorkspaceManageMembers,

    // User management permissions
    UserRead,
    UserCreate,
    UserUpdate,
    UserDelete,

    // API key permissions
    ApiKeyRead,
    ApiKeyCreate,
    ApiKeyRevoke,

    // System permissions
    SystemAdmin,
    MetricsRead,
    AuditLogRead,

    // Task permissions
    TaskRead,
    TaskCreate,
    TaskCancel,
}

impl Permission {
    /// Get permission name as string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DocumentRead => "document:read",
            Self::DocumentCreate => "document:create",
            Self::DocumentUpdate => "document:update",
            Self::DocumentDelete => "document:delete",

            Self::EntityRead => "entity:read",
            Self::EntityCreate => "entity:create",
            Self::EntityUpdate => "entity:update",
            Self::EntityDelete => "entity:delete",

            Self::RelationshipRead => "relationship:read",
            Self::RelationshipCreate => "relationship:create",
            Self::RelationshipUpdate => "relationship:update",
            Self::RelationshipDelete => "relationship:delete",

            Self::QueryExecute => "query:execute",
            Self::QueryAdvanced => "query:advanced",

            Self::WorkspaceRead => "workspace:read",
            Self::WorkspaceCreate => "workspace:create",
            Self::WorkspaceUpdate => "workspace:update",
            Self::WorkspaceDelete => "workspace:delete",
            Self::WorkspaceManageMembers => "workspace:manage_members",

            Self::UserRead => "user:read",
            Self::UserCreate => "user:create",
            Self::UserUpdate => "user:update",
            Self::UserDelete => "user:delete",

            Self::ApiKeyRead => "api_key:read",
            Self::ApiKeyCreate => "api_key:create",
            Self::ApiKeyRevoke => "api_key:revoke",

            Self::SystemAdmin => "system:admin",
            Self::MetricsRead => "system:metrics",
            Self::AuditLogRead => "system:audit_log",

            Self::TaskRead => "task:read",
            Self::TaskCreate => "task:create",
            Self::TaskCancel => "task:cancel",
        }
    }

    /// Parse permission from string.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

impl FromStr for Permission {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "document:read" => Ok(Self::DocumentRead),
            "document:create" => Ok(Self::DocumentCreate),
            "document:update" => Ok(Self::DocumentUpdate),
            "document:delete" => Ok(Self::DocumentDelete),

            "entity:read" => Ok(Self::EntityRead),
            "entity:create" => Ok(Self::EntityCreate),
            "entity:update" => Ok(Self::EntityUpdate),
            "entity:delete" => Ok(Self::EntityDelete),

            "relationship:read" => Ok(Self::RelationshipRead),
            "relationship:create" => Ok(Self::RelationshipCreate),
            "relationship:update" => Ok(Self::RelationshipUpdate),
            "relationship:delete" => Ok(Self::RelationshipDelete),

            "query:execute" => Ok(Self::QueryExecute),
            "query:advanced" => Ok(Self::QueryAdvanced),

            "workspace:read" => Ok(Self::WorkspaceRead),
            "workspace:create" => Ok(Self::WorkspaceCreate),
            "workspace:update" => Ok(Self::WorkspaceUpdate),
            "workspace:delete" => Ok(Self::WorkspaceDelete),
            "workspace:manage_members" => Ok(Self::WorkspaceManageMembers),

            "user:read" => Ok(Self::UserRead),
            "user:create" => Ok(Self::UserCreate),
            "user:update" => Ok(Self::UserUpdate),
            "user:delete" => Ok(Self::UserDelete),

            "api_key:read" => Ok(Self::ApiKeyRead),
            "api_key:create" => Ok(Self::ApiKeyCreate),
            "api_key:revoke" => Ok(Self::ApiKeyRevoke),

            "system:admin" => Ok(Self::SystemAdmin),
            "system:metrics" => Ok(Self::MetricsRead),
            "system:audit_log" => Ok(Self::AuditLogRead),

            "task:read" => Ok(Self::TaskRead),
            "task:create" => Ok(Self::TaskCreate),
            "task:cancel" => Ok(Self::TaskCancel),

            _ => Err(format!("Unknown permission: {}", s)),
        }
    }
}

/// RBAC service for permission checking.
#[derive(Debug, Clone, Default)]
pub struct RbacService;

impl RbacService {
    /// Create a new RBAC service.
    pub fn new() -> Self {
        Self
    }

    /// Get all permissions for a role.
    pub fn role_permissions(&self, role: &Role) -> Vec<Permission> {
        match role {
            Role::Admin => {
                // Admin has all permissions
                vec![
                    Permission::DocumentRead,
                    Permission::DocumentCreate,
                    Permission::DocumentUpdate,
                    Permission::DocumentDelete,
                    Permission::EntityRead,
                    Permission::EntityCreate,
                    Permission::EntityUpdate,
                    Permission::EntityDelete,
                    Permission::RelationshipRead,
                    Permission::RelationshipCreate,
                    Permission::RelationshipUpdate,
                    Permission::RelationshipDelete,
                    Permission::QueryExecute,
                    Permission::QueryAdvanced,
                    Permission::WorkspaceRead,
                    Permission::WorkspaceCreate,
                    Permission::WorkspaceUpdate,
                    Permission::WorkspaceDelete,
                    Permission::WorkspaceManageMembers,
                    Permission::UserRead,
                    Permission::UserCreate,
                    Permission::UserUpdate,
                    Permission::UserDelete,
                    Permission::ApiKeyRead,
                    Permission::ApiKeyCreate,
                    Permission::ApiKeyRevoke,
                    Permission::SystemAdmin,
                    Permission::MetricsRead,
                    Permission::AuditLogRead,
                    Permission::TaskRead,
                    Permission::TaskCreate,
                    Permission::TaskCancel,
                ]
            }
            Role::User => {
                // Regular user: CRUD on documents, entities, relationships, queries
                vec![
                    Permission::DocumentRead,
                    Permission::DocumentCreate,
                    Permission::DocumentUpdate,
                    Permission::DocumentDelete,
                    Permission::EntityRead,
                    Permission::EntityCreate,
                    Permission::EntityUpdate,
                    Permission::EntityDelete,
                    Permission::RelationshipRead,
                    Permission::RelationshipCreate,
                    Permission::RelationshipUpdate,
                    Permission::RelationshipDelete,
                    Permission::QueryExecute,
                    Permission::QueryAdvanced,
                    Permission::WorkspaceRead,
                    Permission::ApiKeyRead,
                    Permission::ApiKeyCreate,
                    Permission::ApiKeyRevoke,
                    Permission::TaskRead,
                    Permission::TaskCreate,
                    Permission::TaskCancel,
                ]
            }
            Role::Readonly => {
                // Readonly: only read operations
                vec![
                    Permission::DocumentRead,
                    Permission::EntityRead,
                    Permission::RelationshipRead,
                    Permission::QueryExecute,
                    Permission::WorkspaceRead,
                    Permission::ApiKeyRead,
                    Permission::TaskRead,
                ]
            }
        }
    }

    /// Check if a role has a specific permission.
    pub fn has_permission(&self, role: &Role, permission: Permission) -> bool {
        self.role_permissions(role).contains(&permission)
    }

    /// Check if a role has all of the specified permissions.
    pub fn has_all_permissions(&self, role: &Role, permissions: &[Permission]) -> bool {
        let role_perms = self.role_permissions(role);
        permissions.iter().all(|p| role_perms.contains(p))
    }

    /// Check if a role has any of the specified permissions.
    pub fn has_any_permission(&self, role: &Role, permissions: &[Permission]) -> bool {
        let role_perms = self.role_permissions(role);
        permissions.iter().any(|p| role_perms.contains(p))
    }

    /// Require a specific role, returning an error if not met.
    pub fn require_role(&self, role: &Role, required: &Role) -> Result<(), AuthError> {
        // Check hierarchy: Admin > User > Readonly
        let has_access = match required {
            Role::Readonly => true, // Everyone can access readonly-level
            Role::User => matches!(role, Role::Admin | Role::User),
            Role::Admin => matches!(role, Role::Admin),
        };

        if has_access {
            Ok(())
        } else {
            Err(AuthError::Forbidden {
                required_permission: format!("role:{}", required.as_str()),
            })
        }
    }

    /// Require a specific permission, returning an error if not met.
    pub fn require_permission(&self, role: &Role, permission: Permission) -> Result<(), AuthError> {
        if self.has_permission(role, permission) {
            Ok(())
        } else {
            Err(AuthError::Forbidden {
                required_permission: permission.as_str().to_string(),
            })
        }
    }

    /// Require all specified permissions, returning an error if any is missing.
    pub fn require_all_permissions(
        &self,
        role: &Role,
        permissions: &[Permission],
    ) -> Result<(), AuthError> {
        for permission in permissions {
            if !self.has_permission(role, *permission) {
                return Err(AuthError::Forbidden {
                    required_permission: permission.as_str().to_string(),
                });
            }
        }
        Ok(())
    }

    /// Require any of the specified permissions, returning an error if none are met.
    pub fn require_any_permission(
        &self,
        role: &Role,
        permissions: &[Permission],
    ) -> Result<(), AuthError> {
        if self.has_any_permission(role, permissions) {
            Ok(())
        } else {
            let perms: Vec<&str> = permissions.iter().map(|p| p.as_str()).collect();
            Err(AuthError::Forbidden {
                required_permission: perms.join(" | "),
            })
        }
    }

    /// Check if a role can manage another role (for user management).
    pub fn can_manage_role(&self, manager_role: &Role, target_role: &Role) -> bool {
        match manager_role {
            Role::Admin => true, // Admin can manage all roles
            Role::User => matches!(target_role, Role::Readonly), // User can only manage readonly
            Role::Readonly => false, // Readonly cannot manage anyone
        }
    }

    /// Validate that API key scopes cover the required permission.
    pub fn validate_api_key_scopes(
        &self,
        scopes: &[String],
        permission: Permission,
    ) -> Result<(), AuthError> {
        let permission_str = permission.as_str();

        // Check exact match or wildcard
        for scope in scopes {
            if scope == "*" || scope == permission_str {
                return Ok(());
            }

            // Check category wildcard (e.g., "document:*" matches "document:read")
            if scope.ends_with(":*") {
                let category = scope.trim_end_matches(":*");
                if permission_str.starts_with(category) {
                    return Ok(());
                }
            }
        }

        Err(AuthError::InsufficientScope {
            required: permission_str.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_has_all_permissions() {
        let rbac = RbacService::new();
        let perms = rbac.role_permissions(&Role::Admin);

        assert!(perms.contains(&Permission::SystemAdmin));
        assert!(perms.contains(&Permission::UserDelete));
        assert!(perms.contains(&Permission::DocumentRead));
    }

    #[test]
    fn test_user_permissions() {
        let rbac = RbacService::new();

        assert!(rbac.has_permission(&Role::User, Permission::DocumentCreate));
        assert!(rbac.has_permission(&Role::User, Permission::QueryExecute));
        assert!(!rbac.has_permission(&Role::User, Permission::SystemAdmin));
        assert!(!rbac.has_permission(&Role::User, Permission::UserDelete));
    }

    #[test]
    fn test_readonly_permissions() {
        let rbac = RbacService::new();

        assert!(rbac.has_permission(&Role::Readonly, Permission::DocumentRead));
        assert!(rbac.has_permission(&Role::Readonly, Permission::QueryExecute));
        assert!(!rbac.has_permission(&Role::Readonly, Permission::DocumentCreate));
        assert!(!rbac.has_permission(&Role::Readonly, Permission::EntityUpdate));
    }

    #[test]
    fn test_require_role() {
        let rbac = RbacService::new();

        // Admin can access everything
        assert!(rbac.require_role(&Role::Admin, &Role::Admin).is_ok());
        assert!(rbac.require_role(&Role::Admin, &Role::User).is_ok());
        assert!(rbac.require_role(&Role::Admin, &Role::Readonly).is_ok());

        // User can access User and Readonly levels
        assert!(rbac.require_role(&Role::User, &Role::Admin).is_err());
        assert!(rbac.require_role(&Role::User, &Role::User).is_ok());
        assert!(rbac.require_role(&Role::User, &Role::Readonly).is_ok());

        // Readonly can only access Readonly level
        assert!(rbac.require_role(&Role::Readonly, &Role::Admin).is_err());
        assert!(rbac.require_role(&Role::Readonly, &Role::User).is_err());
        assert!(rbac.require_role(&Role::Readonly, &Role::Readonly).is_ok());
    }

    #[test]
    fn test_require_permission() {
        let rbac = RbacService::new();

        assert!(rbac
            .require_permission(&Role::Admin, Permission::SystemAdmin)
            .is_ok());
        assert!(rbac
            .require_permission(&Role::User, Permission::SystemAdmin)
            .is_err());
    }

    #[test]
    fn test_can_manage_role() {
        let rbac = RbacService::new();

        assert!(rbac.can_manage_role(&Role::Admin, &Role::Admin));
        assert!(rbac.can_manage_role(&Role::Admin, &Role::User));
        assert!(rbac.can_manage_role(&Role::Admin, &Role::Readonly));

        assert!(!rbac.can_manage_role(&Role::User, &Role::Admin));
        assert!(!rbac.can_manage_role(&Role::User, &Role::User));
        assert!(rbac.can_manage_role(&Role::User, &Role::Readonly));

        assert!(!rbac.can_manage_role(&Role::Readonly, &Role::Readonly));
    }

    #[test]
    fn test_api_key_scopes() {
        let rbac = RbacService::new();

        // Exact match
        let scopes = vec!["document:read".to_string()];
        assert!(rbac
            .validate_api_key_scopes(&scopes, Permission::DocumentRead)
            .is_ok());
        assert!(rbac
            .validate_api_key_scopes(&scopes, Permission::DocumentCreate)
            .is_err());

        // Wildcard
        let scopes = vec!["*".to_string()];
        assert!(rbac
            .validate_api_key_scopes(&scopes, Permission::SystemAdmin)
            .is_ok());

        // Category wildcard
        let scopes = vec!["document:*".to_string()];
        assert!(rbac
            .validate_api_key_scopes(&scopes, Permission::DocumentRead)
            .is_ok());
        assert!(rbac
            .validate_api_key_scopes(&scopes, Permission::DocumentDelete)
            .is_ok());
        assert!(rbac
            .validate_api_key_scopes(&scopes, Permission::EntityRead)
            .is_err());
    }

    #[test]
    fn test_permission_string_conversion() {
        let permission = Permission::DocumentRead;
        assert_eq!(permission.as_str(), "document:read");
        assert_eq!(
            Permission::parse("document:read"),
            Some(Permission::DocumentRead)
        );
        assert_eq!(Permission::parse("invalid"), None);
    }
}
