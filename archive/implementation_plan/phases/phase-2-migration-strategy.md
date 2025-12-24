# Phase 2: Migration Strategy

**Phase Duration**: Weeks 3-4  
**Owner**: Senior Backend Engineer  
**Status**: 🔴 Not Started

---

## Objective

Implement the storage adapter framework with concrete database implementations for PostgreSQL (AGE + pgvector) and SurrealDB, establishing the async patterns and error handling that will be used throughout EdgeQuake.

---

## Reference Documentation

| Document | Purpose |
|----------|---------|
| [docs_retro/06-storage-contracts.md](../../docs_retro/06-storage-contracts.md) | Storage interface specifications |
| [docs_retro/09-security-errors.md](../../docs_retro/09-security-errors.md) | Error handling patterns |
| [tech_stack/postgresql-age-pgvector.md](../../tech_stack/postgresql-age-pgvector.md) | PostgreSQL setup |
| [tech_stack/surrealdb.md](../../tech_stack/surrealdb.md) | SurrealDB setup |
| [tech_stack/technology_choice.md](../../tech_stack/technology_choice.md) | Database rationale |
| [plan/integration/MIGRATION_GUIDE.md](../../plan/integration/MIGRATION_GUIDE.md) | Python→Rust patterns |

---

## Deliverables

### 1. Error Handling Framework

```rust
// edgequake-core/src/error.rs
use thiserror::Error;

/// Base error type for EdgeQuake
#[derive(Error, Debug)]
pub enum EdgeQuakeError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    
    #[error("LLM error: {0}")]
    LLM(#[from] LLMError),
    
    #[error("Pipeline error: {0}")]
    Pipeline(#[from] PipelineError),
    
    #[error("Query error: {0}")]
    Query(#[from] QueryError),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
}

/// Storage-specific errors
/// Reference: docs_retro/09-security-errors.md
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Record not found: {table}.{id}")]
    NotFound { table: String, id: String },
    
    #[error("Duplicate key: {table}.{id}")]
    DuplicateKey { table: String, id: String },
    
    #[error("Schema error: {0}")]
    SchemaError(String),
    
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    
    #[error("Query execution failed: {0}")]
    QueryFailed(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Storage is full")]
    StorageFull,
    
    #[error("Operation timed out")]
    Timeout,
    
    #[error("Storage not initialized")]
    NotInitialized,
}

/// LLM-specific errors
#[derive(Error, Debug)]
pub enum LLMError {
    #[error("API connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Rate limited, retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },
    
    #[error("Token limit exceeded: {used} > {limit}")]
    TokenLimitExceeded { used: usize, limit: usize },
    
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    
    #[error("Authentication failed")]
    AuthenticationFailed,
    
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    
    #[error("Request timeout")]
    Timeout,
}

/// Pipeline-specific errors
#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("Chunking failed: {0}")]
    ChunkingFailed(String),
    
    #[error("Entity extraction failed: {0}")]
    ExtractionFailed(String),
    
    #[error("Embedding generation failed: {0}")]
    EmbeddingFailed(String),
    
    #[error("Merge conflict: {0}")]
    MergeConflict(String),
    
    #[error("Document already processing: {0}")]
    AlreadyProcessing(String),
}

/// Query-specific errors
#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Empty query")]
    EmptyQuery,
    
    #[error("Invalid query mode: {0}")]
    InvalidMode(String),
    
    #[error("Context retrieval failed: {0}")]
    ContextRetrievalFailed(String),
    
    #[error("Response generation failed: {0}")]
    GenerationFailed(String),
}
```

---

### 2. In-Memory Storage Adapters (Testing)

```rust
// edgequake-storage/src/adapters/memory.rs
use crate::traits::{KVStorage, VectorStorage, GraphStorage};
use crate::error::StorageError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory KV storage for testing
pub struct MemoryKVStorage {
    namespace: String,
    data: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl MemoryKVStorage {
    pub fn new(namespace: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl KVStorage for MemoryKVStorage {
    fn namespace(&self) -> &str {
        &self.namespace
    }
    
    async fn initialize(&self) -> Result<(), StorageError> {
        Ok(())
    }
    
    async fn finalize(&self) -> Result<(), StorageError> {
        Ok(())
    }
    
    async fn get_by_id<T: serde::de::DeserializeOwned + Send>(
        &self,
        id: &str,
    ) -> Result<Option<T>, StorageError> {
        let data = self.data.read().await;
        match data.get(id) {
            Some(value) => {
                let result: T = serde_json::from_value(value.clone())
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }
    
    async fn get_by_ids<T: serde::de::DeserializeOwned + Send>(
        &self,
        ids: &[String],
    ) -> Result<Vec<T>, StorageError> {
        let data = self.data.read().await;
        let mut results = Vec::new();
        for id in ids {
            if let Some(value) = data.get(id) {
                let result: T = serde_json::from_value(value.clone())
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;
                results.push(result);
            }
        }
        Ok(results)
    }
    
    async fn filter_keys(&self, keys: std::collections::HashSet<String>) -> Result<std::collections::HashSet<String>, StorageError> {
        let data = self.data.read().await;
        Ok(keys.into_iter().filter(|k| !data.contains_key(k)).collect())
    }
    
    async fn upsert<T: serde::Serialize + Send + Sync>(
        &self,
        items: &[(String, T)],
    ) -> Result<(), StorageError> {
        let mut data = self.data.write().await;
        for (id, value) in items {
            let json = serde_json::to_value(value)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;
            data.insert(id.clone(), json);
        }
        Ok(())
    }
    
    async fn delete(&self, ids: &[String]) -> Result<(), StorageError> {
        let mut data = self.data.write().await;
        for id in ids {
            data.remove(id);
        }
        Ok(())
    }
    
    async fn is_empty(&self) -> Result<bool, StorageError> {
        let data = self.data.read().await;
        Ok(data.is_empty())
    }
}

/// In-memory vector storage for testing
pub struct MemoryVectorStorage {
    namespace: String,
    vectors: Arc<RwLock<HashMap<String, (Vec<f32>, serde_json::Value)>>>,
}

impl MemoryVectorStorage {
    pub fn new(namespace: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            vectors: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Compute cosine similarity between two vectors
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }
}

#[async_trait]
impl VectorStorage for MemoryVectorStorage {
    fn namespace(&self) -> &str {
        &self.namespace
    }
    
    async fn initialize(&self) -> Result<(), StorageError> {
        Ok(())
    }
    
    async fn finalize(&self) -> Result<(), StorageError> {
        Ok(())
    }
    
    async fn query(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
    ) -> Result<Vec<crate::traits::vector::VectorSearchResult>, StorageError> {
        let vectors = self.vectors.read().await;
        
        let mut results: Vec<_> = vectors
            .iter()
            .filter(|(id, _)| {
                filter_ids.map_or(true, |ids| ids.contains(id))
            })
            .map(|(id, (vec, meta))| {
                let score = Self::cosine_similarity(query_embedding, vec);
                crate::traits::vector::VectorSearchResult {
                    id: id.clone(),
                    score,
                    metadata: meta.clone(),
                }
            })
            .collect();
        
        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        
        Ok(results)
    }
    
    async fn upsert(
        &self,
        data: &[(String, Vec<f32>, serde_json::Value)],
    ) -> Result<(), StorageError> {
        let mut vectors = self.vectors.write().await;
        for (id, vec, meta) in data {
            vectors.insert(id.clone(), (vec.clone(), meta.clone()));
        }
        Ok(())
    }
    
    async fn delete(&self, ids: &[String]) -> Result<(), StorageError> {
        let mut vectors = self.vectors.write().await;
        for id in ids {
            vectors.remove(id);
        }
        Ok(())
    }
    
    async fn delete_entity(&self, entity_name: &str) -> Result<(), StorageError> {
        let mut vectors = self.vectors.write().await;
        vectors.retain(|id, _| !id.contains(entity_name));
        Ok(())
    }
    
    async fn delete_entity_relations(&self, entity_name: &str) -> Result<(), StorageError> {
        let mut vectors = self.vectors.write().await;
        vectors.retain(|id, _| !id.contains(entity_name));
        Ok(())
    }
    
    async fn get_by_id(&self, id: &str) -> Result<Option<Vec<f32>>, StorageError> {
        let vectors = self.vectors.read().await;
        Ok(vectors.get(id).map(|(v, _)| v.clone()))
    }
    
    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<(String, Vec<f32>)>, StorageError> {
        let vectors = self.vectors.read().await;
        Ok(ids
            .iter()
            .filter_map(|id| vectors.get(id).map(|(v, _)| (id.clone(), v.clone())))
            .collect())
    }
}
```

---

### 3. PostgreSQL AGE Graph Adapter

```rust
// edgequake-storage/src/adapters/postgres/graph.rs
use crate::traits::graph::{GraphStorage, GraphNode, GraphEdge, KnowledgeGraph};
use crate::error::StorageError;
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::collections::HashMap;

/// PostgreSQL AGE graph storage adapter
/// Reference: tech_stack/postgresql-age-pgvector.md
pub struct PostgresGraphStorage {
    namespace: String,
    pool: PgPool,
    graph_name: String,
}

impl PostgresGraphStorage {
    pub async fn new(namespace: &str, pool: PgPool) -> Result<Self, StorageError> {
        let graph_name = format!("edgequake_{}", namespace);
        Ok(Self {
            namespace: namespace.to_string(),
            pool,
            graph_name,
        })
    }
    
    async fn execute_cypher(&self, query: &str) -> Result<Vec<sqlx::postgres::PgRow>, StorageError> {
        let sql = format!(
            "SELECT * FROM ag_catalog.cypher('{}', $1) AS (result agtype);",
            self.graph_name
        );
        
        sqlx::query(&sql)
            .bind(query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))
    }
}

#[async_trait]
impl GraphStorage for PostgresGraphStorage {
    fn namespace(&self) -> &str {
        &self.namespace
    }
    
    async fn initialize(&self) -> Result<(), StorageError> {
        // Create graph if not exists
        sqlx::query(&format!(
            "SELECT create_graph('{}');",
            self.graph_name
        ))
        .execute(&self.pool)
        .await
        .ok(); // Ignore error if graph already exists
        
        // Create entity label
        let create_entity = format!(
            "MATCH (n) RETURN n LIMIT 0"  // Just validate graph exists
        );
        self.execute_cypher(&create_entity).await?;
        
        Ok(())
    }
    
    async fn finalize(&self) -> Result<(), StorageError> {
        // PostgreSQL handles persistence automatically
        Ok(())
    }
    
    async fn has_node(&self, node_id: &str) -> Result<bool, StorageError> {
        let cypher = format!(
            "MATCH (n:Entity {{id: '{}'}}) RETURN count(n) > 0 AS exists",
            node_id.replace("'", "''")
        );
        
        let rows = self.execute_cypher(&cypher).await?;
        if let Some(row) = rows.first() {
            let result: serde_json::Value = row.try_get("result")
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
            return Ok(result.as_bool().unwrap_or(false));
        }
        Ok(false)
    }
    
    async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>, StorageError> {
        let cypher = format!(
            "MATCH (n:Entity {{id: '{}'}}) RETURN n",
            node_id.replace("'", "''")
        );
        
        let rows = self.execute_cypher(&cypher).await?;
        if let Some(row) = rows.first() {
            let result: serde_json::Value = row.try_get("result")
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
            
            if let Some(obj) = result.as_object() {
                let mut properties = HashMap::new();
                for (k, v) in obj {
                    properties.insert(k.clone(), v.clone());
                }
                return Ok(Some(GraphNode {
                    id: node_id.to_string(),
                    properties,
                }));
            }
        }
        Ok(None)
    }
    
    async fn upsert_node(
        &self,
        node_id: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<(), StorageError> {
        let props_json = serde_json::to_string(&properties)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        
        let cypher = format!(
            "MERGE (n:Entity {{id: '{}'}}) SET n += {} RETURN n",
            node_id.replace("'", "''"),
            props_json
        );
        
        self.execute_cypher(&cypher).await?;
        Ok(())
    }
    
    async fn delete_node(&self, node_id: &str) -> Result<(), StorageError> {
        let cypher = format!(
            "MATCH (n:Entity {{id: '{}'}}) DETACH DELETE n",
            node_id.replace("'", "''")
        );
        
        self.execute_cypher(&cypher).await?;
        Ok(())
    }
    
    async fn node_degree(&self, node_id: &str) -> Result<usize, StorageError> {
        let cypher = format!(
            "MATCH (n:Entity {{id: '{}'}})-[r]-() RETURN count(r) AS degree",
            node_id.replace("'", "''")
        );
        
        let rows = self.execute_cypher(&cypher).await?;
        if let Some(row) = rows.first() {
            let result: serde_json::Value = row.try_get("result")
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
            return Ok(result.as_u64().unwrap_or(0) as usize);
        }
        Ok(0)
    }
    
    async fn get_all_nodes(&self) -> Result<Vec<GraphNode>, StorageError> {
        let cypher = "MATCH (n:Entity) RETURN n";
        let rows = self.execute_cypher(cypher).await?;
        
        let mut nodes = Vec::new();
        for row in rows {
            let result: serde_json::Value = row.try_get("result")
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
            
            if let Some(obj) = result.as_object() {
                let id = obj.get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                
                let mut properties = HashMap::new();
                for (k, v) in obj {
                    properties.insert(k.clone(), v.clone());
                }
                
                nodes.push(GraphNode { id, properties });
            }
        }
        
        Ok(nodes)
    }
    
    async fn has_edge(&self, source: &str, target: &str) -> Result<bool, StorageError> {
        let cypher = format!(
            "MATCH (a:Entity {{id: '{}'}})-[r:RELATES_TO]-(b:Entity {{id: '{}'}}) RETURN count(r) > 0 AS exists",
            source.replace("'", "''"),
            target.replace("'", "''")
        );
        
        let rows = self.execute_cypher(&cypher).await?;
        if let Some(row) = rows.first() {
            let result: serde_json::Value = row.try_get("result")
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
            return Ok(result.as_bool().unwrap_or(false));
        }
        Ok(false)
    }
    
    async fn get_edge(&self, source: &str, target: &str) -> Result<Option<GraphEdge>, StorageError> {
        let cypher = format!(
            "MATCH (a:Entity {{id: '{}'}})-[r:RELATES_TO]-(b:Entity {{id: '{}'}}) RETURN r, a.id AS src, b.id AS tgt",
            source.replace("'", "''"),
            target.replace("'", "''")
        );
        
        let rows = self.execute_cypher(&cypher).await?;
        if let Some(row) = rows.first() {
            let result: serde_json::Value = row.try_get("result")
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
            
            if let Some(obj) = result.as_object() {
                let mut properties = HashMap::new();
                for (k, v) in obj {
                    properties.insert(k.clone(), v.clone());
                }
                
                return Ok(Some(GraphEdge {
                    source: source.to_string(),
                    target: target.to_string(),
                    properties,
                }));
            }
        }
        Ok(None)
    }
    
    async fn upsert_edge(
        &self,
        source: &str,
        target: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<(), StorageError> {
        let props_json = serde_json::to_string(&properties)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        
        let cypher = format!(
            "MATCH (a:Entity {{id: '{}'}}), (b:Entity {{id: '{}'}}) \
             MERGE (a)-[r:RELATES_TO]-(b) \
             SET r += {} \
             RETURN r",
            source.replace("'", "''"),
            target.replace("'", "''"),
            props_json
        );
        
        self.execute_cypher(&cypher).await?;
        Ok(())
    }
    
    async fn delete_edge(&self, source: &str, target: &str) -> Result<(), StorageError> {
        let cypher = format!(
            "MATCH (a:Entity {{id: '{}'}})-[r:RELATES_TO]-(b:Entity {{id: '{}'}}) DELETE r",
            source.replace("'", "''"),
            target.replace("'", "''")
        );
        
        self.execute_cypher(&cypher).await?;
        Ok(())
    }
    
    async fn get_node_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>, StorageError> {
        let cypher = format!(
            "MATCH (a:Entity {{id: '{}'}})-[r:RELATES_TO]-(b:Entity) RETURN r, a.id AS src, b.id AS tgt",
            node_id.replace("'", "''")
        );
        
        let rows = self.execute_cypher(&cypher).await?;
        let mut edges = Vec::new();
        
        for row in rows {
            let result: serde_json::Value = row.try_get("result")
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
            
            if let Some(obj) = result.as_object() {
                let mut properties = HashMap::new();
                for (k, v) in obj {
                    if k != "src" && k != "tgt" {
                        properties.insert(k.clone(), v.clone());
                    }
                }
                
                let src = obj.get("src").and_then(|v| v.as_str()).unwrap_or_default();
                let tgt = obj.get("tgt").and_then(|v| v.as_str()).unwrap_or_default();
                
                edges.push(GraphEdge {
                    source: src.to_string(),
                    target: tgt.to_string(),
                    properties,
                });
            }
        }
        
        Ok(edges)
    }
    
    async fn get_all_edges(&self) -> Result<Vec<GraphEdge>, StorageError> {
        let cypher = "MATCH (a:Entity)-[r:RELATES_TO]-(b:Entity) WHERE a.id < b.id RETURN r, a.id AS src, b.id AS tgt";
        let rows = self.execute_cypher(cypher).await?;
        
        let mut edges = Vec::new();
        for row in rows {
            let result: serde_json::Value = row.try_get("result")
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
            
            if let Some(obj) = result.as_object() {
                let mut properties = HashMap::new();
                for (k, v) in obj {
                    properties.insert(k.clone(), v.clone());
                }
                
                let src = obj.get("src").and_then(|v| v.as_str()).unwrap_or_default();
                let tgt = obj.get("tgt").and_then(|v| v.as_str()).unwrap_or_default();
                
                edges.push(GraphEdge {
                    source: src.to_string(),
                    target: tgt.to_string(),
                    properties,
                });
            }
        }
        
        Ok(edges)
    }
    
    async fn get_knowledge_graph(
        &self,
        start_node: &str,
        max_depth: usize,
        max_nodes: usize,
    ) -> Result<KnowledgeGraph, StorageError> {
        let cypher = if start_node == "*" {
            format!(
                "MATCH (n:Entity) WITH n LIMIT {} \
                 OPTIONAL MATCH (n)-[r:RELATES_TO]-(m:Entity) \
                 RETURN n, r, m",
                max_nodes
            )
        } else {
            format!(
                "MATCH path = (start:Entity {{id: '{}'}})-[*0..{}]-(connected:Entity) \
                 WITH DISTINCT connected LIMIT {} \
                 MATCH (connected)-[r:RELATES_TO]-(other:Entity) \
                 RETURN connected AS n, r, other AS m",
                start_node.replace("'", "''"),
                max_depth,
                max_nodes
            )
        };
        
        let rows = self.execute_cypher(&cypher).await?;
        
        let mut nodes = HashMap::new();
        let mut edges = Vec::new();
        
        for row in rows {
            let result: serde_json::Value = row.try_get("result")
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
            
            if let Some(obj) = result.as_object() {
                // Process node
                if let Some(n) = obj.get("n").and_then(|v| v.as_object()) {
                    let id = n.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                    if !nodes.contains_key(id) {
                        let mut props = HashMap::new();
                        for (k, v) in n {
                            props.insert(k.clone(), v.clone());
                        }
                        nodes.insert(id.to_string(), GraphNode {
                            id: id.to_string(),
                            properties: props,
                        });
                    }
                }
                
                // Process edge if present
                if let (Some(r), Some(m)) = (obj.get("r"), obj.get("m")) {
                    if let (Some(r_obj), Some(m_obj)) = (r.as_object(), m.as_object()) {
                        let src = obj.get("n")
                            .and_then(|v| v.as_object())
                            .and_then(|o| o.get("id"))
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();
                        let tgt = m_obj.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                        
                        let mut props = HashMap::new();
                        for (k, v) in r_obj {
                            props.insert(k.clone(), v.clone());
                        }
                        
                        edges.push(GraphEdge {
                            source: src.to_string(),
                            target: tgt.to_string(),
                            properties: props,
                        });
                    }
                }
            }
        }
        
        let is_truncated = nodes.len() >= max_nodes;
        
        Ok(KnowledgeGraph {
            nodes: nodes.into_values().collect(),
            edges,
            is_truncated,
        })
    }
    
    async fn get_popular_labels(&self, limit: usize) -> Result<Vec<String>, StorageError> {
        let cypher = format!(
            "MATCH (n:Entity)-[r]-() \
             WITH n.id AS id, count(r) AS degree \
             ORDER BY degree DESC \
             LIMIT {} \
             RETURN id",
            limit
        );
        
        let rows = self.execute_cypher(&cypher).await?;
        let mut labels = Vec::new();
        
        for row in rows {
            let result: serde_json::Value = row.try_get("result")
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
            
            if let Some(id) = result.as_str() {
                labels.push(id.to_string());
            }
        }
        
        Ok(labels)
    }
    
    async fn search_labels(&self, query: &str, limit: usize) -> Result<Vec<String>, StorageError> {
        let cypher = format!(
            "MATCH (n:Entity) \
             WHERE n.entity_name CONTAINS '{}' \
             RETURN n.id AS id \
             LIMIT {}",
            query.replace("'", "''").to_uppercase(),
            limit
        );
        
        let rows = self.execute_cypher(&cypher).await?;
        let mut labels = Vec::new();
        
        for row in rows {
            let result: serde_json::Value = row.try_get("result")
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
            
            if let Some(id) = result.as_str() {
                labels.push(id.to_string());
            }
        }
        
        Ok(labels)
    }
}
```

---

### 4. pgvector Vector Adapter

```rust
// edgequake-storage/src/adapters/postgres/vector.rs
use crate::traits::vector::{VectorStorage, VectorSearchResult};
use crate::error::StorageError;
use async_trait::async_trait;
use sqlx::{PgPool, Row};

/// PostgreSQL pgvector storage adapter
/// Reference: tech_stack/postgresql-age-pgvector.md
pub struct PgVectorStorage {
    namespace: String,
    pool: PgPool,
    table_name: String,
    dimension: usize,
}

impl PgVectorStorage {
    pub fn new(namespace: &str, pool: PgPool, dimension: usize) -> Self {
        let table_name = format!("edgequake_vectors_{}", namespace);
        Self {
            namespace: namespace.to_string(),
            pool,
            table_name,
            dimension,
        }
    }
}

#[async_trait]
impl VectorStorage for PgVectorStorage {
    fn namespace(&self) -> &str {
        &self.namespace
    }
    
    async fn initialize(&self) -> Result<(), StorageError> {
        // Enable pgvector extension
        sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::SchemaError(e.to_string()))?;
        
        // Create table
        let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                embedding vector({}),
                metadata JSONB NOT NULL DEFAULT '{{}}'::jsonb,
                created_at TIMESTAMPTZ DEFAULT NOW()
            )",
            self.table_name,
            self.dimension
        );
        
        sqlx::query(&create_sql)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::SchemaError(e.to_string()))?;
        
        // Create HNSW index for fast similarity search
        let index_sql = format!(
            "CREATE INDEX IF NOT EXISTS {}_embedding_idx \
             ON {} USING hnsw (embedding vector_cosine_ops)",
            self.table_name,
            self.table_name
        );
        
        sqlx::query(&index_sql)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::SchemaError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn finalize(&self) -> Result<(), StorageError> {
        // PostgreSQL handles persistence automatically
        Ok(())
    }
    
    async fn query(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
    ) -> Result<Vec<VectorSearchResult>, StorageError> {
        let embedding_str = format!("[{}]", 
            query_embedding.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        
        let sql = if let Some(ids) = filter_ids {
            let id_list = ids.iter()
                .map(|id| format!("'{}'", id.replace("'", "''")))
                .collect::<Vec<_>>()
                .join(",");
            
            format!(
                "SELECT id, metadata, 1 - (embedding <=> '{}') AS score \
                 FROM {} \
                 WHERE id IN ({}) \
                 ORDER BY embedding <=> '{}' \
                 LIMIT {}",
                embedding_str,
                self.table_name,
                id_list,
                embedding_str,
                top_k
            )
        } else {
            format!(
                "SELECT id, metadata, 1 - (embedding <=> '{}') AS score \
                 FROM {} \
                 ORDER BY embedding <=> '{}' \
                 LIMIT {}",
                embedding_str,
                self.table_name,
                embedding_str,
                top_k
            )
        };
        
        let rows = sqlx::query(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
        
        let mut results = Vec::new();
        for row in rows {
            let id: String = row.try_get("id")
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
            let score: f32 = row.try_get("score")
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
            let metadata: serde_json::Value = row.try_get("metadata")
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
            
            results.push(VectorSearchResult { id, score, metadata });
        }
        
        Ok(results)
    }
    
    async fn upsert(
        &self,
        data: &[(String, Vec<f32>, serde_json::Value)],
    ) -> Result<(), StorageError> {
        for (id, embedding, metadata) in data {
            let embedding_str = format!("[{}]", 
                embedding.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );
            
            let sql = format!(
                "INSERT INTO {} (id, embedding, metadata) \
                 VALUES ($1, '{}', $2) \
                 ON CONFLICT (id) DO UPDATE SET \
                 embedding = EXCLUDED.embedding, \
                 metadata = EXCLUDED.metadata",
                self.table_name,
                embedding_str
            );
            
            sqlx::query(&sql)
                .bind(id)
                .bind(metadata)
                .execute(&self.pool)
                .await
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
        }
        
        Ok(())
    }
    
    async fn delete(&self, ids: &[String]) -> Result<(), StorageError> {
        if ids.is_empty() {
            return Ok(());
        }
        
        let id_list = ids.iter()
            .map(|id| format!("'{}'", id.replace("'", "''")))
            .collect::<Vec<_>>()
            .join(",");
        
        let sql = format!("DELETE FROM {} WHERE id IN ({})", self.table_name, id_list);
        
        sqlx::query(&sql)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
        
        Ok(())
    }
    
    async fn delete_entity(&self, entity_name: &str) -> Result<(), StorageError> {
        let sql = format!(
            "DELETE FROM {} WHERE metadata->>'entity_name' = $1",
            self.table_name
        );
        
        sqlx::query(&sql)
            .bind(entity_name)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
        
        Ok(())
    }
    
    async fn delete_entity_relations(&self, entity_name: &str) -> Result<(), StorageError> {
        let sql = format!(
            "DELETE FROM {} WHERE \
             metadata->>'src_id' = $1 OR metadata->>'tgt_id' = $1",
            self.table_name
        );
        
        sqlx::query(&sql)
            .bind(entity_name)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
        
        Ok(())
    }
    
    async fn get_by_id(&self, id: &str) -> Result<Option<Vec<f32>>, StorageError> {
        let sql = format!(
            "SELECT embedding::text FROM {} WHERE id = $1",
            self.table_name
        );
        
        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
        
        if let Some(row) = row {
            let embedding_str: String = row.try_get("embedding")
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
            
            // Parse "[1.0, 2.0, ...]" format
            let embedding: Vec<f32> = embedding_str
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            
            Ok(Some(embedding))
        } else {
            Ok(None)
        }
    }
    
    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<(String, Vec<f32>)>, StorageError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        
        let id_list = ids.iter()
            .map(|id| format!("'{}'", id.replace("'", "''")))
            .collect::<Vec<_>>()
            .join(",");
        
        let sql = format!(
            "SELECT id, embedding::text FROM {} WHERE id IN ({})",
            self.table_name,
            id_list
        );
        
        let rows = sqlx::query(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
        
        let mut results = Vec::new();
        for row in rows {
            let id: String = row.try_get("id")
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
            let embedding_str: String = row.try_get("embedding")
                .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
            
            let embedding: Vec<f32> = embedding_str
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            
            results.push((id, embedding));
        }
        
        Ok(results)
    }
}
```

---

## Week-by-Week Tasks

### Week 3: Storage Framework

| Task | Description | Owner | Status |
|------|-------------|-------|--------|
| 2.1.1 | Define error type hierarchy | Backend | ⬜ |
| 2.1.2 | Implement KVStorage trait | Backend | ⬜ |
| 2.1.3 | Implement VectorStorage trait | Backend | ⬜ |
| 2.1.4 | Implement GraphStorage trait | Backend | ⬜ |
| 2.1.5 | Create MemoryKVStorage | Backend | ⬜ |
| 2.1.6 | Create MemoryVectorStorage | Backend | ⬜ |
| 2.1.7 | Create MemoryGraphStorage | Backend | ⬜ |
| 2.1.8 | Write unit tests for memory adapters | QA | ⬜ |

### Week 4: Database Adapters

| Task | Description | Owner | Status |
|------|-------------|-------|--------|
| 2.2.1 | Set up PostgreSQL + AGE + pgvector | DevOps | ⬜ |
| 2.2.2 | Implement PostgresKVStorage | Backend | ⬜ |
| 2.2.3 | Implement PostgresGraphStorage (AGE) | Backend | ⬜ |
| 2.2.4 | Implement PgVectorStorage | Backend | ⬜ |
| 2.2.5 | Create docker-compose.yml for dev | DevOps | ⬜ |
| 2.2.6 | Write integration tests | QA | ⬜ |
| 2.2.7 | (Optional) Implement SurrealDBStorage | Backend | ⬜ |
| 2.2.8 | Document adapter usage | Tech Writer | ⬜ |

---

## Acceptance Criteria

- [ ] All storage traits are object-safe and async
- [ ] Memory adapters pass all CRUD tests
- [ ] PostgreSQL adapters connect and run queries
- [ ] AGE graph adapter executes Cypher queries
- [ ] pgvector performs similarity search correctly
- [ ] Error types cover all failure scenarios
- [ ] Integration tests achieve 80%+ coverage
- [ ] Docker development environment works

---

## Docker Development Environment

```yaml
# docker-compose.yml
version: '3.8'

services:
  postgres:
    image: apache/age:latest
    container_name: edgequake-db
    environment:
      POSTGRES_USER: edgequake
      POSTGRES_PASSWORD: edgequake_secret
      POSTGRES_DB: edgequake
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./scripts/init-db.sql:/docker-entrypoint-initdb.d/init.sql
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U edgequake"]
      interval: 5s
      timeout: 5s
      retries: 5

  surrealdb:
    image: surrealdb/surrealdb:latest
    container_name: edgequake-surreal
    command: start --auth --user root --pass root
    ports:
      - "8000:8000"
    volumes:
      - surreal_data:/data

volumes:
  postgres_data:
  surreal_data:
```

```sql
-- scripts/init-db.sql
-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS age;

-- Load AGE
LOAD 'age';
SET search_path = ag_catalog, "$user", public;

-- Create test graph
SELECT create_graph('edgequake_test');
```

---

## Dependencies

```toml
[workspace.dependencies]
# Database
sqlx = { version = "0.7", features = ["runtime-tokio", "postgres", "json"] }
surrealdb = "1.0"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Async
tokio = { version = "1.35", features = ["full"] }
async-trait = "0.1"
```

---

## Related Documents

- [Phase 1: Component Mapping](phase-1-component-mapping.md) - Previous phase
- [Phase 3: Development Roadmap](phase-3-development-roadmap.md) - Next phase
- [master.md](../master.md) - Overall plan
- [craft_pad.md](../craft_pad.md) - Working notes
