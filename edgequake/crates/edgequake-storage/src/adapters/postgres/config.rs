//! PostgreSQL configuration.
//!
//! Provides configuration for PostgreSQL connections, pooling, and extensions.
//!
//! ## Implements
//!
//! - [`FEAT0243`]: Connection pool configuration
//! - [`FEAT0244`]: SSL mode configuration
//! - [`FEAT0245`]: Vector index type selection
//!
//! ## Use Cases
//!
//! - [`UC0901`]: System configures database connection
//!
//! ## Enforces
//!
//! - [`BR0243`]: Connection pool limits
//! - [`BR0244`]: Timeout configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// PostgreSQL connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    /// Database host.
    pub host: String,

    /// Database port.
    pub port: u16,

    /// Database name.
    pub database: String,

    /// Username.
    pub user: String,

    /// Password.
    pub password: String,

    /// Namespace/schema for this instance.
    pub namespace: String,

    /// Maximum number of connections in the pool.
    pub max_connections: u32,

    /// Minimum number of connections in the pool.
    pub min_connections: u32,

    /// Connection timeout.
    pub connect_timeout: Duration,

    /// Idle connection timeout.
    pub idle_timeout: Duration,

    /// SSL mode.
    pub ssl_mode: SslMode,

    /// Vector index type for pgvector.
    pub vector_index_type: VectorIndexType,

    /// HNSW M parameter (for HNSW index).
    pub hnsw_m: u32,

    /// HNSW ef_construction parameter.
    pub hnsw_ef_construction: u32,

    /// IVFFlat lists parameter.
    pub ivfflat_lists: u32,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            database: "edgequake".to_string(),
            user: "postgres".to_string(),
            password: String::new(),
            namespace: "default".to_string(),
            // QW5 (F11): default pool sized to >= 2x the pipeline's default
            // max_concurrent_extractions (16). WHY: each in-flight document can
            // hold a connection for the vector upsert AND another for graph
            // MERGEs; a pool smaller than peak concurrent demand serializes the
            // pipeline behind connection acquisition (head-of-line blocking).
            // First principle: pool_size must exceed peak simultaneous
            // connection holders, not average. Operators must ensure the
            // PostgreSQL server `max_connections` >= sum of all app pools.
            max_connections: 32,
            min_connections: 1,
            connect_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            ssl_mode: SslMode::Prefer,
            vector_index_type: VectorIndexType::HNSW,
            hnsw_m: 16,
            // Default 128 (SPEC-090 F-090-24/25). Override via EDGEQUAKE_HNSW_EF_CONSTRUCTION.
            // Never REINDEX on boot — operator-driven only.
            // SPEC-091 IW1 LD-06 (GAP-091-25): 128 is the single converged value for new builds.
            // Migration 071 (ef=32) is checksum-locked historical; init.sql matches 128.
            hnsw_ef_construction: hnsw_ef_construction_from_env(),
            ivfflat_lists: 100,
        }
    }
}

/// HNSW `ef_construction` from env (default **128**, SPEC-090 F-090-24/25).
///
/// Changing this only affects **new** index builds — existing HNSW requires
/// operator `REINDEX CONCURRENTLY`.
///
/// SPEC-091 IW1 LD-06 (GAP-091-25): **128 is the converged SSOT for new builds.**
/// Migration 071 historically set ef_construction=32 (checksum-locked, do not
/// edit). `docker/init.sql` and typed-index migrations (129+) use 128. Existing
/// legacy HNSW built at 32 requires operator `REINDEX CONCURRENTLY` to converge.
pub fn hnsw_ef_construction_from_env() -> u32 {
    std::env::var("EDGEQUAKE_HNSW_EF_CONSTRUCTION")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .map(|v| v.clamp(4, 1000))
        .unwrap_or(128)
}

/// Optional single-process role label (legacy). Prefer [`crate::PgPoolBundle`].
///
/// `EDGEQUAKE_DB_POOL_ROLE=query|ingest|queue|admin` — when set with a matching
/// `EDGEQUAKE_DB_POOL_SIZE_{ROLE}` override, adjusts `max_connections` for
/// a single-pool process. In-server multi-pool uses `PgPoolBundle` instead.
pub fn db_pool_role_from_env() -> Option<String> {
    std::env::var("EDGEQUAKE_DB_POOL_ROLE")
        .ok()
        .map(|r| r.to_ascii_lowercase())
        .filter(|r| matches!(r.as_str(), "query" | "ingest" | "queue" | "admin"))
}

/// Resolve pool size: role-specific override, else configured default.
pub fn resolve_pool_max_connections(default: u32) -> u32 {
    let override_env = db_pool_role_from_env().and_then(|role| {
        let key = match role.as_str() {
            "query" => "EDGEQUAKE_DB_POOL_SIZE_QUERY",
            "ingest" => "EDGEQUAKE_DB_POOL_SIZE_INGEST",
            "queue" => "EDGEQUAKE_DB_POOL_SIZE_QUEUE",
            "admin" => "EDGEQUAKE_DB_POOL_SIZE_ADMIN",
            _ => return None,
        };
        std::env::var(key).ok()
    });
    override_env
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod hnsw_ef_construction_tests {
    use super::hnsw_ef_construction_from_env;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn default_is_128_when_unset() {
        let _g = env_lock().lock().unwrap();
        let prev = std::env::var("EDGEQUAKE_HNSW_EF_CONSTRUCTION").ok();
        std::env::remove_var("EDGEQUAKE_HNSW_EF_CONSTRUCTION");
        assert_eq!(hnsw_ef_construction_from_env(), 128);
        match prev {
            Some(v) => std::env::set_var("EDGEQUAKE_HNSW_EF_CONSTRUCTION", v),
            None => std::env::remove_var("EDGEQUAKE_HNSW_EF_CONSTRUCTION"),
        }
    }

    #[test]
    fn production_profile_128() {
        let _g = env_lock().lock().unwrap();
        let prev = std::env::var("EDGEQUAKE_HNSW_EF_CONSTRUCTION").ok();
        std::env::set_var("EDGEQUAKE_HNSW_EF_CONSTRUCTION", "128");
        assert_eq!(hnsw_ef_construction_from_env(), 128);
        match prev {
            Some(v) => std::env::set_var("EDGEQUAKE_HNSW_EF_CONSTRUCTION", v),
            None => std::env::remove_var("EDGEQUAKE_HNSW_EF_CONSTRUCTION"),
        }
    }
}

impl PostgresConfig {
    /// Create a new configuration with the given connection string parts.
    pub fn new(
        host: impl Into<String>,
        port: u16,
        database: impl Into<String>,
        user: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            database: database.into(),
            user: user.into(),
            password: password.into(),
            ..Default::default()
        }
    }

    /// Set the namespace.
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Set max connections.
    pub fn with_max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    /// Set vector index type.
    pub fn with_vector_index(mut self, index_type: VectorIndexType) -> Self {
        self.vector_index_type = index_type;
        self
    }

    /// Build a connection URL.
    pub fn connection_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user, self.password, self.host, self.port, self.database
        )
    }

    /// Preserve literal credentials and enforce the configured TLS policy.
    /// URL interpolation can misparse passwords containing `@`, `/`, or `?`.
    pub fn connect_options(&self) -> sqlx::postgres::PgConnectOptions {
        use sqlx::postgres::{PgConnectOptions, PgSslMode};

        let ssl_mode = match self.ssl_mode {
            SslMode::Disable => PgSslMode::Disable,
            SslMode::Allow => PgSslMode::Allow,
            SslMode::Prefer => PgSslMode::Prefer,
            SslMode::Require => PgSslMode::Require,
            SslMode::VerifyCa => PgSslMode::VerifyCa,
            SslMode::VerifyFull => PgSslMode::VerifyFull,
        };
        PgConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .database(&self.database)
            .username(&self.user)
            .password(&self.password)
            .ssl_mode(ssl_mode)
    }

    /// Get the table prefix for this namespace.
    ///
    /// # WHY: prefix is interpolated into DDL/table names (not a bind param)
    /// The namespace flows into `format!("eq_{prefix}_vectors")` style identifiers
    /// that cannot be parameterized. To close the identifier-injection surface
    /// (security S2) we map any character outside `[A-Za-z0-9_]` to `_`. Hyphens
    /// keep their historical mapping to `_` so existing deployments are unaffected.
    pub fn table_prefix(&self) -> String {
        crate::namespace_tables::table_prefix_for_namespace(&self.namespace)
    }

    /// AGE graph catalog name (SPEC-104 LAW-I1 SSOT).
    ///
    /// `namespace = "default"` → prefix `eq_default` → `eq_eq_default_graph`.
    pub fn age_graph_name(&self) -> String {
        crate::namespace_tables::age_graph_name_for_namespace(&self.namespace)
    }

    /// Unqualified KV table name (`eq_{prefix}_kv`) — SPEC-104 LAW-I1.
    pub fn bare_kv_table(&self) -> String {
        crate::namespace_tables::bare_kv_table_for_namespace(&self.namespace)
    }

    /// Unqualified vectors table name (`eq_{prefix}_vectors`) — SPEC-104 LAW-I1.
    pub fn bare_vectors_table(&self) -> String {
        crate::namespace_tables::bare_vectors_table_for_namespace(&self.namespace)
    }

    /// Qualified KV table for this namespace (`public.eq_{prefix}_kv`).
    pub fn qualified_kv_table(&self) -> String {
        qualified_kv_table_name(&self.table_prefix())
    }
}

/// Split `schema.table` or return `(public, table)`.
pub(crate) fn split_qualified_table_name(qualified_name: &str) -> (&str, &str) {
    qualified_name
        .split_once('.')
        .map_or(("public", qualified_name), |(schema, table)| {
            (schema, table)
        })
}

/// Qualified KV table (`public.eq_{prefix}_kv`).
pub fn qualified_kv_table_name(prefix: &str) -> String {
    format!("public.eq_{prefix}_kv")
}

/// Qualified vectors table (`public.eq_{prefix}_vectors`).
pub(crate) fn qualified_vectors_table_name(prefix: &str) -> String {
    format!("public.eq_{prefix}_vectors")
}

/// Qualified vectors stats table (`public.eq_{prefix}_vectors_stats`).
pub(crate) fn qualified_vectors_stats_table_name(prefix: &str) -> String {
    format!("public.eq_{prefix}_vectors_stats")
}

/// SSL connection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum SslMode {
    /// Disable SSL.
    Disable,
    /// Allow SSL if available.
    Allow,
    /// Prefer SSL.
    #[default]
    Prefer,
    /// Require SSL.
    Require,
    /// Require SSL and verify CA.
    VerifyCa,
    /// Require SSL and verify full chain.
    VerifyFull,
}

/// Vector index type for pgvector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum VectorIndexType {
    /// No index (brute force).
    None,
    /// IVFFlat index.
    IVFFlat,
    /// HNSW index (Hierarchical Navigable Small World).
    #[default]
    #[allow(clippy::upper_case_acronyms)]
    HNSW,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = PostgresConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        // QW5: default pool raised to 32 (>= 2x default concurrent extractions).
        assert_eq!(config.max_connections, 32);
    }

    #[test]
    fn test_connection_url() {
        let config = PostgresConfig::new("db.example.com", 5432, "mydb", "user", "pass123");
        assert_eq!(
            config.connection_url(),
            "postgres://user:pass123@db.example.com:5432/mydb"
        );
    }

    #[test]
    fn typed_options_preserve_literal_connection_fields() {
        let config = PostgresConfig::new("::1", 5433, "db/name?", "user@host", "p@ss:/?#%");
        let options = config.connect_options();
        assert_eq!(options.get_host(), "::1");
        assert_eq!(options.get_port(), 5433);
        assert_eq!(options.get_database(), Some("db/name?"));
        assert_eq!(options.get_username(), "user@host");
    }

    #[test]
    fn typed_options_honor_every_ssl_mode() {
        use sqlx::postgres::PgSslMode;
        for (mode, expected) in [
            (SslMode::Disable, PgSslMode::Disable),
            (SslMode::Allow, PgSslMode::Allow),
            (SslMode::Prefer, PgSslMode::Prefer),
            (SslMode::Require, PgSslMode::Require),
            (SslMode::VerifyCa, PgSslMode::VerifyCa),
            (SslMode::VerifyFull, PgSslMode::VerifyFull),
        ] {
            let config = PostgresConfig {
                ssl_mode: mode,
                ..Default::default()
            };
            assert_eq!(
                std::mem::discriminant(&config.connect_options().get_ssl_mode()),
                std::mem::discriminant(&expected)
            );
        }
    }

    #[test]
    fn test_table_prefix() {
        let config = PostgresConfig::default().with_namespace("my-workspace");
        assert_eq!(config.table_prefix(), "eq_my_workspace");
    }

    #[test]
    fn e2e_104_06_age_graph_and_bare_relation_names() {
        let default = PostgresConfig::default();
        assert_eq!(default.age_graph_name(), "eq_eq_default_graph");
        assert_eq!(default.bare_kv_table(), "eq_eq_default_kv");
        assert_eq!(default.bare_vectors_table(), "eq_eq_default_vectors");

        let ws = PostgresConfig::default().with_namespace("my-ws");
        assert_eq!(ws.table_prefix(), "eq_my_ws");
        assert_eq!(ws.age_graph_name(), "eq_eq_my_ws_graph");
        assert_eq!(ws.bare_kv_table(), "eq_eq_my_ws_kv");
        assert_eq!(ws.bare_vectors_table(), "eq_eq_my_ws_vectors");
    }

    #[test]
    fn test_table_prefix_sanitizes_injection_chars() {
        // Security S2: identifier-injection attempt must be neutralized.
        let config = PostgresConfig::default().with_namespace("a\"; DROP TABLE x;--");
        let prefix = config.table_prefix();
        assert!(!prefix.contains('"'));
        assert!(!prefix.contains(';'));
        assert!(!prefix.contains(' '));
        assert!(prefix.starts_with("eq_"));
        // Only [A-Za-z0-9_] survive.
        assert!(prefix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_'));
    }
}
