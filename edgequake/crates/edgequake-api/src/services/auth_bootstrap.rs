//! First-run admin bootstrap when auth is enabled (GitHub #288 / SPEC-027 AC-4).
//!
//! ## First principles
//!
//! v0.15+ enables authentication by default. Login requires at least one user row in
//! PostgreSQL with a real password hash. Fresh installs and upgrades that only had KV
//! identity rows therefore return 401 until an admin exists.
//!
//! This module closes the gap by creating a bootstrap admin from
//! `EDGEQUAKE_BOOTSTRAP_ADMIN_*` when no login-capable users remain.
//!
//! ## Local pin login (`EDGEQUAKE_DEV_PIN_LOGIN`)
//!
//! `make dev` pins a well-known local account (`admin` / `EdgeQuake1`) on every boot so
//! credentials stay stable across Postgres volume reuse. Pinning refuses non-local
//! `DATABASE_URL` values.
//!
//! SPEC-091 Wave B7: the legacy KV `auth:user:*` import shim is removed —
//! identity is PostgreSQL-native, and remaining `auth:%` KV keys are purged
//! by migration 120. Deployments upgrading from the KV-identity era must pass
//! through an intermediate release that still carried the importer.

use chrono::Utc;
use edgequake_auth::Role;
use tracing::{info, warn};
use uuid::Uuid;

use crate::handlers::auth::UserRecord;
use crate::startup_security::is_non_local_database;
use crate::state::{AppState, PostgresRuntime};

/// Returns true when the stored hash can authenticate a password login.
pub fn is_login_capable_password_hash(hash: &str) -> bool {
    let hash = hash.trim();
    if hash.is_empty() || hash == "anonymous" || hash == "not_a_real_hash" {
        return false;
    }
    hash.starts_with("$argon2") || hash.starts_with("$2")
}

fn env_truthy(name: &str) -> bool {
    std::env::var(name)
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Whether `EDGEQUAKE_DEV_PIN_LOGIN` is set and the database is local-only.
fn resolve_dev_pin_login() -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    if !env_truthy("EDGEQUAKE_DEV_PIN_LOGIN") {
        return Ok(false);
    }
    let database_url = std::env::var("DATABASE_URL").unwrap_or_default();
    if database_url.trim().is_empty() {
        return Err(
            "EDGEQUAKE_DEV_PIN_LOGIN requires DATABASE_URL (local Postgres only)".into(),
        );
    }
    if is_non_local_database(&database_url) {
        return Err(
            "EDGEQUAKE_DEV_PIN_LOGIN refuses non-local DATABASE_URL (local development only)"
                .into(),
        );
    }
    Ok(true)
}

/// Bootstrap identity when auth is required but no login-capable users exist.
///
/// When `EDGEQUAKE_DEV_PIN_LOGIN=1` and DATABASE_URL is local, also reset the
/// bootstrap admin password on every boot so `make dev` credentials stay fixed.
pub async fn bootstrap_auth_identity_if_needed(
    state: &AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !state.auth.config.auth_enabled || state.auth.config.dev_mode {
        return Ok(());
    }

    #[cfg(not(feature = "postgres"))]
    {
        return Ok(());
    }

    #[cfg(feature = "postgres")]
    {
        let Some(pool) = state.pg_pool.as_ref() else {
            return Ok(());
        };

        let pin_login = resolve_dev_pin_login()?;

        let pg_runtime = PostgresRuntime {
            pool: state.pg_pool.clone(),
            capabilities: state.postgres_capabilities.clone(),
        };

        let username = std::env::var("EDGEQUAKE_BOOTSTRAP_ADMIN_USERNAME")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "admin".to_string());

        let email = std::env::var("EDGEQUAKE_BOOTSTRAP_ADMIN_EMAIL")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("{username}@localhost"));

        let password = std::env::var("EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let Some(password) = password else {
            let login_users = crate::services::identity_storage::count_login_capable_users_pg(
                pool,
                &state.security,
            )
            .await?;
            if login_users == 0 {
                warn!(
                    username = %username,
                    "Authentication is enabled but no login-capable users exist. \
                     Set EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD (and optionally EDGEQUAKE_BOOTSTRAP_ADMIN_USERNAME) \
                     or create a user with the master API key. \
                     For local open API, set EDGEQUAKE_DEV_MODE=true or use `make dev-open`."
                );
            }
            return Ok(());
        };

        if let Some(existing) = crate::services::identity_storage::find_user_record_by_login(
            &state.storage,
            Some(&pg_runtime),
            &state.security,
            &username,
        )
        .await?
        {
            let already_login_capable = is_login_capable_password_hash(&existing.password_hash);
            if already_login_capable && !pin_login {
                return Ok(());
            }

            let password_hash = state
                .auth
                .password
                .hash_password(&password)
                .map_err(|e| format!("bootstrap admin password hash failed: {e}"))?;

            let mut record = existing;
            record.password_hash = password_hash;
            record.role = Role::Admin.to_string();
            record.is_active = true;
            record.updated_at = Utc::now();
            record.failed_login_attempts = 0;
            record.locked_until = None;

            crate::services::identity_storage::persist_user_record(
                &state.storage,
                Some(&pg_runtime),
                &state.security,
                &record,
            )
            .await?;

            if pin_login {
                info!(
                    user_id = %record.user_id,
                    username = %username,
                    "Pinned local-dev bootstrap admin password (EDGEQUAKE_DEV_PIN_LOGIN)"
                );
            } else {
                info!(
                    user_id = %record.user_id,
                    username = %username,
                    "Upgraded existing user to login-capable bootstrap admin (GitHub #288)"
                );
            }
            return Ok(());
        }

        let password_hash = state
            .auth
            .password
            .hash_password(&password)
            .map_err(|e| format!("bootstrap admin password hash failed: {e}"))?;

        let user_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let record = UserRecord {
            user_id: user_id.clone(),
            username: username.clone(),
            email,
            password_hash,
            role: Role::Admin.to_string(),
            is_active: true,
            created_at: now,
            updated_at: now,
            last_login_at: None,
            failed_login_attempts: 0,
            locked_until: None,
            metadata: Default::default(),
        };

        crate::services::identity_storage::persist_user_record(
            &state.storage,
            Some(&pg_runtime),
            &state.security,
            &record,
        )
        .await?;

        info!(
            user_id = %user_id,
            username = %username,
            pin_login,
            "Created bootstrap admin user (GitHub #288 — set EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD before first boot)"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn login_capable_hash_detection() {
        assert!(is_login_capable_password_hash(
            "$argon2id$v=19$m=65536,t=3,p=4$abc$def"
        ));
        assert!(is_login_capable_password_hash(
            "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/X4.G2oG0Kq5K5K5K5"
        ));
        assert!(!is_login_capable_password_hash("anonymous"));
        assert!(!is_login_capable_password_hash("not_a_real_hash"));
        assert!(!is_login_capable_password_hash(""));
    }

    #[test]
    fn pin_login_refuses_empty_database_url() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        std::env::set_var("EDGEQUAKE_DEV_PIN_LOGIN", "1");
        std::env::remove_var("DATABASE_URL");
        let err = resolve_dev_pin_login().expect_err("must refuse empty DATABASE_URL");
        assert!(err.to_string().contains("DATABASE_URL"));
        std::env::remove_var("EDGEQUAKE_DEV_PIN_LOGIN");
    }

    #[test]
    fn pin_login_refuses_non_local_database() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        std::env::set_var("EDGEQUAKE_DEV_PIN_LOGIN", "1");
        std::env::set_var(
            "DATABASE_URL",
            "postgres://edgequake:edgequake@db.example.com:5432/edgequake",
        );
        let err = resolve_dev_pin_login().expect_err("must refuse remote DATABASE_URL");
        assert!(err.to_string().contains("non-local"));
        std::env::remove_var("EDGEQUAKE_DEV_PIN_LOGIN");
        std::env::remove_var("DATABASE_URL");
    }

    #[test]
    fn pin_login_allows_localhost() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        std::env::set_var("EDGEQUAKE_DEV_PIN_LOGIN", "1");
        std::env::set_var(
            "DATABASE_URL",
            "postgres://edgequake:edgequake@localhost:5432/edgequake",
        );
        assert!(resolve_dev_pin_login().expect("localhost ok"));
        std::env::remove_var("EDGEQUAKE_DEV_PIN_LOGIN");
        std::env::remove_var("DATABASE_URL");
    }
}
