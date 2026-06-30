//! PostgreSQL graph storage using Apache AGE extension.
//!
//! This module implements graph storage using Apache AGE (A Graph Extension)
//! for PostgreSQL. AGE provides native graph database capabilities with
//! Cypher query language support.
//!
//! ## Implements
//!
//! - [`FEAT0203`]: Apache AGE graph storage
//! - [`FEAT0310`]: Cypher query language support
//! - [`FEAT0311`]: Variable-length path traversal
//! - [`FEAT0312`]: Multi-tenant graph isolation
//!
//! ## Use Cases
//!
//! - [`UC0602`]: System stores entities and relationships
//! - [`UC0701`]: System traverses knowledge graph
//! - [`UC0702`]: System finds entity relationships
//!
//! ## Enforces
//!
//! - [`BR0203`]: ACID transactions for graph operations
//! - [`BR0310`]: Namespace isolation per tenant
//! - [`BR0311`]: Lazy index creation after first insert
//!
//! # Features
//!
//! - Native Cypher query language support
//! - Variable-length path traversal
//! - ACID transactions with graph operations
//! - Native graph storage (vertices and edges)
//! - Efficient graph-optimized indexes
//!
//! # Requirements
//!
//! - PostgreSQL 11-17
//! - Apache AGE extension installed and loaded
//!
//! # Example
//!
//! ```ignore
//! use edgequake_storage::adapters::postgres::{PostgresConfig, PostgresAGEGraphStorage};
//!
//! let config = PostgresConfig::new("localhost", 5432, "edgequake", "user", "pass")
//!     .with_namespace("my-workspace");
//!
//! let storage = PostgresAGEGraphStorage::new(config);
//! storage.initialize().await?;
//! ```

use std::sync::atomic::AtomicBool;

use super::config::PostgresConfig;
use super::connection::PostgresPool;
use crate::error::{Result, StorageError};

/// PostgreSQL graph storage using Apache AGE.
///
/// # WHY: Apache AGE for Graph Storage
///
/// We chose Apache AGE (A Graph Extension) over alternatives:
///
/// 1. **Native PostgreSQL Integration**
///    - WHY: Leverages PostgreSQL's ACID guarantees, replication, and ecosystem
///    - WHY: No separate graph database to manage; uses existing Postgres infra
///
/// 2. **Cypher Query Language**
///    - WHY: Industry-standard graph query language (Neo4j compatible)
///    - WHY: Rich traversal syntax (variable-length paths, pattern matching)
///
/// 3. **Performance Optimizations**
///    - WHY indexes_verified: AGE creates label tables lazily; indexes created on first use
///    - WHY SQL fallback for degree: Cypher OPTIONAL MATCH is 10x slower than native SQL
///    - WHY graphid::text: AGE's graphid type lacks native equality operator
///
/// 4. **Multi-Tenancy via Namespace**
///    - WHY graph_name includes prefix: Each tenant gets isolated graph
///    - WHY namespace tracking: Enables per-tenant vector filtering
///
/// Uses the AGE extension for native graph operations with Cypher queries.
/// All operations use AGE's graph-optimized storage and query engine.
mod helpers;

pub struct PostgresAGEGraphStorage {
    pool: PostgresPool,
    graph_name: String,
    namespace: String,
    prefix: String,
    initialized: AtomicBool,
    /// Track if indexes have been created after first node insertion.
    /// AGE creates label tables lazily on first use, so indexes must be
    /// created after the first node/edge is inserted.
    indexes_verified: AtomicBool,
}

impl PostgresAGEGraphStorage {
    /// Create a new Apache AGE graph storage.
    pub fn new(config: PostgresConfig) -> Self {
        Self::with_pool(PostgresPool::new(config.clone()), config)
    }

    /// Create graph storage using a shared connection pool (SPEC-011).
    pub fn with_pool(pool: PostgresPool, config: PostgresConfig) -> Self {
        let prefix = config.table_prefix();
        let graph_name = format!("eq_{}_graph", prefix);
        let namespace = config.namespace.clone();

        Self {
            pool,
            graph_name,
            namespace,
            prefix,
            initialized: AtomicBool::new(false),
            indexes_verified: AtomicBool::new(false),
        }
    }

    /// Get the underlying pool.
    pub fn pool(&self) -> &PostgresPool {
        &self.pool
    }

    /// Get the graph name.
    pub fn graph_name(&self) -> &str {
        &self.graph_name
    }

    /// Read an O(1) planner estimate of `(graph_name).<label>` row count.
    ///
    /// Returns 0 if the table is missing or the catalog row is unavailable —
    /// callers (the `*_fast` variants) treat that as "no estimate", which is
    /// the correct behaviour for an empty / uninitialised graph. If you need
    /// an exact value, call [`node_count`] / [`edge_count`] instead.
    ///
    /// SPEC-011 iter 02 Fix B / SPEC-012 Fix B'.
    ///
    /// WHY pg_inherits SUM: Apache AGE stores graph rows in **child label tables**
    /// (e.g. `Node`, `EDGE`) that inherit from the abstract parent tables
    /// `_ag_label_vertex` / `_ag_label_edge`. The parent tables hold 0 rows
    /// themselves, so querying `pg_class.reltuples` directly for the parent gives
    /// 0 or -1 (never analysed). The correct O(1) estimate is the SUM of
    /// `reltuples` across all child tables registered in `pg_inherits`.
    async fn reltuples_estimate(&self, label: &str) -> Result<usize> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;
        // Sum reltuples of all child tables that inherit from the parent label.
        // Falls back to 0 if no children exist or parent is not found.
        let row: Option<(f64,)> = sqlx::query_as(
            "SELECT COALESCE(SUM(c.reltuples), 0)::float8
             FROM pg_class c
             JOIN pg_inherits i ON i.inhrelid = c.oid
             WHERE i.inhparent = (
                 SELECT pc.oid
                 FROM pg_class pc
                 JOIN pg_namespace pn ON pn.oid = pc.relnamespace
                 WHERE pn.nspname = $1 AND pc.relname = $2
             )",
        )
        .bind(&self.graph_name)
        .bind(label)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("reltuples estimate failed: {}", e)))?;

        Ok(row
            .map(|(t,)| if t < 0.0 { 0 } else { t as usize })
            .unwrap_or(0))
    }
}

mod analytics_ops;
mod edges_ops;
mod graph_storage_impl;
mod lifecycle_ops;
mod nodes_ops;
mod query_ops;
mod scan_ops;

/// SPEC-034 IMP-01: Check whether the native SQL write path is enabled.
///
/// # WHY: Feature flag for safe rollout
///
/// The native SQL path replaces Cypher MERGE with INSERT ON CONFLICT DO UPDATE,
/// changing write complexity from O(G) (GIN scan) to O(log G) (btree) — ~69×
/// faster at 50K nodes. It is gated behind an environment variable so that:
///   1. The old Cypher path is preserved as a fallback during validation.
///   2. CI and offline tests continue to work without the Migration 067 helpers.
///   3. Production rollout can be controlled per-deployment without code changes.
///
/// Set `EDGEQUAKE_NATIVE_GRAPH_WRITES=1` (or `true`) to enable.
/// Default: `false` — the Cypher UNWIND MERGE path is used.
pub(super) fn native_graph_writes_enabled() -> bool {
    std::env::var("EDGEQUAKE_NATIVE_GRAPH_WRITES")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

impl std::fmt::Debug for PostgresAGEGraphStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresAGEGraphStorage")
            .field("namespace", &self.namespace)
            .field("graph_name", &self.graph_name)
            .field("prefix", &self.prefix)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_graph_storage_creation() {
        let config = PostgresConfig::default().with_namespace("test");
        let storage = PostgresAGEGraphStorage::new(config);

        // Graph name includes eq_ prefix from table_prefix() which returns "eq_test"
        // Then format!("eq_{}_graph", prefix) creates "eq_eq_test_graph"
        assert_eq!(storage.graph_name, "eq_eq_test_graph");
        assert_eq!(storage.namespace, "test");
    }

    #[test]
    fn test_escape_cypher_string() {
        assert_eq!(
            PostgresAGEGraphStorage::escape_cypher_string("hello'world"),
            "hello\\'world"
        );
        assert_eq!(
            PostgresAGEGraphStorage::escape_cypher_string("line\nnew"),
            "line\\nnew"
        );
    }

    #[test]
    fn test_properties_to_cypher() {
        let mut props = HashMap::new();
        props.insert("name".to_string(), serde_json::json!("Alice"));
        props.insert("age".to_string(), serde_json::json!(30));
        props.insert("active".to_string(), serde_json::json!(true));

        let cypher = PostgresAGEGraphStorage::properties_to_cypher(&props);

        // Properties order is not guaranteed, so just check for presence
        assert!(cypher.starts_with('{'));
        assert!(cypher.ends_with('}'));
        assert!(cypher.contains("name: 'Alice'"));
        assert!(cypher.contains("age: 30"));
        assert!(cypher.contains("active: true"));
    }

    #[test]
    fn test_parse_vertex() {
        let agtype = r#"{"id": 123, "label": "Node", "properties": {"node_id": "test-1", "name": "Test Node"}}"#;

        let node = PostgresAGEGraphStorage::parse_vertex(agtype);
        assert!(node.is_some());

        let node = node.unwrap();
        assert_eq!(node.id, "test-1");
        assert_eq!(node.properties.get("name").unwrap(), "Test Node");
    }

    #[test]
    fn test_parse_edge() {
        let agtype = r#"{"id": 456, "label": "EDGE", "start_id": 123, "end_id": 789, "properties": {"source_id": "node-1", "target_id": "node-2", "weight": 0.5}}"#;

        let edge = PostgresAGEGraphStorage::parse_edge(agtype);
        assert!(edge.is_some());

        let edge = edge.unwrap();
        assert_eq!(edge.source, "node-1");
        assert_eq!(edge.target, "node-2");
        assert_eq!(edge.properties.get("weight").unwrap(), 0.5);
    }

    // -------------------------------------------------------------------------
    // SPEC-034 IMP-01: Feature flag tests
    // -------------------------------------------------------------------------

    /// Test: feature flag defaults to disabled when env var is unset.
    #[test]
    fn test_native_graph_writes_disabled_by_default() {
        // Remove env var to simulate clean environment.
        // NOTE: env vars are process-global; use temp scope to avoid test pollution.
        let original = std::env::var("EDGEQUAKE_NATIVE_GRAPH_WRITES").ok();
        std::env::remove_var("EDGEQUAKE_NATIVE_GRAPH_WRITES");

        let result = native_graph_writes_enabled();

        // Restore
        if let Some(v) = original {
            std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", v);
        }

        assert!(
            !result,
            "native_graph_writes_enabled() must be false when env var is unset"
        );
    }

    /// Test: feature flag enabled when set to "1".
    #[test]
    fn test_native_graph_writes_enabled_with_one() {
        let original = std::env::var("EDGEQUAKE_NATIVE_GRAPH_WRITES").ok();
        std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", "1");

        let result = native_graph_writes_enabled();

        // Restore
        match original {
            Some(v) => std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", v),
            None => std::env::remove_var("EDGEQUAKE_NATIVE_GRAPH_WRITES"),
        }

        assert!(
            result,
            "native_graph_writes_enabled() must be true when EDGEQUAKE_NATIVE_GRAPH_WRITES=1"
        );
    }

    /// Test: feature flag enabled when set to "true" (case-insensitive).
    #[test]
    fn test_native_graph_writes_enabled_with_true() {
        let original = std::env::var("EDGEQUAKE_NATIVE_GRAPH_WRITES").ok();
        std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", "true");

        let result = native_graph_writes_enabled();

        match original {
            Some(v) => std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", v),
            None => std::env::remove_var("EDGEQUAKE_NATIVE_GRAPH_WRITES"),
        }

        assert!(
            result,
            "native_graph_writes_enabled() must be true when EDGEQUAKE_NATIVE_GRAPH_WRITES=true"
        );
    }

    /// Test: feature flag disabled for any other value (e.g. "0", "false").
    #[test]
    fn test_native_graph_writes_disabled_with_zero() {
        let original = std::env::var("EDGEQUAKE_NATIVE_GRAPH_WRITES").ok();
        std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", "0");

        let result = native_graph_writes_enabled();

        match original {
            Some(v) => std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", v),
            None => std::env::remove_var("EDGEQUAKE_NATIVE_GRAPH_WRITES"),
        }

        assert!(
            !result,
            "native_graph_writes_enabled() must be false when EDGEQUAKE_NATIVE_GRAPH_WRITES=0"
        );
    }
}
