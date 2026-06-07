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
            hnsw_ef_construction: 64,
            ivfflat_lists: 100,
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

    /// Get the table prefix for this namespace.
    ///
    /// # WHY: prefix is interpolated into DDL/table names (not a bind param)
    /// The namespace flows into `format!("eq_{prefix}_vectors")` style identifiers
    /// that cannot be parameterized. To close the identifier-injection surface
    /// (security S2) we map any character outside `[A-Za-z0-9_]` to `_`. Hyphens
    /// keep their historical mapping to `_` so existing deployments are unaffected.
    pub fn table_prefix(&self) -> String {
        let sanitized: String = self
            .namespace
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        format!("eq_{}", sanitized)
    }
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
    fn test_table_prefix() {
        let config = PostgresConfig::default().with_namespace("my-workspace");
        assert_eq!(config.table_prefix(), "eq_my_workspace");
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
