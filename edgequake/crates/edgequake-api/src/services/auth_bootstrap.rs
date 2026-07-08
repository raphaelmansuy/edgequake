//! First-run admin bootstrap when auth is enabled (GitHub #288 / SPEC-027 AC-4).
//!
//! ## First principles
//!
//! v0.15+ enables authentication by default. Login requires at least one user row in
//! PostgreSQL with a real password hash. Fresh installs and upgrades that only had KV
//! identity rows therefore return 401 until an admin exists.
//!
//! This module closes the gap by:
//! 1. Importing legacy KV `auth:user:*` records into PostgreSQL when present.
//! 2. Creating a bootstrap admin from `EDGEQUAKE_BOOTSTRAP_ADMIN_*` when no
//!    login-capable users remain.

use chrono::Utc;
use edgequake_auth::Role;
use tracing::{info, warn};
use uuid::Uuid;

use crate::handlers::auth::UserRecord;
use crate::state::{AppState, PostgresRuntime};

/// Returns true when the stored hash can authenticate a password login.
pub fn is_login_capable_password_hash(hash: &str) -> bool {
    let hash = hash.trim();
    if hash.is_empty() || hash == "anonymous" || hash == "not_a_real_hash" {
        return false;
    }
    hash.starts_with("$argon2") || hash.starts_with("$2")
}

/// Bootstrap identity when auth is required but no login-capable users exist.
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

        let pg_runtime = PostgresRuntime {
            pool: state.pg_pool.clone(),
            capabilities: state.postgres_capabilities.clone(),
        };

        let imported = import_legacy_kv_users(state, &pg_runtime).await?;
        if imported > 0 {
            info!(
                imported,
                "Imported legacy KV auth users into PostgreSQL (SPEC-027 upgrade path)"
            );
        }

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
                     For local quickstart, set EDGEQUAKE_DEV_MODE=true."
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
            if is_login_capable_password_hash(&existing.password_hash) {
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

            info!(
                user_id = %record.user_id,
                username = %username,
                "Upgraded existing user to login-capable bootstrap admin (GitHub #288)"
            );
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
            "Created bootstrap admin user (GitHub #288 — set EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD before first boot)"
        );

        Ok(())
    }
}

#[cfg(feature = "postgres")]
async fn import_legacy_kv_users(
    state: &AppState,
    pg_runtime: &PostgresRuntime,
) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
    let keys = state
        .storage
        .kv_storage
        .keys_with_prefix("auth:user:")
        .await
        .map_err(|e| format!("KV auth user scan failed: {e}"))?;

    if keys.is_empty() {
        return Ok(0);
    }

    let mut imported = 0u32;
    for key in keys {
        let Some(raw) = state
            .storage
            .kv_storage
            .get_by_id(&key)
            .await
            .map_err(|e| format!("KV auth user load failed for {key}: {e}"))?
        else {
            continue;
        };

        let record: UserRecord = match serde_json::from_value(raw) {
            Ok(record) => record,
            Err(e) => {
                warn!(key = %key, error = %e, "Skipping malformed legacy KV auth user");
                continue;
            }
        };

        if !is_login_capable_password_hash(&record.password_hash) {
            continue;
        }

        crate::services::identity_storage::persist_user_record(
            &state.storage,
            Some(pg_runtime),
            &state.security,
            &record,
        )
        .await?;

        imported += 1;
    }

    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
