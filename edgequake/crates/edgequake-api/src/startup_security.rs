//! Production startup security checks — SPEC-027 IMP-001 / SPEC-083 S-09/S-10.

use edgequake_auth::{AuthConfig, DEFAULT_INSECURE_JWT_SECRET};
use tracing::{error, warn};

use crate::state::security_config::ApiSecurityConfig;

/// Outcome of startup security validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupSecurityOutcome {
    Ok,
    Warn(Vec<String>),
    Fatal(String),
}

/// Validate security configuration before serving traffic.
pub fn validate_startup_security(
    database_url: Option<&str>,
    auth: &AuthConfig,
    security: &ApiSecurityConfig,
) -> StartupSecurityOutcome {
    let mut warnings = Vec::new();
    let production_db = database_url.map(is_non_local_database).unwrap_or(false);

    // SPEC-083 S-09: default/short JWT secret is fatal unless EDGEQUAKE_DEV_MODE.
    if auth.jwt_secret == DEFAULT_INSECURE_JWT_SECRET || auth.jwt_secret.len() < 32 {
        let msg = if auth.jwt_secret == DEFAULT_INSECURE_JWT_SECRET {
            "JWT_SECRET is the insecure default — set a strong secret (≥32 bytes) or EDGEQUAKE_DEV_MODE=true for local only"
                .to_string()
        } else {
            "JWT_SECRET is shorter than 32 bytes — refuse to start without EDGEQUAKE_DEV_MODE"
                .to_string()
        };
        if auth.dev_mode {
            warnings.push(msg);
        } else {
            return StartupSecurityOutcome::Fatal(msg);
        }
    }

    if production_db && !auth.auth_enabled && !auth.dev_mode {
        warnings.push(
            "Authentication disabled with non-local DATABASE_URL — set EDGEQUAKE_DEV_MODE only on local dev"
                .to_string(),
        );
    }

    if auth.auth_enabled && auth.api_keys.is_empty() && auth.master_api_key.is_none() {
        warnings.push(
            "Authentication enabled but no EDGEQUAKE_API_KEYS or EDGEQUAKE_MASTER_API_KEY — configure credentials or use EDGEQUAKE_DEV_MODE for local dev"
                .to_string(),
        );
    }

    // SPEC-083 S-10: production CORS must be an explicit allow-list.
    let cors_empty = security
        .cors_origins
        .as_ref()
        .map(|o| o.is_empty())
        .unwrap_or(true);
    if !auth.dev_mode && production_db && cors_empty {
        return StartupSecurityOutcome::Fatal(
            "EDGEQUAKE_CORS_ORIGINS is required in production (non-local DATABASE_URL); refuse open CORS"
                .to_string(),
        );
    }

    if security.kv_identity_mirror && security.pg_identity_ssot {
        warnings.push(
            "EDGEQUAKE_KV_IDENTITY_MIRROR is ignored when PostgreSQL pool is available (phase 47) — remove this env var"
                .to_string(),
        );
    }

    // LAW-146-20 / G-146-00: ABAC requires authentication (refuse DEV_MODE / auth-off / anonymous).
    if security.doc_abac {
        if auth.dev_mode || !auth.auth_enabled {
            return StartupSecurityOutcome::Fatal(
                "EDGEQUAKE_DOC_ABAC=1 requires authentication enabled and EDGEQUAKE_DEV_MODE off (LAW-146-20)"
                    .to_string(),
            );
        }
        if auth.allow_anonymous {
            return StartupSecurityOutcome::Fatal(
                "EDGEQUAKE_DOC_ABAC=1 refuses EDGEQUAKE_ALLOW_ANONYMOUS (LAW-146-20)"
                    .to_string(),
            );
        }
    }

    if warnings.is_empty() {
        return StartupSecurityOutcome::Ok;
    }

    for message in &warnings {
        warn!(message = %message, "Startup security warning (SPEC-027)");
    }

    if security.strict_startup {
        return StartupSecurityOutcome::Fatal(format!(
            "EDGEQUAKE_STRICT_STARTUP=1: {}",
            warnings.join("; ")
        ));
    }

    StartupSecurityOutcome::Warn(warnings)
}

/// Returns true when `DATABASE_URL` points at a non-local host (SPEC-027 / SPEC-083).
pub fn is_non_local_database(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    !(lower.contains("localhost")
        || lower.contains("127.0.0.1")
        || lower.contains("@host.docker.internal")
        || lower.contains("postgres://edgequake:edgequake@localhost"))
}

/// Log outcome; exit process on fatal when strict startup is enabled.
pub fn enforce_startup_security(outcome: StartupSecurityOutcome) {
    match outcome {
        StartupSecurityOutcome::Ok => {}
        StartupSecurityOutcome::Warn(_) => {}
        StartupSecurityOutcome::Fatal(message) => {
            error!(message = %message, "Refusing to start — fix configuration or enable EDGEQUAKE_DEV_MODE for local development only");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_db_with_auth_off_only_warns() {
        let mut auth = AuthConfig::new("secure-test-secret-spec027-long-enough");
        auth.auth_enabled = false;
        auth.dev_mode = true;
        let security = ApiSecurityConfig::default();
        let outcome = validate_startup_security(
            Some("postgres://edgequake:edgequake@localhost/edgequake"),
            &auth,
            &security,
        );
        assert_eq!(outcome, StartupSecurityOutcome::Ok);
    }

    #[test]
    fn contract_startup_rejects_default_secret() {
        let auth = AuthConfig::default();
        let security = ApiSecurityConfig::default();
        let outcome = validate_startup_security(
            Some("postgres://edgequake:edgequake@localhost/edgequake"),
            &auth,
            &security,
        );
        assert!(matches!(outcome, StartupSecurityOutcome::Fatal(_)));
    }

    #[test]
    fn short_jwt_secret_fatal_without_dev_mode() {
        let auth = AuthConfig::new("too-short-secret");
        let security = ApiSecurityConfig::default();
        let outcome = validate_startup_security(
            Some("postgres://edgequake:edgequake@localhost/edgequake"),
            &auth,
            &security,
        );
        assert!(matches!(outcome, StartupSecurityOutcome::Fatal(_)));
    }

    #[test]
    fn default_jwt_secret_warns_in_dev_mode() {
        let auth = AuthConfig {
            dev_mode: true,
            ..AuthConfig::default()
        };
        let security = ApiSecurityConfig::default();
        let outcome = validate_startup_security(
            Some("postgres://edgequake:edgequake@localhost/edgequake"),
            &auth,
            &security,
        );
        assert!(matches!(outcome, StartupSecurityOutcome::Warn(_)));
    }

    #[test]
    fn remote_db_auth_off_strict_exits_message() {
        // SPEC-083 S-09/S-10: remote DB + auth off is a warning; strict_startup
        // promotes it to Fatal. Must not set dev_mode (that opts out of both
        // the auth-off warning and the prod CORS fatal).
        let auth = AuthConfig {
            auth_enabled: false,
            dev_mode: false,
            jwt_secret: "secure-test-secret-spec027-long-enough".to_string(),
            ..AuthConfig::default()
        };
        let security = ApiSecurityConfig {
            strict_startup: true,
            // Explicit CORS so we exercise the auth-off → strict path, not S-10 CORS fatal.
            cors_origins: Some(vec!["https://app.example.com".into()]),
            ..Default::default()
        };
        let outcome = validate_startup_security(
            Some("postgres://user:pass@db.example.com:5432/edgequake"),
            &auth,
            &security,
        );
        assert!(matches!(outcome, StartupSecurityOutcome::Fatal(_)));
        if let StartupSecurityOutcome::Fatal(msg) = outcome {
            assert!(
                msg.contains("STRICT_STARTUP") && msg.contains("Authentication disabled"),
                "expected strict auth-off fatal, got: {msg}"
            );
        }
    }

    #[test]
    fn production_cors_missing_is_fatal() {
        let auth = AuthConfig::new("secure-test-secret-spec027-long-enough");
        let security = ApiSecurityConfig::default();
        let outcome = validate_startup_security(
            Some("postgres://user:pass@db.example.com:5432/edgequake"),
            &auth,
            &security,
        );
        assert!(matches!(outcome, StartupSecurityOutcome::Fatal(_)));
        if let StartupSecurityOutcome::Fatal(msg) = outcome {
            assert!(msg.contains("CORS"));
        }
    }

    /// SPEC-083 matrix name (S-10) — prod CORS missing is fatal; fail-closed layer has no Any origin.
    #[test]
    fn contract_cors_default_fail_closed_prod() {
        production_cors_missing_is_fatal();

        // Fail-closed without origins must build (AllowOrigin::Any unreachable).
        let security = ApiSecurityConfig {
            cors_origins: None,
            cors_fail_closed: true,
            ..Default::default()
        };
        let _layer = crate::server::build_cors_layer(&security);

        let server_src = include_str!("server.rs");
        assert!(
            server_src.contains("apply_cors_methods_headers")
                && server_src.contains("CORS_FAIL_CLOSED_METHODS"),
            "fail-closed CORS must use explicit method/header allow-lists"
        );
        assert!(
            server_src.contains("AllowOrigin::Any is intentionally unreachable")
                || server_src.contains("AllowOrigin::Any is unreachable"),
            "S-10: document that AllowOrigin::Any is unreachable when cors_fail_closed"
        );
    }

    #[test]
    fn auth_enabled_no_keys_warns() {
        let auth = AuthConfig::new("secure-test-secret-spec027-long-enough");
        let security = ApiSecurityConfig::default();
        let outcome = validate_startup_security(
            Some("postgres://edgequake:edgequake@localhost/edgequake"),
            &auth,
            &security,
        );
        assert!(matches!(outcome, StartupSecurityOutcome::Warn(_)));
    }

    #[test]
    fn kv_identity_mirror_deprecated_warns() {
        let auth = AuthConfig::new("secure-test-secret-spec027-long-enough");
        let security = ApiSecurityConfig {
            kv_identity_mirror: true,
            ..Default::default()
        };
        let outcome = validate_startup_security(
            Some("postgres://edgequake:edgequake@localhost/edgequake"),
            &auth,
            &security,
        );
        assert!(matches!(outcome, StartupSecurityOutcome::Warn(_)));
        if let StartupSecurityOutcome::Warn(w) = outcome {
            assert!(w.iter().any(|m| m.contains("KV_IDENTITY_MIRROR")));
        }
    }

    #[test]
    fn g146_00_doc_abac_refuses_auth_off() {
        let mut auth = AuthConfig::new("secure-test-secret-spec027-long-enough");
        auth.auth_enabled = false;
        auth.dev_mode = false;
        auth.allow_anonymous = false;
        let security = ApiSecurityConfig {
            doc_abac: true,
            ..Default::default()
        };
        let outcome = validate_startup_security(
            Some("postgres://edgequake:edgequake@localhost/edgequake"),
            &auth,
            &security,
        );
        assert!(
            matches!(outcome, StartupSecurityOutcome::Fatal(ref m) if m.contains("DOC_ABAC")),
            "got {outcome:?}"
        );
    }

    #[test]
    fn g146_00_doc_abac_refuses_dev_mode() {
        let mut auth = AuthConfig::new("secure-test-secret-spec027-long-enough");
        auth.auth_enabled = true;
        auth.dev_mode = true;
        auth.allow_anonymous = false;
        let security = ApiSecurityConfig {
            doc_abac: true,
            ..Default::default()
        };
        let outcome = validate_startup_security(
            Some("postgres://edgequake:edgequake@localhost/edgequake"),
            &auth,
            &security,
        );
        assert!(matches!(outcome, StartupSecurityOutcome::Fatal(_)));
    }

    #[test]
    fn g146_00_doc_abac_refuses_anonymous() {
        let mut auth = AuthConfig::new("secure-test-secret-spec027-long-enough");
        auth.auth_enabled = true;
        auth.dev_mode = false;
        auth.allow_anonymous = true;
        auth.api_keys = vec!["eq_test".into()];
        let security = ApiSecurityConfig {
            doc_abac: true,
            ..Default::default()
        };
        let outcome = validate_startup_security(
            Some("postgres://edgequake:edgequake@localhost/edgequake"),
            &auth,
            &security,
        );
        assert!(matches!(outcome, StartupSecurityOutcome::Fatal(_)));
    }
}
