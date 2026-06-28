//! OpenID Connect configuration (SPEC-027 phase 54).
//!
//! OIDC is **opt-in** via `EDGEQUAKE_OIDC_ENABLED=true`. When disabled, enterprise SSO
//! remains external (oauth2-proxy per `EXTERNAL_SSO_PATTERN`).

use crate::config::parse_bool_env;

/// Built-in mechanism label for OIDC login routes.
pub const MECHANISM_OIDC: &str = "oidc";

/// Built-in mechanism label for password JWT login.
pub const MECHANISM_JWT_PASSWORD: &str = "jwt_password";

/// Built-in mechanism label for API keys.
pub const MECHANISM_API_KEY: &str = "api_key";

/// Runtime OIDC settings loaded from environment.
#[derive(Debug, Clone, Default)]
pub struct OidcConfig {
    pub enabled: bool,
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub redirect_uri: String,
    /// Optional frontend URL to redirect after successful OIDC (tokens in query).
    pub success_redirect_url: Option<String>,
}

impl OidcConfig {
    pub fn from_env() -> Self {
        let enabled = parse_bool_env("EDGEQUAKE_OIDC_ENABLED", false);
        let issuer_url = std::env::var("EDGEQUAKE_OIDC_ISSUER_URL")
            .unwrap_or_default()
            .trim()
            .to_string();
        let client_id = std::env::var("EDGEQUAKE_OIDC_CLIENT_ID")
            .unwrap_or_default()
            .trim()
            .to_string();
        let client_secret = std::env::var("EDGEQUAKE_OIDC_CLIENT_SECRET")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let redirect_uri = std::env::var("EDGEQUAKE_OIDC_REDIRECT_URI")
            .unwrap_or_default()
            .trim()
            .to_string();
        let success_redirect_url = std::env::var("EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        Self {
            enabled,
            issuer_url,
            client_id,
            client_secret,
            redirect_uri,
            success_redirect_url,
        }
    }

    /// True when OIDC routes are active and `/health` should report builtin OIDC.
    pub fn is_runtime_builtin(&self) -> bool {
        self.enabled
            && self.issuer_url.starts_with("http")
            && !self.client_id.is_empty()
            && self.redirect_uri.starts_with("http")
    }

    /// Auth mechanisms exposed on `/health` (runtime).
    pub fn resolved_auth_mechanisms(&self) -> Vec<String> {
        let mut mechanisms = vec![
            MECHANISM_JWT_PASSWORD.to_string(),
            MECHANISM_API_KEY.to_string(),
        ];
        if self.is_runtime_builtin() {
            mechanisms.push(MECHANISM_OIDC.to_string());
        }
        mechanisms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_oidc_disabled() {
        let cfg = OidcConfig::default();
        assert!(!cfg.is_runtime_builtin());
        assert_eq!(cfg.resolved_auth_mechanisms().len(), 2);
    }
}
