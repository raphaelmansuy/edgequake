//! PostgreSQL pool runtime — document list relational backfill (API-SOLID-I-001).

#[cfg(feature = "postgres")]
use edgequake_storage::adapters::postgres::PostgresCapabilities;
#[cfg(feature = "postgres")]
use sqlx::PgPool;

/// Optional PostgreSQL pool for relational read models.
#[cfg(feature = "postgres")]
#[derive(Clone)]
pub struct PostgresRuntime {
    pub pool: Option<PgPool>,
    pub capabilities: Option<PostgresCapabilities>,
}

#[cfg(not(feature = "postgres"))]
#[derive(Clone)]
pub struct PostgresRuntime;
