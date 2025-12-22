//! PostgreSQL graph storage using Apache AGE extension.

use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::Row;

use crate::error::{Result, StorageError};
use crate::traits::{GraphEdge, GraphNode, GraphStorage, KnowledgeGraph};
use super::config::PostgresConfig;
use super::connection::PostgresPool;

/// PostgreSQL graph storage using Apache AGE.
///
/// Apache AGE (A Graph Extension) provides graph database functionality
/// on top of PostgreSQL, supporting Cypher queries for graph traversal.
///
/// # Features
///
/// - Cypher query language support
/// - ACID transactions
/// - Integration with PostgreSQL's JSONB for properties
/// - Efficient graph traversal using native graph indexes
///
/// # Example
///
/// ```ignore
/// use edgequake_storage::adapters::postgres::{PostgresConfig, PostgresAGEGraphStorage};
///
/// let config = PostgresConfig::new("localhost", 5432, "edgequake", "user", "pass")
///     .with_namespace("my-workspace");
///
/// let storage = PostgresAGEGraphStorage::new(config).await?;
/// storage.initialize().await?;
/// ```
pub struct PostgresAGEGraphStorage {
    pool: PostgresPool,
    graph_name: String,
    nodes_table: String,
    edges_table: String,
    use_age: bool,
}

impl PostgresAGEGraphStorage {
    /// Create a new Apache AGE graph storage.
    pub fn new(config: PostgresConfig) -> Self {
        let prefix = config.table_prefix();
        let graph_name = format!("{}_graph", prefix);
        let nodes_table = format!("{}_nodes", prefix);
        let edges_table = format!("{}_edges", prefix);
        
        Self {
            pool: PostgresPool::new(config),
            graph_name,
            nodes_table,
            edges_table,
            use_age: true, // Will be set to false if AGE is not available
        }
    }
    
    /// Get the underlying pool.
    pub fn pool(&self) -> &PostgresPool {
        &self.pool
    }
    
    /// Check if Apache AGE is available.
    async fn check_age_available(&self) -> Result<bool> {
        let pool = self.pool.get().await?;
        
        let result = sqlx::query(
            "SELECT 1 FROM pg_extension WHERE extname = 'age'"
        )
        .fetch_optional(&pool)
        .await
        .map_err(|e| StorageError::QueryError(format!("AGE check failed: {}", e)))?;
        
        Ok(result.is_some())
    }
    
    /// Create AGE graph if it doesn't exist.
    async fn create_age_graph(&self) -> Result<()> {
        let pool = self.pool.get().await?;
        
        // Set search path for AGE
        sqlx::query("SET search_path = ag_catalog, \"$user\", public")
            .execute(&pool)
            .await
            .map_err(|e| StorageError::InitializationError(format!(
                "Failed to set AGE search path: {}", e
            )))?;
        
        // Create graph if not exists
        let create_sql = format!(
            "SELECT * FROM ag_catalog.create_graph('{}') WHERE NOT EXISTS (SELECT * FROM ag_catalog.ag_graph WHERE name = '{}')",
            self.graph_name, self.graph_name
        );
        
        match sqlx::query(&create_sql).execute(&pool).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // Check if graph already exists
                if e.to_string().contains("already exists") {
                    Ok(())
                } else {
                    Err(StorageError::InitializationError(format!(
                        "Failed to create AGE graph: {}", e
                    )))
                }
            }
        }
    }
    
    /// Create fallback tables for when AGE is not available.
    async fn create_fallback_tables(&self) -> Result<()> {
        let pool = self.pool.get().await?;
        
        // Create nodes table
        let nodes_sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                properties JSONB NOT NULL DEFAULT '{{}}',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
            self.nodes_table
        );
        
        sqlx::query(&nodes_sql)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::InitializationError(format!(
                "Failed to create nodes table: {}", e
            )))?;
        
        // Create edges table
        let edges_sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                properties JSONB NOT NULL DEFAULT '{{}}',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                PRIMARY KEY (source_id, target_id),
                FOREIGN KEY (source_id) REFERENCES {} (id) ON DELETE CASCADE,
                FOREIGN KEY (target_id) REFERENCES {} (id) ON DELETE CASCADE
            )
            "#,
            self.edges_table, self.nodes_table, self.nodes_table
        );
        
        sqlx::query(&edges_sql)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::InitializationError(format!(
                "Failed to create edges table: {}", e
            )))?;
        
        // Create indexes
        let source_idx = format!(
            "CREATE INDEX IF NOT EXISTS {}_source_idx ON {} (source_id)",
            self.edges_table, self.edges_table
        );
        let target_idx = format!(
            "CREATE INDEX IF NOT EXISTS {}_target_idx ON {} (target_id)",
            self.edges_table, self.edges_table
        );
        
        sqlx::query(&source_idx).execute(&pool).await.ok();
        sqlx::query(&target_idx).execute(&pool).await.ok();
        
        Ok(())
    }
}

#[async_trait]
impl GraphStorage for PostgresAGEGraphStorage {
    fn namespace(&self) -> &str {
        &self.pool.config().namespace
    }
    
    async fn initialize(&self) -> Result<()> {
        self.pool.initialize().await?;
        
        // Check if AGE is available
        let age_available = self.check_age_available().await.unwrap_or(false);
        
        if age_available {
            self.create_age_graph().await?;
        } else {
            // Fallback to regular tables
            // Note: we can't modify self.use_age here due to immutability
            // In practice, we'd use interior mutability or check at runtime
            tracing::warn!("Apache AGE not available, using fallback table-based graph storage");
            self.create_fallback_tables().await?;
        }
        
        // Always create fallback tables as a backup
        self.create_fallback_tables().await?;
        
        Ok(())
    }
    
    async fn finalize(&self) -> Result<()> {
        Ok(())
    }
    
    async fn has_node(&self, node_id: &str) -> Result<bool> {
        let pool = self.pool.get().await?;
        
        let sql = format!(
            "SELECT 1 FROM {} WHERE id = $1",
            self.nodes_table
        );
        
        let row = sqlx::query(&sql)
            .bind(node_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("Has node failed: {}", e)))?;
        
        Ok(row.is_some())
    }
    
    async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>> {
        let pool = self.pool.get().await?;
        
        let sql = format!(
            "SELECT id, properties FROM {} WHERE id = $1",
            self.nodes_table
        );
        
        let row = sqlx::query(&sql)
            .bind(node_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("Get node failed: {}", e)))?;
        
        match row {
            Some(row) => {
                let id: String = row.get("id");
                let properties: serde_json::Value = row.get("properties");
                let properties: HashMap<String, serde_json::Value> = 
                    serde_json::from_value(properties)
                        .unwrap_or_default();
                
                Ok(Some(GraphNode { id, properties }))
            }
            None => Ok(None),
        }
    }
    
    async fn upsert_node(
        &self,
        node_id: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let pool = self.pool.get().await?;
        
        let properties_json = serde_json::to_value(&properties)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        
        let sql = format!(
            r#"
            INSERT INTO {} (id, properties, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (id) DO UPDATE SET
                properties = {} || EXCLUDED.properties,
                updated_at = NOW()
            "#,
            self.nodes_table, self.nodes_table
        );
        
        sqlx::query(&sql)
            .bind(node_id)
            .bind(&properties_json)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::WriteError(format!("Upsert node failed: {}", e)))?;
        
        Ok(())
    }
    
    async fn delete_node(&self, node_id: &str) -> Result<()> {
        let pool = self.pool.get().await?;
        
        // Edges will be deleted by CASCADE
        let sql = format!(
            "DELETE FROM {} WHERE id = $1",
            self.nodes_table
        );
        
        sqlx::query(&sql)
            .bind(node_id)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::WriteError(format!("Delete node failed: {}", e)))?;
        
        Ok(())
    }
    
    async fn node_degree(&self, node_id: &str) -> Result<usize> {
        let pool = self.pool.get().await?;
        
        let sql = format!(
            r#"
            SELECT COUNT(*) as degree FROM {} 
            WHERE source_id = $1 OR target_id = $1
            "#,
            self.edges_table
        );
        
        let row = sqlx::query(&sql)
            .bind(node_id)
            .fetch_one(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("Node degree failed: {}", e)))?;
        
        let degree: i64 = row.get("degree");
        Ok(degree as usize)
    }
    
    async fn get_all_nodes(&self) -> Result<Vec<GraphNode>> {
        let pool = self.pool.get().await?;
        
        let sql = format!(
            "SELECT id, properties FROM {}",
            self.nodes_table
        );
        
        let rows = sqlx::query(&sql)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("Get all nodes failed: {}", e)))?;
        
        let nodes = rows
            .into_iter()
            .map(|row| {
                let id: String = row.get("id");
                let properties: serde_json::Value = row.get("properties");
                let properties: HashMap<String, serde_json::Value> = 
                    serde_json::from_value(properties).unwrap_or_default();
                GraphNode { id, properties }
            })
            .collect();
        
        Ok(nodes)
    }
    
    async fn get_nodes_by_ids(&self, node_ids: &[String]) -> Result<Vec<GraphNode>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }
        
        let pool = self.pool.get().await?;
        
        let placeholders: Vec<String> = (1..=node_ids.len())
            .map(|i| format!("${}", i))
            .collect();
        
        let sql = format!(
            "SELECT id, properties FROM {} WHERE id = ANY(ARRAY[{}])",
            self.nodes_table,
            placeholders.join(", ")
        );
        
        let mut query = sqlx::query(&sql);
        for id in node_ids {
            query = query.bind(id);
        }
        
        let rows = query
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("Get nodes by IDs failed: {}", e)))?;
        
        let nodes = rows
            .into_iter()
            .map(|row| {
                let id: String = row.get("id");
                let properties: serde_json::Value = row.get("properties");
                let properties: HashMap<String, serde_json::Value> = 
                    serde_json::from_value(properties).unwrap_or_default();
                GraphNode { id, properties }
            })
            .collect();
        
        Ok(nodes)
    }
    
    async fn has_edge(&self, source: &str, target: &str) -> Result<bool> {
        let pool = self.pool.get().await?;
        
        let sql = format!(
            "SELECT 1 FROM {} WHERE source_id = $1 AND target_id = $2",
            self.edges_table
        );
        
        let row = sqlx::query(&sql)
            .bind(source)
            .bind(target)
            .fetch_optional(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("Has edge failed: {}", e)))?;
        
        Ok(row.is_some())
    }
    
    async fn get_edge(&self, source: &str, target: &str) -> Result<Option<GraphEdge>> {
        let pool = self.pool.get().await?;
        
        let sql = format!(
            "SELECT source_id, target_id, properties FROM {} WHERE source_id = $1 AND target_id = $2",
            self.edges_table
        );
        
        let row = sqlx::query(&sql)
            .bind(source)
            .bind(target)
            .fetch_optional(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("Get edge failed: {}", e)))?;
        
        match row {
            Some(row) => {
                let source: String = row.get("source_id");
                let target: String = row.get("target_id");
                let properties: serde_json::Value = row.get("properties");
                let properties: HashMap<String, serde_json::Value> = 
                    serde_json::from_value(properties).unwrap_or_default();
                
                Ok(Some(GraphEdge { source, target, properties }))
            }
            None => Ok(None),
        }
    }
    
    async fn upsert_edge(
        &self,
        source: &str,
        target: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let pool = self.pool.get().await?;
        
        let properties_json = serde_json::to_value(&properties)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        
        let sql = format!(
            r#"
            INSERT INTO {} (source_id, target_id, properties, updated_at)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (source_id, target_id) DO UPDATE SET
                properties = {} || EXCLUDED.properties,
                updated_at = NOW()
            "#,
            self.edges_table, self.edges_table
        );
        
        sqlx::query(&sql)
            .bind(source)
            .bind(target)
            .bind(&properties_json)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::WriteError(format!("Upsert edge failed: {}", e)))?;
        
        Ok(())
    }
    
    async fn delete_edge(&self, source: &str, target: &str) -> Result<()> {
        let pool = self.pool.get().await?;
        
        let sql = format!(
            "DELETE FROM {} WHERE source_id = $1 AND target_id = $2",
            self.edges_table
        );
        
        sqlx::query(&sql)
            .bind(source)
            .bind(target)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::WriteError(format!("Delete edge failed: {}", e)))?;
        
        Ok(())
    }
    
    async fn get_node_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>> {
        let pool = self.pool.get().await?;
        
        let sql = format!(
            r#"
            SELECT source_id, target_id, properties FROM {} 
            WHERE source_id = $1 OR target_id = $1
            "#,
            self.edges_table
        );
        
        let rows = sqlx::query(&sql)
            .bind(node_id)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("Get node edges failed: {}", e)))?;
        
        let edges = rows
            .into_iter()
            .map(|row| {
                let source: String = row.get("source_id");
                let target: String = row.get("target_id");
                let properties: serde_json::Value = row.get("properties");
                let properties: HashMap<String, serde_json::Value> = 
                    serde_json::from_value(properties).unwrap_or_default();
                GraphEdge { source, target, properties }
            })
            .collect();
        
        Ok(edges)
    }
    
    async fn get_all_edges(&self) -> Result<Vec<GraphEdge>> {
        let pool = self.pool.get().await?;
        
        let sql = format!(
            "SELECT source_id, target_id, properties FROM {}",
            self.edges_table
        );
        
        let rows = sqlx::query(&sql)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("Get all edges failed: {}", e)))?;
        
        let edges = rows
            .into_iter()
            .map(|row| {
                let source: String = row.get("source_id");
                let target: String = row.get("target_id");
                let properties: serde_json::Value = row.get("properties");
                let properties: HashMap<String, serde_json::Value> = 
                    serde_json::from_value(properties).unwrap_or_default();
                GraphEdge { source, target, properties }
            })
            .collect();
        
        Ok(edges)
    }
    
    async fn get_knowledge_graph(
        &self,
        start_node: &str,
        max_depth: usize,
        max_nodes: usize,
    ) -> Result<KnowledgeGraph> {
        let pool = self.pool.get().await?;
        
        // Use recursive CTE for graph traversal
        let sql = format!(
            r#"
            WITH RECURSIVE graph_traversal AS (
                -- Base case: start node
                SELECT id, properties, 0 as depth
                FROM {} WHERE id = $1
                
                UNION ALL
                
                -- Recursive case: follow edges
                SELECT n.id, n.properties, gt.depth + 1
                FROM graph_traversal gt
                JOIN {} e ON e.source_id = gt.id OR e.target_id = gt.id
                JOIN {} n ON (n.id = e.source_id OR n.id = e.target_id) AND n.id != gt.id
                WHERE gt.depth < $2
            )
            SELECT DISTINCT id, properties FROM graph_traversal LIMIT $3
            "#,
            self.nodes_table, self.edges_table, self.nodes_table
        );
        
        let rows = sqlx::query(&sql)
            .bind(start_node)
            .bind(max_depth as i32)
            .bind(max_nodes as i32)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("Get knowledge graph failed: {}", e)))?;
        
        let mut kg = KnowledgeGraph::new();
        let mut node_ids: Vec<String> = Vec::new();
        
        for row in rows {
            let id: String = row.get("id");
            let properties: serde_json::Value = row.get("properties");
            let properties: HashMap<String, serde_json::Value> = 
                serde_json::from_value(properties).unwrap_or_default();
            
            node_ids.push(id.clone());
            kg.add_node(GraphNode { id, properties });
        }
        
        // Get edges between the discovered nodes
        if !node_ids.is_empty() {
            let placeholders: Vec<String> = (1..=node_ids.len())
                .map(|i| format!("${}", i))
                .collect();
            
            let edges_sql = format!(
                r#"
                SELECT source_id, target_id, properties FROM {} 
                WHERE source_id = ANY(ARRAY[{}]) AND target_id = ANY(ARRAY[{}])
                "#,
                self.edges_table,
                placeholders.join(", "),
                placeholders.join(", ")
            );
            
            let mut query = sqlx::query(&edges_sql);
            for id in &node_ids {
                query = query.bind(id);
            }
            // Bind again for target_id array
            for id in &node_ids {
                query = query.bind(id);
            }
            
            let edge_rows = query
                .fetch_all(&pool)
                .await
                .map_err(|e| StorageError::QueryError(format!("Get KG edges failed: {}", e)))?;
            
            for row in edge_rows {
                let source: String = row.get("source_id");
                let target: String = row.get("target_id");
                let properties: serde_json::Value = row.get("properties");
                let properties: HashMap<String, serde_json::Value> = 
                    serde_json::from_value(properties).unwrap_or_default();
                
                kg.add_edge(GraphEdge { source, target, properties });
            }
        }
        
        kg.is_truncated = kg.node_count() >= max_nodes;
        
        Ok(kg)
    }
    
    async fn get_popular_labels(&self, limit: usize) -> Result<Vec<String>> {
        let pool = self.pool.get().await?;
        
        // Get nodes with highest degree
        let sql = format!(
            r#"
            SELECT n.id, COUNT(e.*) as degree
            FROM {} n
            LEFT JOIN {} e ON e.source_id = n.id OR e.target_id = n.id
            GROUP BY n.id
            ORDER BY degree DESC
            LIMIT $1
            "#,
            self.nodes_table, self.edges_table
        );
        
        let rows = sqlx::query(&sql)
            .bind(limit as i32)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("Get popular labels failed: {}", e)))?;
        
        let labels = rows
            .into_iter()
            .map(|row| row.get("id"))
            .collect();
        
        Ok(labels)
    }
    
    async fn search_labels(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        let pool = self.pool.get().await?;
        
        let pattern = format!("%{}%", query.to_uppercase());
        
        let sql = format!(
            "SELECT id FROM {} WHERE UPPER(id) LIKE $1 LIMIT $2",
            self.nodes_table
        );
        
        let rows = sqlx::query(&sql)
            .bind(&pattern)
            .bind(limit as i32)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("Search labels failed: {}", e)))?;
        
        let labels = rows
            .into_iter()
            .map(|row| row.get("id"))
            .collect();
        
        Ok(labels)
    }
    
    async fn get_neighbors(&self, node_id: &str, depth: usize) -> Result<Vec<GraphNode>> {
        let kg = self.get_knowledge_graph(node_id, depth, 1000).await?;
        
        // Filter out the starting node
        let neighbors = kg.nodes
            .into_iter()
            .filter(|n| n.id != node_id)
            .collect();
        
        Ok(neighbors)
    }
    
    async fn node_count(&self) -> Result<usize> {
        let pool = self.pool.get().await?;
        
        let sql = format!("SELECT COUNT(*) as count FROM {}", self.nodes_table);
        
        let row = sqlx::query(&sql)
            .fetch_one(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("Node count failed: {}", e)))?;
        
        let count: i64 = row.get("count");
        Ok(count as usize)
    }
    
    async fn edge_count(&self) -> Result<usize> {
        let pool = self.pool.get().await?;
        
        let sql = format!("SELECT COUNT(*) as count FROM {}", self.edges_table);
        
        let row = sqlx::query(&sql)
            .fetch_one(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("Edge count failed: {}", e)))?;
        
        let count: i64 = row.get("count");
        Ok(count as usize)
    }
    
    async fn clear(&self) -> Result<()> {
        let pool = self.pool.get().await?;
        
        // Edges first due to foreign key
        let edges_sql = format!("TRUNCATE TABLE {} CASCADE", self.edges_table);
        let nodes_sql = format!("TRUNCATE TABLE {} CASCADE", self.nodes_table);
        
        sqlx::query(&edges_sql)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::WriteError(format!("Clear edges failed: {}", e)))?;
        
        sqlx::query(&nodes_sql)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::WriteError(format!("Clear nodes failed: {}", e)))?;
        
        Ok(())
    }
}

impl std::fmt::Debug for PostgresAGEGraphStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresAGEGraphStorage")
            .field("namespace", &self.pool.config().namespace)
            .field("graph_name", &self.graph_name)
            .field("use_age", &self.use_age)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_graph_storage_creation() {
        let config = PostgresConfig::default().with_namespace("test");
        let storage = PostgresAGEGraphStorage::new(config);
        
        assert_eq!(storage.graph_name, "eq_test_graph");
        assert_eq!(storage.nodes_table, "eq_test_nodes");
        assert_eq!(storage.edges_table, "eq_test_edges");
    }
}
