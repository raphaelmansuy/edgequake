# Phase 3: Storage Backend Expansion

**Document ID:** 05-PHASE3-STORAGE  
**Priority:** 🟡 P2 MEDIUM  
**Effort:** 10 person-days  
**Duration:** Weeks 7-9  
**Dependencies:** None  
**Blocks:** None

---

## 📋 Overview

This document provides implementation guidance for expanding storage backend support, including Neo4j for graph storage, Qdrant for vector storage, and Redis for KV caching.

### Gaps Addressed

| Gap ID      | Feature               | Severity | Status         | Effort |
| ----------- | --------------------- | -------- | -------------- | ------ |
| **GAP-012** | Neo4j Storage         | 🟡 P2    | 🔲 Not started | 4 days |
| **GAP-013** | Milvus/Qdrant Storage | 🟡 P2    | 🔲 Not started | 3 days |
| **GAP-024** | Redis Storage         | 🟢 P3    | 🔲 Not started | 3 days |

### Cross-References

- **Source Analysis:** [../gap-analysis.md](../gap-analysis.md#feature-f-033-neo4j)
- **Master Plan:** [00-INDEX.md](./00-INDEX.md#phase-3-expansion-weeks-7-9)
- **Testing Plan:** [07-VALIDATION-TESTING.md](./07-VALIDATION-TESTING.md#storage-tests)

---

## 🎯 Neo4j Graph Storage

### 1.1 Objective

Implement Neo4j graph storage backend for production-grade knowledge graph management.

### 1.2 Source Reference

**Location:** `lightrag/kg/neo4j_impl.py`
**Driver:** neo4rs (Rust Neo4j driver)

### 1.3 Implementation Tasks

#### Task 1.3.1: Create Neo4j Graph Storage

**File:** `edgequake/crates/edgequake-storage/src/neo4j/graph.rs` (NEW)

```rust
// NEW FILE: edgequake/crates/edgequake-storage/src/neo4j/graph.rs

//! Neo4j graph storage implementation.

use crate::traits::{GraphStorage, StorageResult, StorageError, GraphNode, GraphEdge};
use async_trait::async_trait;
use neo4rs::{Graph, query, Node, Relation};
use std::collections::HashMap;

/// Neo4j graph storage configuration
#[derive(Debug, Clone)]
pub struct Neo4jConfig {
    pub uri: String,
    pub username: String,
    pub password: String,
    pub database: String,
    pub max_connections: usize,
}

impl Default for Neo4jConfig {
    fn default() -> Self {
        Self {
            uri: std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string()),
            username: std::env::var("NEO4J_USERNAME").unwrap_or_else(|_| "neo4j".to_string()),
            password: std::env::var("NEO4J_PASSWORD").unwrap_or_default(),
            database: "neo4j".to_string(),
            max_connections: 10,
        }
    }
}

/// Neo4j graph storage
pub struct Neo4jGraphStorage {
    graph: Graph,
    database: String,
}

impl Neo4jGraphStorage {
    pub async fn new(config: Neo4jConfig) -> StorageResult<Self> {
        let graph = Graph::new(
            &config.uri,
            &config.username,
            &config.password,
        ).await
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        Ok(Self {
            graph,
            database: config.database,
        })
    }

    pub async fn from_env() -> StorageResult<Self> {
        Self::new(Neo4jConfig::default()).await
    }

    /// Initialize schema (indexes and constraints)
    pub async fn init_schema(&self) -> StorageResult<()> {
        // Create index on entity name
        self.graph.run(query(
            "CREATE INDEX entity_name IF NOT EXISTS FOR (e:Entity) ON (e.name)"
        )).await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        // Create index on entity type
        self.graph.run(query(
            "CREATE INDEX entity_type IF NOT EXISTS FOR (e:Entity) ON (e.entity_type)"
        )).await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    fn node_to_graph_node(node: Node) -> GraphNode {
        let id = node.get::<String>("name").unwrap_or_default();
        let mut properties = HashMap::new();

        // Extract all properties
        if let Some(name) = node.get::<String>("name") {
            properties.insert("name".to_string(), serde_json::json!(name));
        }
        if let Some(entity_type) = node.get::<String>("entity_type") {
            properties.insert("entity_type".to_string(), serde_json::json!(entity_type));
        }
        if let Some(description) = node.get::<String>("description") {
            properties.insert("description".to_string(), serde_json::json!(description));
        }
        if let Some(source_id) = node.get::<String>("source_id") {
            properties.insert("source_id".to_string(), serde_json::json!(source_id));
        }

        GraphNode { id, properties }
    }
}

#[async_trait]
impl GraphStorage for Neo4jGraphStorage {
    async fn get_node(&self, id: &str) -> StorageResult<Option<GraphNode>> {
        let mut result = self.graph.execute(
            query("MATCH (e:Entity {name: $name}) RETURN e")
                .param("name", id)
        ).await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        if let Some(row) = result.next().await
            .map_err(|e| StorageError::Query(e.to_string()))?
        {
            let node: Node = row.get("e")
                .map_err(|e| StorageError::Query(e.to_string()))?;
            Ok(Some(Self::node_to_graph_node(node)))
        } else {
            Ok(None)
        }
    }

    async fn upsert_node(
        &self,
        id: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> StorageResult<()> {
        let props_json = serde_json::to_string(&properties)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        self.graph.run(
            query("MERGE (e:Entity {name: $name}) SET e += $props")
                .param("name", id)
                .param("props", props_json)
        ).await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    async fn delete_node(&self, id: &str) -> StorageResult<()> {
        self.graph.run(
            query("MATCH (e:Entity {name: $name}) DETACH DELETE e")
                .param("name", id)
        ).await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    async fn get_node_edges(&self, id: &str) -> StorageResult<Vec<GraphEdge>> {
        let mut result = self.graph.execute(
            query(
                "MATCH (e:Entity {name: $name})-[r]-(other:Entity)
                 RETURN e.name as source, type(r) as rel_type, other.name as target, properties(r) as props"
            ).param("name", id)
        ).await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        let mut edges = Vec::new();
        while let Some(row) = result.next().await
            .map_err(|e| StorageError::Query(e.to_string()))?
        {
            let source: String = row.get("source")
                .map_err(|e| StorageError::Query(e.to_string()))?;
            let target: String = row.get("target")
                .map_err(|e| StorageError::Query(e.to_string()))?;

            edges.push(GraphEdge {
                source,
                target,
                properties: HashMap::new(), // Parse from props
            });
        }

        Ok(edges)
    }

    async fn upsert_edge(
        &self,
        source: &str,
        target: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> StorageResult<()> {
        let props_json = serde_json::to_string(&properties)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        self.graph.run(
            query(
                "MATCH (s:Entity {name: $source}), (t:Entity {name: $target})
                 MERGE (s)-[r:RELATED]->(t)
                 SET r += $props"
            )
                .param("source", source)
                .param("target", target)
                .param("props", props_json)
        ).await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    async fn delete_edge(&self, source: &str, target: &str) -> StorageResult<()> {
        self.graph.run(
            query(
                "MATCH (s:Entity {name: $source})-[r]->(t:Entity {name: $target})
                 DELETE r"
            )
                .param("source", source)
                .param("target", target)
        ).await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    async fn get_all_nodes(&self) -> StorageResult<Vec<GraphNode>> {
        let mut result = self.graph.execute(
            query("MATCH (e:Entity) RETURN e LIMIT 10000")
        ).await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        let mut nodes = Vec::new();
        while let Some(row) = result.next().await
            .map_err(|e| StorageError::Query(e.to_string()))?
        {
            let node: Node = row.get("e")
                .map_err(|e| StorageError::Query(e.to_string()))?;
            nodes.push(Self::node_to_graph_node(node));
        }

        Ok(nodes)
    }

    async fn get_all_edges(&self) -> StorageResult<Vec<GraphEdge>> {
        let mut result = self.graph.execute(
            query(
                "MATCH (s:Entity)-[r]->(t:Entity)
                 RETURN s.name as source, t.name as target LIMIT 50000"
            )
        ).await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        let mut edges = Vec::new();
        while let Some(row) = result.next().await
            .map_err(|e| StorageError::Query(e.to_string()))?
        {
            let source: String = row.get("source")
                .map_err(|e| StorageError::Query(e.to_string()))?;
            let target: String = row.get("target")
                .map_err(|e| StorageError::Query(e.to_string()))?;

            edges.push(GraphEdge {
                source,
                target,
                properties: HashMap::new(),
            });
        }

        Ok(edges)
    }

    async fn clear(&self) -> StorageResult<()> {
        self.graph.run(query("MATCH (n) DETACH DELETE n")).await
            .map_err(|e| StorageError::Query(e.to_string()))?;
        Ok(())
    }
}
```

**Dependencies:**

```toml
# Add to edgequake/crates/edgequake-storage/Cargo.toml [dependencies]
neo4rs = "0.7"
```

---

## 🎯 Qdrant Vector Storage

### 2.1 Objective

Implement Qdrant vector storage for high-performance vector similarity search.

### 2.2 Implementation Tasks

#### Task 2.2.1: Create Qdrant Vector Storage

**File:** `edgequake/crates/edgequake-storage/src/qdrant/vector.rs` (NEW)

```rust
// NEW FILE: edgequake/crates/edgequake-storage/src/qdrant/vector.rs

//! Qdrant vector storage implementation.

use crate::traits::{VectorStorage, VectorQueryResult, VectorNamespace, StorageResult, StorageError};
use async_trait::async_trait;
use qdrant_client::prelude::*;
use qdrant_client::qdrant::{
    vectors_config::Config, CreateCollection, Distance, PointStruct,
    SearchPoints, VectorParams, VectorsConfig, Filter, Condition,
};
use std::collections::HashMap;
use uuid::Uuid;

/// Qdrant vector storage configuration
#[derive(Debug, Clone)]
pub struct QdrantConfig {
    pub url: String,
    pub api_key: Option<String>,
    pub collection_prefix: String,
    pub vector_size: u64,
}

impl Default for QdrantConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6334".to_string()),
            api_key: std::env::var("QDRANT_API_KEY").ok(),
            collection_prefix: "edgequake".to_string(),
            vector_size: 1536, // OpenAI text-embedding-3-small
        }
    }
}

/// Qdrant vector storage
pub struct QdrantVectorStorage {
    client: QdrantClient,
    config: QdrantConfig,
}

impl QdrantVectorStorage {
    pub async fn new(config: QdrantConfig) -> StorageResult<Self> {
        let mut client_config = QdrantClientConfig::from_url(&config.url);

        if let Some(api_key) = &config.api_key {
            client_config = client_config.with_api_key(api_key);
        }

        let client = QdrantClient::new(Some(client_config))
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        let storage = Self { client, config };
        storage.ensure_collections().await?;

        Ok(storage)
    }

    pub async fn from_env() -> StorageResult<Self> {
        Self::new(QdrantConfig::default()).await
    }

    fn collection_name(&self, namespace: VectorNamespace) -> String {
        let suffix = match namespace {
            VectorNamespace::Chunk => "chunks",
            VectorNamespace::Entity => "entities",
            VectorNamespace::Relationship => "relationships",
        };
        format!("{}_{}", self.config.collection_prefix, suffix)
    }

    async fn ensure_collections(&self) -> StorageResult<()> {
        for namespace in [VectorNamespace::Chunk, VectorNamespace::Entity, VectorNamespace::Relationship] {
            let collection_name = self.collection_name(namespace);

            // Check if collection exists
            let exists = self.client
                .collection_exists(&collection_name)
                .await
                .map_err(|e| StorageError::Query(e.to_string()))?;

            if !exists {
                // Create collection
                self.client
                    .create_collection(&CreateCollection {
                        collection_name: collection_name.clone(),
                        vectors_config: Some(VectorsConfig {
                            config: Some(Config::Params(VectorParams {
                                size: self.config.vector_size,
                                distance: Distance::Cosine.into(),
                                ..Default::default()
                            })),
                        }),
                        ..Default::default()
                    })
                    .await
                    .map_err(|e| StorageError::Query(e.to_string()))?;

                tracing::info!(collection = %collection_name, "Created Qdrant collection");
            }
        }

        Ok(())
    }
}

#[async_trait]
impl VectorStorage for QdrantVectorStorage {
    async fn insert(
        &self,
        vectors: Vec<(String, Vec<f32>, HashMap<String, serde_json::Value>)>,
    ) -> StorageResult<()> {
        self.insert_with_namespace(vectors, VectorNamespace::Chunk).await
    }

    async fn insert_with_namespace(
        &self,
        vectors: Vec<(String, Vec<f32>, HashMap<String, serde_json::Value>)>,
        namespace: VectorNamespace,
    ) -> StorageResult<()> {
        let collection_name = self.collection_name(namespace);

        let points: Vec<PointStruct> = vectors
            .into_iter()
            .map(|(id, embedding, metadata)| {
                let payload: HashMap<String, qdrant_client::qdrant::Value> = metadata
                    .into_iter()
                    .map(|(k, v)| {
                        let value = match v {
                            serde_json::Value::String(s) => qdrant_client::qdrant::Value::from(s),
                            serde_json::Value::Number(n) => {
                                if let Some(i) = n.as_i64() {
                                    qdrant_client::qdrant::Value::from(i)
                                } else if let Some(f) = n.as_f64() {
                                    qdrant_client::qdrant::Value::from(f)
                                } else {
                                    qdrant_client::qdrant::Value::from(n.to_string())
                                }
                            }
                            serde_json::Value::Bool(b) => qdrant_client::qdrant::Value::from(b),
                            _ => qdrant_client::qdrant::Value::from(v.to_string()),
                        };
                        (k, value)
                    })
                    .collect();

                PointStruct::new(id, embedding, payload)
            })
            .collect();

        self.client
            .upsert_points(&collection_name, None, points, None)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    async fn query(
        &self,
        query_vector: &[f32],
        top_k: usize,
        filter: Option<&serde_json::Value>,
    ) -> StorageResult<Vec<VectorQueryResult>> {
        self.query_with_namespace(query_vector, top_k, VectorNamespace::Chunk, filter).await
    }

    async fn query_with_namespace(
        &self,
        query_vector: &[f32],
        top_k: usize,
        namespace: VectorNamespace,
        _filter: Option<&serde_json::Value>,
    ) -> StorageResult<Vec<VectorQueryResult>> {
        let collection_name = self.collection_name(namespace);

        let search_result = self.client
            .search_points(&SearchPoints {
                collection_name,
                vector: query_vector.to_vec(),
                limit: top_k as u64,
                with_payload: Some(true.into()),
                ..Default::default()
            })
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        let results = search_result.result
            .into_iter()
            .map(|point| {
                let id = match point.id {
                    Some(id) => match id.point_id_options {
                        Some(qdrant_client::qdrant::point_id::PointIdOptions::Uuid(u)) => u,
                        Some(qdrant_client::qdrant::point_id::PointIdOptions::Num(n)) => n.to_string(),
                        None => String::new(),
                    },
                    None => String::new(),
                };

                let metadata: HashMap<String, serde_json::Value> = point.payload
                    .into_iter()
                    .map(|(k, v)| {
                        let json_value = match v.kind {
                            Some(qdrant_client::qdrant::value::Kind::StringValue(s)) => serde_json::json!(s),
                            Some(qdrant_client::qdrant::value::Kind::IntegerValue(i)) => serde_json::json!(i),
                            Some(qdrant_client::qdrant::value::Kind::DoubleValue(d)) => serde_json::json!(d),
                            Some(qdrant_client::qdrant::value::Kind::BoolValue(b)) => serde_json::json!(b),
                            _ => serde_json::Value::Null,
                        };
                        (k, json_value)
                    })
                    .collect();

                VectorQueryResult {
                    id,
                    score: point.score,
                    metadata,
                }
            })
            .collect();

        Ok(results)
    }

    async fn delete(&self, ids: &[String]) -> StorageResult<()> {
        let collection_name = self.collection_name(VectorNamespace::Chunk);

        let points: Vec<qdrant_client::qdrant::PointId> = ids
            .iter()
            .map(|id| qdrant_client::qdrant::PointId::from(id.clone()))
            .collect();

        self.client
            .delete_points(&collection_name, None, &points.into(), None)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    async fn clear(&self) -> StorageResult<()> {
        for namespace in [VectorNamespace::Chunk, VectorNamespace::Entity, VectorNamespace::Relationship] {
            let collection_name = self.collection_name(namespace);

            // Delete and recreate collection
            let _ = self.client.delete_collection(&collection_name).await;
        }

        self.ensure_collections().await
    }
}
```

**Dependencies:**

```toml
# Add to edgequake/crates/edgequake-storage/Cargo.toml
qdrant-client = "1.7"
uuid = { version = "1.0", features = ["v4"] }
```

---

## 🎯 Redis KV Storage

### 3.1 Objective

Implement Redis KV storage for high-performance caching and key-value operations.

### 3.2 Implementation Tasks

#### Task 3.2.1: Create Redis KV Storage

**File:** `edgequake/crates/edgequake-storage/src/redis/kv.rs` (NEW)

```rust
// NEW FILE: edgequake/crates/edgequake-storage/src/redis/kv.rs

//! Redis KV storage implementation.

use crate::traits::{KVStorage, StorageResult, StorageError, TenantContext};
use async_trait::async_trait;
use redis::{AsyncCommands, Client};
use std::sync::Arc;

/// Redis KV storage configuration
#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub key_prefix: String,
    pub default_ttl: Option<usize>,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            key_prefix: "edgequake".to_string(),
            default_ttl: None,
        }
    }
}

/// Redis KV storage
pub struct RedisKVStorage {
    client: Client,
    config: RedisConfig,
    tenant_context: Option<TenantContext>,
}

impl RedisKVStorage {
    pub fn new(config: RedisConfig) -> StorageResult<Self> {
        let client = Client::open(config.url.clone())
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        Ok(Self {
            client,
            config,
            tenant_context: None,
        })
    }

    pub fn from_env() -> StorageResult<Self> {
        Self::new(RedisConfig::default())
    }

    fn full_key(&self, key: &str) -> String {
        match &self.tenant_context {
            Some(ctx) => format!("{}:{}:{}:{}", self.config.key_prefix, ctx.tenant_id, ctx.kb_id, key),
            None => format!("{}:{}", self.config.key_prefix, key),
        }
    }

    async fn get_connection(&self) -> StorageResult<redis::aio::MultiplexedConnection> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| StorageError::Connection(e.to_string()))
    }
}

#[async_trait]
impl KVStorage for RedisKVStorage {
    fn with_tenant_context(&self, context: TenantContext) -> Box<dyn KVStorage> {
        Box::new(Self {
            client: self.client.clone(),
            config: self.config.clone(),
            tenant_context: Some(context),
        })
    }

    async fn get(&self, key: &str) -> StorageResult<Option<serde_json::Value>> {
        let mut conn = self.get_connection().await?;
        let full_key = self.full_key(key);

        let value: Option<String> = conn.get(&full_key).await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        match value {
            Some(json_str) => {
                let parsed = serde_json::from_str(&json_str)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    async fn set(&self, key: &str, value: serde_json::Value) -> StorageResult<()> {
        let mut conn = self.get_connection().await?;
        let full_key = self.full_key(key);
        let json_str = serde_json::to_string(&value)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        match self.config.default_ttl {
            Some(ttl) => {
                conn.set_ex(&full_key, &json_str, ttl as u64).await
                    .map_err(|e| StorageError::Query(e.to_string()))?;
            }
            None => {
                conn.set(&full_key, &json_str).await
                    .map_err(|e| StorageError::Query(e.to_string()))?;
            }
        }

        Ok(())
    }

    async fn delete(&self, key: &str) -> StorageResult<()> {
        let mut conn = self.get_connection().await?;
        let full_key = self.full_key(key);

        conn.del(&full_key).await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    async fn list_keys(&self, prefix: &str) -> StorageResult<Vec<String>> {
        let mut conn = self.get_connection().await?;
        let pattern = format!("{}*", self.full_key(prefix));

        let keys: Vec<String> = conn.keys(&pattern).await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        // Strip the full prefix from keys
        let base_prefix = self.full_key("");
        let stripped: Vec<String> = keys
            .into_iter()
            .filter_map(|k| k.strip_prefix(&base_prefix).map(|s| s.to_string()))
            .collect();

        Ok(stripped)
    }

    async fn exists(&self, key: &str) -> StorageResult<bool> {
        let mut conn = self.get_connection().await?;
        let full_key = self.full_key(key);

        let exists: bool = conn.exists(&full_key).await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(exists)
    }
}
```

**Dependencies:**

```toml
# Add to edgequake/crates/edgequake-storage/Cargo.toml
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }
```

---

## 📊 Testing Requirements

### Docker Compose for Testing

```yaml
# docker-compose.storage-test.yml
version: "3.8"
services:
  neo4j:
    image: neo4j:5.15-community
    ports:
      - "7474:7474"
      - "7687:7687"
    environment:
      NEO4J_AUTH: neo4j/password

  qdrant:
    image: qdrant/qdrant:latest
    ports:
      - "6333:6333"
      - "6334:6334"

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
```

### Integration Tests

```bash
# Start services
docker-compose -f docker-compose.storage-test.yml up -d

# Run tests
cargo test --package edgequake-storage --test neo4j_integration
cargo test --package edgequake-storage --test qdrant_integration
cargo test --package edgequake-storage --test redis_integration
```

---

## 🔗 Cross-References

| Topic        | Document                                               | Section       |
| ------------ | ------------------------------------------------------ | ------------- |
| Gap Details  | [../gap-analysis.md](../gap-analysis.md)               | F-033, F-036  |
| Testing Plan | [07-VALIDATION-TESTING.md](./07-VALIDATION-TESTING.md) | Storage Tests |
| Master Index | [00-INDEX.md](./00-INDEX.md)                           | Phase 3       |

---

## ✅ Completion Criteria

| Criterion           | Target          | Validation       |
| ------------------- | --------------- | ---------------- |
| Neo4j CRUD works    | All operations  | Integration test |
| Qdrant search works | Correct results | Integration test |
| Redis caching works | Get/Set cycle   | Integration test |
| Tenant isolation    | No cross-tenant | Integration test |

---

_Document Version: 1.0_  
_Last Updated: 2024-12-24_  
_Owner: EdgeQuake Storage Team_
