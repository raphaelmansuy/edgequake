//! API security runtime flags — SPEC-027 (IMP-001, IMP-005, IMP-007, IMP-008).
//!
//! All tightening defaults preserve ascending compatibility (AC-4): flags default OFF
//! or to permissive values matching pre-SPEC-027 behavior.

/// Runtime security configuration loaded from environment.
#[derive(Debug, Clone)]
pub struct ApiSecurityConfig {
    /// Exit on insecure production config when `EDGEQUAKE_STRICT_STARTUP=1`.
    pub strict_startup: bool,
    /// Ollama-compatible `/api/*` shim (default true — existing deployments).
    pub enable_ollama_compat: bool,
    /// Apply tenant rate-limit middleware on `/api/v1/*`.
    pub rate_limit_enabled: bool,
    /// Reject JWT/header tenant mismatch with 403.
    pub strict_tenant_bind: bool,
    /// Comma-separated CORS origins; `None` = allow any (legacy default).
    pub cors_origins: Option<Vec<String>>,
    /// Require `X-EdgeQuake-Confirm: delete-all-documents` for bulk DELETE.
    pub require_delete_all_confirm: bool,
}

impl Default for ApiSecurityConfig {
    fn default() -> Self {
        Self {
            strict_startup: false,
            enable_ollama_compat: true,
            rate_limit_enabled: false,
            strict_tenant_bind: false,
            cors_origins: None,
            require_delete_all_confirm: false,
        }
    }
}

impl ApiSecurityConfig {
    /// Load from environment (SPEC-027 ascending-compat defaults).
    pub fn from_env() -> Self {
        Self {
            strict_startup: parse_bool_env("EDGEQUAKE_STRICT_STARTUP", false),
            enable_ollama_compat: parse_bool_env("EDGEQUAKE_OLLAMA_COMPAT_ENABLED", true),
            rate_limit_enabled: parse_bool_env("EDGEQUAKE_RATE_LIMIT_ENABLED", false),
            strict_tenant_bind: parse_bool_env("EDGEQUAKE_STRICT_TENANT_BIND", false),
            cors_origins: std::env::var("EDGEQUAKE_CORS_ORIGINS").ok().map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            }),
            require_delete_all_confirm: parse_bool_env(
                "EDGEQUAKE_REQUIRE_DELETE_ALL_CONFIRM",
                false,
            ),
        }
    }
}

fn parse_bool_env(var_name: &str, default: bool) -> bool {
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
