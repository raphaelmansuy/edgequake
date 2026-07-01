//! Production startup security checks — SPEC-027 IMP-001.

use edgequake_auth::AuthConfig;
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

    if auth.jwt_secret == edgequake_auth::DEFAULT_INSECURE_JWT_SECRET {
        warnings.push(
            "JWT_SECRET is the insecure default — set a strong secret in production".to_string(),
        );
    }

    if security.kv_identity_mirror && security.pg_identity_ssot {
        warnings.push(
            "EDGEQUAKE_KV_IDENTITY_MIRROR is ignored when PostgreSQL pool is available (phase 47) — remove this env var"
                .to_string(),
        );
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

fn is_non_local_database(url: &str) -> bool {
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
            error!(message = %message, "Refusing to start — fix configuration or unset EDGEQUAKE_STRICT_STARTUP");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_db_with_auth_off_only_warns() {
        let mut auth = AuthConfig::new("secure-test-secret-spec027");
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
    fn default_jwt_secret_warns_even_on_local_db() {
        let auth = AuthConfig::default();
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
        let mut auth = AuthConfig::default();
        auth.auth_enabled = false;
        auth.dev_mode = true;
        let security = ApiSecurityConfig {
            strict_startup: true,
            ..Default::default()
        };
        let outcome = validate_startup_security(
            Some("postgres://user:pass@db.example.com:5432/edgequake"),
            &auth,
            &security,
        );
        assert!(matches!(outcome, StartupSecurityOutcome::Fatal(_)));
    }

    #[test]
    fn auth_enabled_no_keys_warns() {
        let auth = AuthConfig::new("secure-test-secret-spec027");
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
        let auth = AuthConfig::new("secure-test-secret-spec027");
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
}
