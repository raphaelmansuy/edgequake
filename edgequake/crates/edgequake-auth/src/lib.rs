//! # EdgeQuake Auth
//!
//! Authentication and authorization module for EdgeQuake.
//!
//! This crate provides:
//! - JWT-based authentication for user sessions
//! - API key authentication for service-to-service communication
//! - Role-based access control (RBAC)
//! - Multi-tenancy support (optional feature)
//!
//! ## Features
//!
//! - `multi-tenant`: Enable multi-tenancy support with tenant isolation
//!
//! ## Example
//!
//! ```rust,ignore
//! use edgequake_auth::{AuthService, Claims};
//!
//! let auth_service = AuthService::new(config);
//! let token = auth_service.login("user@example.com", "password").await?;
//! let claims = auth_service.verify_jwt(&token.access_token)?;
//! ```

pub mod config;
pub mod error;
pub mod extractors;
pub mod jwt;
pub mod password;
pub mod rbac;
pub mod types;

#[cfg(feature = "multi-tenant")]
pub mod tenant;

// Re-export main types
pub use config::AuthConfig;
pub use error::{AuthError, AuthResult};
pub use extractors::{AuthUser, ApiKeyAuth, OptionalAuth};
pub use jwt::{Claims, JwtService};
pub use password::PasswordService;
pub use rbac::{Permission, RbacService};
pub use types::Role;
pub use types::*;

#[cfg(feature = "multi-tenant")]
pub use tenant::{TenantContext, TenantService};
