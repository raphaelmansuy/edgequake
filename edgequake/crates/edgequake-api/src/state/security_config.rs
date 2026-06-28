//! API security runtime flags — SPEC-027 (IMP-001, IMP-005, IMP-007, IMP-008).
//!
//! All tightening defaults preserve ascending compatibility where noted.
//! Auth defaults **secure** (AC-4 phase 44): use `EDGEQUAKE_DEV_MODE=true` for local open API.

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
    /// PostgreSQL is SSOT for identity when pool available (SPEC-027 phase 38).
    pub pg_identity_ssot: bool,
    /// Parsed from `EDGEQUAKE_KV_IDENTITY_MIRROR` — **ignored** when PG pool + `pg_identity_ssot` (phase 47).
    pub kv_identity_mirror: bool,
    /// Use PostgreSQL RLS via acquired connections for PG handlers (SEC-014 — default on).
    pub pg_rls_enabled: bool,
    /// Comma-separated CORS origins; `None` = allow any (legacy default).
    pub cors_origins: Option<Vec<String>>,
    /// Require `X-EdgeQuake-Confirm: delete-all-documents` for bulk DELETE.
    pub require_delete_all_confirm: bool,
    /// Return HTTP 202 Accepted (with Location) on v1 async RPC when a job/track id is present.
    /// Default **true** (REST-025) — set `EDGEQUAKE_V1_RPC_RETURN_202=0` for legacy 200.
    pub v1_rpc_return_202: bool,
}

impl Default for ApiSecurityConfig {
    fn default() -> Self {
        Self {
            strict_startup: false,
            enable_ollama_compat: true,
            rate_limit_enabled: false,
            strict_tenant_bind: false,
            pg_identity_ssot: true,
            kv_identity_mirror: false,
            pg_rls_enabled: true,
            cors_origins: None,
            require_delete_all_confirm: false,
            v1_rpc_return_202: true,
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
            pg_identity_ssot: parse_bool_env("EDGEQUAKE_PG_IDENTITY_SSOT", true),
            kv_identity_mirror: parse_bool_env("EDGEQUAKE_KV_IDENTITY_MIRROR", false),
            pg_rls_enabled: parse_bool_env("EDGEQUAKE_PG_RLS_ENABLED", true),
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
            v1_rpc_return_202: parse_bool_env("EDGEQUAKE_V1_RPC_RETURN_202", true),
        }
    }
}

fn parse_bool_env(var_name: &str, default: bool) -> bool {
    std::env::var(var_name)
        .ok()
        .as_deref()
        .map(parse_bool_value)
        .unwrap_or(default)
}

fn parse_bool_value(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pg_rls_defaults_true() {
        let cfg = ApiSecurityConfig::default();
        assert!(cfg.pg_rls_enabled);
        assert!(cfg.pg_identity_ssot);
        assert!(!cfg.kv_identity_mirror);
    }

    #[test]
    fn v1_rpc_202_defaults_true() {
        let cfg = ApiSecurityConfig::default();
        assert!(cfg.v1_rpc_return_202);
    }

    #[test]
    fn parse_bool_env_respects_explicit_false() {
        assert!(!parse_bool_value("0"));
        assert!(!parse_bool_value("false"));
    }
}
