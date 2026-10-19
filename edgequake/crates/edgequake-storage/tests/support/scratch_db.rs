//! Provisioning lock and rebuild for disposable `*_test` scratch databases.
//!
//! Shared by the storage harness (`postgres_test_config.rs`) and the API
//! harness (`edgequake-api/tests/common/test_db.rs`). Depends on sqlx only.
//!
//! Why rebuild instead of repair. A scratch database holds nothing worth
//! keeping, and a stale one can be impossible to migrate: `edgequake_test`
//! predates the plain graph and still carries Apache AGE graphs, so migration
//! 171 (drop AGE) correctly refuses (`cutover guard is red`). The harness used
//! to print that and carry on, which left every later test running against a
//! pre-171 schema. Recreating the database is the only fix that does not
//! weaken the guard.
//!
//! Two safeguards keep a rebuild from doing damage:
//! * only names ending in `_test` are ever dropped ([`is_disposable`]);
//! * provisioning holds a cluster-wide advisory lock, so concurrent test
//!   processes provision one at a time and never drop a database another
//!   process is still migrating.
#![allow(dead_code)]

use sqlx::postgres::PgConnectOptions;
use sqlx::{ConnectOptions, Connection, PgConnection};

/// Only `*_test` databases may be dropped. The dev database is never one.
pub fn is_disposable(database: &str) -> bool {
    database.len() > "_test".len() && database.ends_with("_test")
}

/// Quote an identifier for `CREATE`/`DROP DATABASE`, which cannot be bound.
fn quoted(database: &str) -> String {
    format!("\"{}\"", database.replace('"', "\"\""))
}

/// A session on the `postgres` maintenance database holding the provisioning
/// lock for one scratch database. The lock is released when this is dropped.
pub struct ScratchProvisioner {
    admin: PgConnection,
    database: String,
}

impl ScratchProvisioner {
    /// Connect to the maintenance database and wait for the provisioning lock.
    /// `None` when the server is unreachable (the caller keeps its soft-skip).
    pub async fn acquire(options: PgConnectOptions, database: &str) -> Option<Self> {
        let mut admin = options.database("postgres").connect().await.ok()?;
        sqlx::query("SELECT pg_advisory_lock(hashtext($1))")
            .bind(format!("edgequake:scratch-db:{database}"))
            .execute(&mut admin)
            .await
            .ok()?;
        Some(Self {
            admin,
            database: database.to_string(),
        })
    }

    /// Create the database when it is missing.
    pub async fn ensure_exists(&mut self) {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)")
                .bind(&self.database)
                .fetch_one(&mut self.admin)
                .await
                .unwrap_or(false);
        if !exists {
            let _ = self.create().await;
        }
    }

    /// Drop and recreate the database. Refuses a name that is not disposable.
    /// Returns whether a fresh database is ready.
    pub async fn rebuild(&mut self) -> bool {
        if !is_disposable(&self.database) {
            eprintln!(
                "test-db: not rebuilding `{}` (only `*_test` databases are disposable)",
                self.database
            );
            return false;
        }
        eprintln!(
            "test-db: `{}` cannot be migrated; recreating the scratch database",
            self.database
        );
        let dropped = sqlx::query(&format!(
            "DROP DATABASE IF EXISTS {} WITH (FORCE)",
            quoted(&self.database)
        ))
        .execute(&mut self.admin)
        .await;
        if let Err(e) = dropped {
            eprintln!("test-db: drop failed: {e}");
            return false;
        }
        self.create().await
    }

    async fn create(&mut self) -> bool {
        sqlx::query(&format!("CREATE DATABASE {}", quoted(&self.database)))
            .execute(&mut self.admin)
            .await
            .is_ok()
    }

    /// Release the lock and close the maintenance session.
    pub async fn finish(self) {
        let _ = self.admin.close().await;
    }
}
