//! Authentication configuration.

use std::time::Duration;

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
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "change-me-in-production-256-bit-secret-key".to_string(),
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
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "change-me-in-production-256-bit-secret-key".to_string());

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

        let allow_registration: bool = std::env::var("ALLOW_REGISTRATION")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);

        Self {
            jwt_secret,
            jwt_expiry: Duration::from_secs(jwt_expiry_hours * 60 * 60),
            refresh_token_expiry: Duration::from_secs(refresh_expiry_days * 24 * 60 * 60),
            api_key_prefix,
            max_login_attempts,
            lockout_duration: Duration::from_secs(lockout_minutes * 60),
            allow_registration,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AuthConfig::default();
        assert_eq!(config.jwt_expiry, Duration::from_secs(24 * 60 * 60));
        assert_eq!(config.api_key_prefix, "sk_");
        assert_eq!(config.max_login_attempts, 5);
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
