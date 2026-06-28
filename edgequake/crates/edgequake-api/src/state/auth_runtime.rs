//! Authentication service runtime bundle (SPEC-017 P1-04).

use std::sync::Arc;

use edgequake_auth::{AuthConfig, JwtService, OidcConfig, PasswordService, RbacService};

use crate::services::oidc_flow::{OidcFlowService, SharedOidcFlowService};

/// JWT, password hashing, RBAC, and optional OIDC services.
#[derive(Clone)]
pub struct AuthRuntime {
    pub config: AuthConfig,
    pub oidc_config: OidcConfig,
    pub oidc_service: Option<SharedOidcFlowService>,
    pub jwt: Arc<JwtService>,
    pub password: Arc<PasswordService>,
    pub rbac: Arc<RbacService>,
}

impl AuthRuntime {
    pub fn new(config: AuthConfig) -> Self {
        let oidc_config = OidcConfig::from_env();
        let oidc_service = if oidc_config.is_runtime_builtin() {
            Some(Arc::new(OidcFlowService::new(oidc_config.clone())))
        } else {
            None
        };
        Self {
            jwt: Arc::new(JwtService::new(config.clone())),
            password: Arc::new(PasswordService::new(config.clone())),
            rbac: Arc::new(RbacService::new()),
            config,
            oidc_config,
            oidc_service,
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
        assert!(runtime.oidc_service.is_none());
        assert!(Arc::strong_count(&runtime.jwt) >= 1);
    }
}
