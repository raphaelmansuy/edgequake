//! Axum extractors for authentication.

use std::sync::Arc;

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header::AUTHORIZATION, request::Parts},
};
use uuid::Uuid;

use crate::error::AuthError;
use crate::jwt::{Claims, JwtService};
use crate::rbac::{Permission, RbacService};
use crate::types::Role;

/// State required for authentication extractors.
#[derive(Clone)]
pub struct AuthState {
    pub jwt_service: Arc<JwtService>,
    pub rbac_service: Arc<RbacService>,
}

impl AuthState {
    /// Create new auth state.
    pub fn new(jwt_service: JwtService, rbac_service: RbacService) -> Self {
        Self {
            jwt_service: Arc::new(jwt_service),
            rbac_service: Arc::new(rbac_service),
        }
    }

    /// Extract user from Authorization header.
    pub fn extract_user(&self, parts: &Parts) -> Result<AuthUser, AuthError> {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(AuthError::MissingToken)?;

        let token =
            auth_header
                .strip_prefix("Bearer ")
                .ok_or(AuthError::InvalidAuthorizationHeader {
                    reason: "Expected 'Bearer <token>' format".to_string(),
                })?;

        let claims = self.jwt_service.verify_token(token)?;
        let user_id = claims.user_id()?;
        let role = claims.role();

        Ok(AuthUser {
            user_id,
            role,
            claims,
        })
    }

    /// Try to extract user from Authorization header, returns None if not present.
    pub fn try_extract_user(&self, parts: &Parts) -> Option<AuthUser> {
        self.extract_user(parts).ok()
    }
}

/// Authenticated user extracted from JWT.
#[derive(Debug, Clone)]
pub struct AuthUser {
    /// User ID.
    pub user_id: Uuid,

    /// User's role.
    pub role: Role,

    /// Full claims from the JWT.
    pub claims: Claims,
}

impl AuthUser {
    /// Check if user has a specific permission.
    pub fn has_permission(&self, rbac: &RbacService, permission: Permission) -> bool {
        rbac.has_permission(&self.role, permission)
    }

    /// Require a specific permission.
    pub fn require_permission(
        &self,
        rbac: &RbacService,
        permission: Permission,
    ) -> Result<(), AuthError> {
        rbac.require_permission(&self.role, permission)
    }

    /// Check if user is admin.
    pub fn is_admin(&self) -> bool {
        matches!(self.role, Role::Admin)
    }

    /// Get tenant ID if present.
    pub fn tenant_id(&self) -> Option<&str> {
        self.claims.tenant_id.as_deref()
    }

    /// Get workspace ID if present.
    pub fn workspace_id(&self) -> Option<&str> {
        self.claims.workspace_id.as_deref()
    }
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AuthState: FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_state = AuthState::from_ref(state);
        auth_state.extract_user(parts)
    }
}

/// Optional authentication - extracts user if present, None otherwise.
#[derive(Debug, Clone)]
pub struct OptionalAuth(pub Option<AuthUser>);

impl OptionalAuth {
    /// Get the authenticated user if present.
    pub fn user(&self) -> Option<&AuthUser> {
        self.0.as_ref()
    }

    /// Check if user is authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.0.is_some()
    }
}

impl<S> FromRequestParts<S> for OptionalAuth
where
    S: Send + Sync,
    AuthState: FromRef<S>,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_state = AuthState::from_ref(state);
        Ok(OptionalAuth(auth_state.try_extract_user(parts)))
    }
}

/// API key authentication extractor.
#[derive(Debug, Clone)]
pub struct ApiKeyAuth {
    /// API key ID.
    pub key_id: Uuid,

    /// User ID associated with the key.
    pub user_id: Uuid,

    /// API key prefix (for identification).
    pub key_prefix: String,

    /// Scopes granted to this key.
    pub scopes: Vec<String>,
}

impl ApiKeyAuth {
    /// Check if key has a specific scope.
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == "*" || s == scope)
    }

    /// Validate scope for a permission.
    pub fn validate_scope(
        &self,
        rbac: &RbacService,
        permission: Permission,
    ) -> Result<(), AuthError> {
        rbac.validate_api_key_scopes(&self.scopes, permission)
    }
}

/// Combined authentication - accepts either JWT or API key.
#[derive(Debug, Clone)]
pub enum CombinedAuth {
    /// Authenticated via JWT.
    Jwt(AuthUser),
    /// Authenticated via API key.
    ApiKey(ApiKeyAuth),
}

impl CombinedAuth {
    /// Get user ID regardless of auth method.
    pub fn user_id(&self) -> Uuid {
        match self {
            Self::Jwt(user) => user.user_id,
            Self::ApiKey(key) => key.user_id,
        }
    }

    /// Check if authenticated via JWT.
    pub fn is_jwt(&self) -> bool {
        matches!(self, Self::Jwt(_))
    }

    /// Check if authenticated via API key.
    pub fn is_api_key(&self) -> bool {
        matches!(self, Self::ApiKey(_))
    }
}

/// Require admin role - can be used as an extractor layer.
#[derive(Debug, Clone)]
pub struct RequireAdmin(pub AuthUser);

impl<S> FromRequestParts<S> for RequireAdmin
where
    S: Send + Sync,
    AuthState: FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_state = AuthState::from_ref(state);
        let user = auth_state.extract_user(parts)?;

        if !user.is_admin() {
            return Err(AuthError::Forbidden {
                required_permission: "role:admin".to_string(),
            });
        }

        Ok(RequireAdmin(user))
    }
}

/// Require user role (or higher) - can be used as an extractor layer.
#[derive(Debug, Clone)]
pub struct RequireUser(pub AuthUser);

impl<S> FromRequestParts<S> for RequireUser
where
    S: Send + Sync,
    AuthState: FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_state = AuthState::from_ref(state);
        let user = auth_state.extract_user(parts)?;

        auth_state
            .rbac_service
            .require_role(&user.role, &Role::User)?;

        Ok(RequireUser(user))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_user_permissions() {
        let rbac = RbacService::default();

        let admin_claims = Claims::new(Uuid::new_v4(), Role::Admin, 3600);
        let admin = AuthUser {
            user_id: admin_claims.user_id().unwrap(),
            role: Role::Admin,
            claims: admin_claims,
        };

        assert!(admin.has_permission(&rbac, Permission::SystemAdmin));
        assert!(admin.is_admin());

        let user_claims = Claims::new(Uuid::new_v4(), Role::User, 3600);
        let user = AuthUser {
            user_id: user_claims.user_id().unwrap(),
            role: Role::User,
            claims: user_claims,
        };

        assert!(!user.has_permission(&rbac, Permission::SystemAdmin));
        assert!(user.has_permission(&rbac, Permission::DocumentCreate));
        assert!(!user.is_admin());
    }

    #[test]
    fn test_api_key_scopes() {
        let key = ApiKeyAuth {
            key_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            key_prefix: "sk_test_".to_string(),
            scopes: vec!["document:read".to_string(), "document:create".to_string()],
        };

        assert!(key.has_scope("document:read"));
        assert!(key.has_scope("document:create"));
        assert!(!key.has_scope("document:delete"));

        // Wildcard scope
        let wildcard_key = ApiKeyAuth {
            scopes: vec!["*".to_string()],
            ..key
        };
        assert!(wildcard_key.has_scope("anything"));
    }

    #[test]
    fn test_optional_auth() {
        let auth = OptionalAuth(None);
        assert!(!auth.is_authenticated());
        assert!(auth.user().is_none());

        let claims = Claims::new(Uuid::new_v4(), Role::User, 3600);
        let user = AuthUser {
            user_id: claims.user_id().unwrap(),
            role: Role::User,
            claims,
        };
        let auth = OptionalAuth(Some(user));
        assert!(auth.is_authenticated());
        assert!(auth.user().is_some());
    }

    #[test]
    fn test_combined_auth() {
        let user_id = Uuid::new_v4();
        let claims = Claims::new(user_id, Role::User, 3600);
        let user = AuthUser {
            user_id,
            role: Role::User,
            claims,
        };

        let jwt_auth = CombinedAuth::Jwt(user);
        assert!(jwt_auth.is_jwt());
        assert!(!jwt_auth.is_api_key());
        assert_eq!(jwt_auth.user_id(), user_id);

        let key_user_id = Uuid::new_v4();
        let key = ApiKeyAuth {
            key_id: Uuid::new_v4(),
            user_id: key_user_id,
            key_prefix: "sk_".to_string(),
            scopes: vec![],
        };

        let api_auth = CombinedAuth::ApiKey(key);
        assert!(!api_auth.is_jwt());
        assert!(api_auth.is_api_key());
        assert_eq!(api_auth.user_id(), key_user_id);
    }
}
