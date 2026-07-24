//! Authentication configuration.

use std::time::Duration;

/// Default JWT secret when `JWT_SECRET` is unset — **must not** be used in production.
pub const DEFAULT_INSECURE_JWT_SECRET: &str = "change-me-in-production-256-bit-secret-key";

/// Built-in authentication mechanisms (SPEC-027 phase 49). OAuth2/OIDC is **not** in-process.
pub const BUILTIN_AUTH_MECHANISMS: &[&str] = &["jwt_password", "api_key"];

/// Compile-time marker: in-process OIDC is opt-in at runtime via `OidcConfig`.
/// `/health` uses `OidcConfig::is_runtime_builtin()` when OIDC env is set.
pub const OAUTH2_OIDC_BUILTIN: bool = false;

/// Recommended external pattern for enterprise SSO (documented in `docs/security/best-practices.md`).
pub const EXTERNAL_SSO_PATTERN: &str = "oauth2-proxy";

/// Authentication service configuration.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// JWT secret key (should be at least 256 bits).
    pub jwt_secret: String,

    /// JWT access token expiry duration.
    pub jwt_expiry: Duration,

    /// Refresh token expiry duration.
    pub refresh_token_expiry: Duration,

    /// API key prefix (e.g., "sk_live_").
    pub api_key_prefix: String,

    /// API key length (excluding prefix).
    pub api_key_length: usize,

    /// Argon2 memory cost (in KiB).
    pub argon2_memory_cost: u32,

    /// Argon2 time cost (iterations).
    pub argon2_time_cost: u32,

    /// Argon2 parallelism.
    pub argon2_parallelism: u32,

    /// Maximum login attempts before lockout.
    pub max_login_attempts: u32,

    /// Account lockout duration.
    pub lockout_duration: Duration,

    /// Whether to require email verification.
    pub require_email_verification: bool,

    /// Default user role for new registrations.
    pub default_role: String,

    /// Whether to allow self-registration.
    pub allow_registration: bool,

    /// Whether protected API routes require authentication.
    pub auth_enabled: bool,

    /// Local development mode — authentication disabled (EDGEQUAKE_DEV_MODE).
    pub dev_mode: bool,

    /// When auth is off (or request unauthenticated), allow a shared per-tenant
    /// guest user for chat/conversations (SPEC-087 / Issue #335).
    ///
    /// Env: `EDGEQUAKE_ALLOW_ANONYMOUS` (default `true`). When `false`, unauthenticated
    /// chat/conversation create returns 401/403 and does not INSERT guest rows.
    pub allow_anonymous: bool,

    /// Optional bootstrap/master API key for secure first-time setup.
    pub master_api_key: Option<String>,

    /// Additional static API keys accepted by the API middleware.
    pub api_keys: Vec<String>,

    /// Optional JWT issuer (`iss`) — validated when set (SPEC-083 S-07).
    pub jwt_issuer: Option<String>,

    /// Optional JWT audience (`aud`) — validated when set (SPEC-083 S-07).
    pub jwt_audience: Option<Vec<String>>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: DEFAULT_INSECURE_JWT_SECRET.to_string(),
            jwt_expiry: Duration::from_secs(24 * 60 * 60), // 24 hours
            refresh_token_expiry: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            api_key_prefix: "sk_".to_string(),
            api_key_length: 32,
            argon2_memory_cost: 65536, // 64 MiB
            argon2_time_cost: 3,
            argon2_parallelism: 4,
            max_login_attempts: 5,
            lockout_duration: Duration::from_secs(15 * 60), // 15 minutes
            require_email_verification: false,
            default_role: "user".to_string(),
            allow_registration: true,
            auth_enabled: true,
            dev_mode: false,
            allow_anonymous: true,
            master_api_key: None,
            api_keys: Vec::new(),
            jwt_issuer: None,
            jwt_audience: None,
        }
    }
}

impl AuthConfig {
    /// Create a new configuration with the given JWT secret.
    pub fn new(jwt_secret: impl Into<String>) -> Self {
        Self {
            jwt_secret: jwt_secret.into(),
            ..Default::default()
        }
    }

    /// Set JWT expiry duration.
    pub fn with_jwt_expiry(mut self, expiry: Duration) -> Self {
        self.jwt_expiry = expiry;
        self
    }

    /// Set refresh token expiry duration.
    pub fn with_refresh_token_expiry(mut self, expiry: Duration) -> Self {
        self.refresh_token_expiry = expiry;
        self
    }

    /// Set API key prefix.
    pub fn with_api_key_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.api_key_prefix = prefix.into();
        self
    }

    /// Set Argon2 parameters.
    pub fn with_argon2_params(
        mut self,
        memory_cost: u32,
        time_cost: u32,
        parallelism: u32,
    ) -> Self {
        self.argon2_memory_cost = memory_cost;
        self.argon2_time_cost = time_cost;
        self.argon2_parallelism = parallelism;
        self
    }

    /// Set maximum login attempts.
    pub fn with_max_login_attempts(mut self, attempts: u32) -> Self {
        self.max_login_attempts = attempts;
        self
    }

    /// Set lockout duration.
    pub fn with_lockout_duration(mut self, duration: Duration) -> Self {
        self.lockout_duration = duration;
        self
    }

    /// Set default role for new users.
    pub fn with_default_role(mut self, role: impl Into<String>) -> Self {
        self.default_role = role.into();
        self
    }

    /// Create configuration from environment variables.
    pub fn from_env() -> Self {
        let jwt_secret =
            std::env::var("JWT_SECRET").unwrap_or_else(|_| DEFAULT_INSECURE_JWT_SECRET.to_string());

        let jwt_expiry_hours: u64 = std::env::var("JWT_EXPIRY_HOURS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(24);

        let refresh_expiry_days: u64 = std::env::var("REFRESH_TOKEN_EXPIRY_DAYS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        let api_key_prefix = std::env::var("API_KEY_PREFIX").unwrap_or_else(|_| "sk_".to_string());

        let max_login_attempts: u32 = std::env::var("MAX_LOGIN_ATTEMPTS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);

        let lockout_minutes: u64 = std::env::var("LOCKOUT_DURATION_MINUTES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(15);

        let allow_registration = parse_bool_env("ALLOW_REGISTRATION", true);
        let dev_mode = parse_bool_env("EDGEQUAKE_DEV_MODE", false);
        let auth_enabled = resolve_auth_enabled_from_env(dev_mode);
        let allow_anonymous = parse_bool_env("EDGEQUAKE_ALLOW_ANONYMOUS", true);

        let master_api_key = std::env::var("EDGEQUAKE_MASTER_API_KEY")
            .ok()
            .or_else(|| std::env::var("EDGEQUAKE_GLOBAL_API_KEY").ok())
            .or_else(|| std::env::var("MASTER_API_KEY").ok())
            .map(|key| key.trim().to_string())
            .filter(|key| !key.is_empty());

        let mut api_keys: Vec<String> = std::env::var("EDGEQUAKE_API_KEYS")
            .ok()
            .or_else(|| std::env::var("API_KEYS").ok())
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();

        if let Some(master_key) = &master_api_key {
            if !api_keys.iter().any(|key| key == master_key) {
                api_keys.push(master_key.clone());
            }
        }

        let jwt_issuer = std::env::var("JWT_ISSUER")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let jwt_audience = std::env::var("JWT_AUDIENCE")
            .ok()
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .filter(|list| !list.is_empty());

        Self {
            jwt_secret,
            jwt_expiry: Duration::from_secs(jwt_expiry_hours * 60 * 60),
            refresh_token_expiry: Duration::from_secs(refresh_expiry_days * 24 * 60 * 60),
            api_key_prefix,
            max_login_attempts,
            lockout_duration: Duration::from_secs(lockout_minutes * 60),
            allow_registration,
            auth_enabled,
            dev_mode,
            allow_anonymous,
            master_api_key,
            api_keys,
            jwt_issuer,
            jwt_audience,
            ..Default::default()
        }
    }
}

/// Resolve whether API authentication is required (SPEC-027 AC-4 phase 44).
///
/// Priority: `EDGEQUAKE_DEV_MODE` → explicit disable → explicit enable env → **secure default true**.
fn resolve_auth_enabled_from_env(dev_mode: bool) -> bool {
    if dev_mode {
        return false;
    }

    if parse_bool_env("EDGEQUAKE_AUTH_DISABLED", false) {
        return false;
    }

    if let Ok(value) = std::env::var("EDGEQUAKE_AUTH_ENABLED") {
        return parse_bool_value(&value);
    }

    if let Ok(value) = std::env::var("AUTH_ENABLED") {
        return parse_bool_value(&value);
    }

    true
}

fn parse_bool_value(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

pub(crate) fn parse_bool_env(var_name: &str, default: bool) -> bool {
    std::env::var(var_name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clear_auth_env_for_tests() {
        for key in [
            "EDGEQUAKE_AUTH_ENABLED",
            "EDGEQUAKE_AUTH_DISABLED",
            "AUTH_ENABLED",
        ] {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn test_default_config() {
        let config = AuthConfig::default();
        assert_eq!(config.jwt_expiry, Duration::from_secs(24 * 60 * 60));
        assert_eq!(config.api_key_prefix, "sk_");
        assert_eq!(config.max_login_attempts, 5);
        assert!(config.auth_enabled);
        assert!(!config.dev_mode);
    }

    #[test]
    fn resolve_auth_enabled_from_env_variants() {
        clear_auth_env_for_tests();
        assert!(resolve_auth_enabled_from_env(false));
        assert!(!resolve_auth_enabled_from_env(true));

        std::env::set_var("EDGEQUAKE_AUTH_ENABLED", "false");
        assert!(!resolve_auth_enabled_from_env(false));
        std::env::remove_var("EDGEQUAKE_AUTH_ENABLED");
    }

    #[test]
    fn test_builder_pattern() {
        let config = AuthConfig::new("my-secret")
            .with_jwt_expiry(Duration::from_secs(3600))
            .with_api_key_prefix("test_")
            .with_max_login_attempts(10);

        assert_eq!(config.jwt_secret, "my-secret");
        assert_eq!(config.jwt_expiry, Duration::from_secs(3600));
        assert_eq!(config.api_key_prefix, "test_");
        assert_eq!(config.max_login_attempts, 10);
    }
}
