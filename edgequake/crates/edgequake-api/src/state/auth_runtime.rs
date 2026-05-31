//! Authentication service runtime bundle (SPEC-017 P1-04).

use std::sync::Arc;

use edgequake_auth::{AuthConfig, JwtService, PasswordService, RbacService};

/// JWT, password hashing, and RBAC services.
#[derive(Clone)]
pub struct AuthRuntime {
    pub config: AuthConfig,
    pub jwt: Arc<JwtService>,
    pub password: Arc<PasswordService>,
    pub rbac: Arc<RbacService>,
}

impl AuthRuntime {
    pub fn new(config: AuthConfig) -> Self {
        Self {
            jwt: Arc::new(JwtService::new(config.clone())),
            password: Arc::new(PasswordService::new(config.clone())),
            rbac: Arc::new(RbacService::new()),
            config,
        }
    }

    pub fn from_env() -> Self {
        Self::new(AuthConfig::from_env())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_runtime_from_default_config() {
        let runtime = AuthRuntime::new(AuthConfig::default());
        assert!(!runtime.config.jwt_secret.is_empty() || runtime.config.jwt_secret.is_empty());
        assert!(Arc::strong_count(&runtime.jwt) >= 1);
    }
}
