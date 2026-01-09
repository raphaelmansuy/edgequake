//! # EdgeQuake Auth
//!
//! Authentication and authorization module for EdgeQuake.
//!
//! ## Implements
//!
//! - [`FEAT0501`]: JWT-based user authentication
//! - [`FEAT0502`]: API key authentication for services
//! - [`FEAT0503`]: Role-based access control (RBAC)
//! - [`FEAT0504`]: Multi-tenancy with workspace isolation
//!
//! ## Enforces
//!
//! - [`BR0501`]: All API endpoints require authentication
//! - [`BR0502`]: JWT tokens expire after configured TTL
//! - [`BR0503`]: API keys are hashed before storage
//! - [`BR0504`]: Tenant isolation enforced at query level
//!
//! ## Use Cases
//!
//! - [`UC0501`]: User authenticates via email/password
//! - [`UC0502`]: Service authenticates via API key
//! - [`UC0503`]: Admin manages user roles and permissions
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
pub use extractors::{ApiKeyAuth, AuthUser, OptionalAuth};
pub use jwt::{Claims, JwtService};
pub use password::PasswordService;
pub use rbac::{Permission, RbacService};
pub use types::Role;
pub use types::*;

#[cfg(feature = "multi-tenant")]
pub use tenant::{TenantContext, TenantService};
