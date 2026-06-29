//! Login lockout enforcement (SPEC-027 SEC-011).
//!
//! Tracks failed password attempts on [`UserRecord`] and locks accounts when
//! [`edgequake_auth::AuthConfig::max_login_attempts`] is exceeded.

use chrono::{DateTime, Utc};

use edgequake_auth::AuthConfig;

use crate::error::ApiError;
use crate::handlers::auth::{persist_user_record, UserRecord};
use crate::state::{ApiSecurityConfig, PostgresRuntime, StorageRuntime};

/// Whether the account is currently locked.
pub(crate) fn is_account_locked(record: &UserRecord, now: DateTime<Utc>) -> bool {
    record.locked_until.is_some_and(|until| until > now)
}

/// Reject login when lockout is active (HTTP 423).
pub(crate) fn ensure_login_allowed(record: &UserRecord) -> Result<(), ApiError> {
    if is_account_locked(record, Utc::now()) {
        return Err(ApiError::account_locked());
    }
    Ok(())
}

/// Increment failed attempts; lock and return 423 when threshold reached.
pub(crate) async fn record_failed_login(
    storage: &StorageRuntime,
    pg_runtime: Option<&PostgresRuntime>,
    security: &ApiSecurityConfig,
    config: &AuthConfig,
    record: &mut UserRecord,
) -> Result<(), ApiError> {
    let now = Utc::now();
    if is_account_locked(record, now) {
        return Err(ApiError::account_locked());
    }

    record.failed_login_attempts += 1;
    let locked_now = record.failed_login_attempts >= config.max_login_attempts;
    if locked_now {
        let lock_secs = config.lockout_duration.as_secs();
        record.locked_until = Some(now + chrono::Duration::seconds(lock_secs as i64));
    }
    record.updated_at = now;
    persist_user_record(storage, pg_runtime, security, record).await?;

    if locked_now {
        return Err(ApiError::account_locked());
    }
    Ok(())
}

/// Clear lockout counters after successful authentication.
pub(crate) async fn record_successful_login(
    storage: &StorageRuntime,
    pg_runtime: Option<&PostgresRuntime>,
    security: &ApiSecurityConfig,
    record: &mut UserRecord,
) -> Result<(), ApiError> {
    let now = Utc::now();
    record.failed_login_attempts = 0;
    record.locked_until = None;
    record.last_login_at = Some(now);
    record.updated_at = now;
    persist_user_record(storage, pg_runtime, security, record).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record() -> UserRecord {
        UserRecord {
            user_id: "u1".into(),
            username: "alice".into(),
            email: "alice@example.com".into(),
            password_hash: "hash".into(),
            role: "user".into(),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login_at: None,
            failed_login_attempts: 0,
            locked_until: None,
            metadata: Default::default(),
        }
    }

    #[test]
    fn locked_until_in_future_blocks_login() {
        let mut record = sample_record();
        record.locked_until = Some(Utc::now() + chrono::Duration::minutes(5));
        assert!(is_account_locked(&record, Utc::now()));
        assert!(ensure_login_allowed(&record).is_err());
    }

    #[test]
    fn expired_lock_allows_login() {
        let mut record = sample_record();
        record.locked_until = Some(Utc::now() - chrono::Duration::minutes(1));
        assert!(!is_account_locked(&record, Utc::now()));
        assert!(ensure_login_allowed(&record).is_ok());
    }
}
