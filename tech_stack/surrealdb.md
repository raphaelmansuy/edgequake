# SurrealDB Multi-Model Database Guide

**Version**: Latest (2025)  
**Category**: Multi-Model Database  
**Use Case**: Primary database for LightRAG (Graph + Vector + Document + KV)  
**Official Docs**: https://surrealdb.com/docs

---

## Overview

SurrealDB is a native Rust, multi-model database that combines document, graph, vector, time-series, and key-value capabilities in a single system. For LightRAG, it replaces **12 separate storage instances** with one unified database.

### Why SurrealDB for LightRAG?

**Python LightRAG uses 12 storage instances**:
- 4x Key-Value Storage (docs, status, chunks, cache)
- 1x Graph Storage (knowledge graph)
- 3x Vector Storage (chunks, entities, relationships)

**Rust LightRAG with SurrealDB = 1 database**:
- ✅ Documents, chunks, and status as tables
- ✅ Native graph relations
- ✅ Built-in vector search
- ✅ Key-value functionality

---

## Installation

### Cargo.toml

```toml
[dependencies]
surrealdb = "2.0"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

### Running SurrealDB Server

```bash
# Docker (recommended for development)
docker run --rm -p 8000:8000 surrealdb/surrealdb:latest start --log trace

# Or via cargo install
cargo install surrealdb
surreal start --log trace
```

---

## Core Concepts

### 1. Namespaces & Databases

Hierarchical organization:
```
Namespace (e.g., "lightrag")
  └─ Database (e.g., "production" or "tenant_123")
      └─ Tables (entity, relationship, document, etc.)
```

### 2. Tables & Records

- **Tables**: Like SQL tables, define schema
- **Records**: Individual rows with unique IDs
- **Record IDs**: Format: `table:id` (e.g., `entity:RUST`)

### 3. Graph Relations

Native graph support with `->` and `<-` operators:
```sql
-- Create entity
CREATE entity:RUST SET name = "Rust", type = "technology";

-- Create relationship (graph edge)
RELATE entity:RUST->uses->entity:TOKIO SET description = "async runtime";

-- Traverse graph
SELECT * FROM entity:RUST->uses;
```

### 4. Vector Search

Built-in vector similarity search:
```sql
-- Define vector index
DEFINE INDEX embedding_idx ON entity FIELDS embedding MTREE DIMENSION 1536;

-- Vector search
SELECT * FROM entity 
WHERE embedding <|1536|> [0.1, 0.2, ..., 0.9];
```

---

## Progressive Examples

### 1. Connection & Setup

```rust
use surrealdb::{engine::remote::ws::Ws, Surreal};

#[tokio::main]
async fn main() -> surrealdb::Result<()> {
    // Connect to SurrealDB
    let db = Surreal::new::<Ws>("localhost:8000").await?;
    
    // Sign in (for authentication)
    db.signin(surrealdb::opt::auth::Root {
        username: "root",
        password: "root",
    }).await?;
    
    // Use namespace and database
    db.use_ns("lightrag").use_db("production").await?;
    
    println!("Connected to SurrealDB");
    
    Ok(())
}
```

**Key Points**:
- Use `Ws` (WebSocket) for remote connections
- Can also use `Mem` for in-memory or `File` for on-disk
- Must call `signin()` and `use_ns().use_db()` before queries

### 2. Define Schema

```rust
use surrealdb::sql;

async fn setup_schema(db: &Surreal<Db>) -> surrealdb::Result<()> {
    // Define entity table
    db.query("
        DEFINE TABLE entity SCHEMAFULL;
        
        DEFINE FIELD name ON entity TYPE string;
        DEFINE FIELD entity_type ON entity TYPE string;
        DEFINE FIELD description ON entity TYPE string;
        DEFINE FIELD embedding ON entity TYPE array<float>;
        DEFINE FIELD created_at ON entity TYPE datetime DEFAULT time::now();
        
        -- Vector index for similarity search
        DEFINE INDEX entity_embedding_idx ON entity 
            FIELDS embedding MTREE DIMENSION 1536;
        
        -- Text index for full-text search
        DEFINE INDEX entity_name_idx ON entity 
            FIELDS name SEARCH ANALYZER ascii BM25;
    ").await?;
    
    // Define relationship table (graph edges)
    db.query("
        DEFINE TABLE relationship SCHEMAFULL;
        
        DEFINE FIELD in ON relationship TYPE record<entity>;
        DEFINE FIELD out ON relationship TYPE record<entity>;
        DEFINE FIELD description ON relationship TYPE string;
        DEFINE FIELD keywords ON relationship TYPE array<string>;
        DEFINE FIELD weight ON relationship TYPE float DEFAULT 1.0;
    ").await?;
    
    // Define document table
    db.query("
        DEFINE TABLE document SCHEMAFULL;
        
        DEFINE FIELD content ON document TYPE string;
        DEFINE FIELD file_path ON document TYPE option<string>;
        DEFINE FIELD status ON document TYPE string 
            ASSERT $value IN ['PENDING', 'PROCESSING', 'PROCESSED', 'FAILED'];
        DEFINE FIELD track_id ON document TYPE string;
        DEFINE FIELD created_at ON document TYPE datetime DEFAULT time::now();
        DEFINE FIELD updated_at ON document TYPE datetime DEFAULT time::now();
    ").await?;
    
    Ok(())
}
```

### 3. Insert Data (Type-Safe)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Entity {
    name: String,
    entity_type: String,
    description: String,
    embedding: Vec<f32>,
}

async fn insert_entity(
    db: &Surreal<Db>,
    entity: Entity,
) -> surrealdb::Result<()> {
    // Create with auto-generated ID
    let created: Vec<Entity> = db
        .create("entity")
        .content(entity)
        .await?;
    
    println!("Created entity: {:?}", created);
    
    Ok(())
}

// Alternative: specify custom ID
async fn insert_entity_with_id(
    db: &Surreal<Db>,
    id: &str,
    entity: Entity,
) -> surrealdb::Result<()> {
    let created: Option<Entity> = db
        .create(("entity", id))
        .content(entity)
        .await?;
    
    Ok(())
}
```

### 4. Query Data

```rust
#[derive(Debug, Deserialize)]
struct QueryResult {
    id: Thing,
    name: String,
    entity_type: String,
}

async fn query_entities(db: &Surreal<Db>) -> surrealdb::Result<Vec<QueryResult>> {
    // Simple query
    let entities: Vec<QueryResult> = db
        .query("SELECT * FROM entity WHERE entity_type = $type")
        .bind(("type", "person"))
        .await?
        .take(0)?;
    
    Ok(entities)
}

// Alternative: Raw SQL
async fn query_raw(db: &Surreal<Db>) -> surrealdb::Result<()> {
    let mut response = db
        .query("SELECT name, entity_type FROM entity LIMIT 10")
        .await?;
    
    let entities: Vec<QueryResult> = response.take(0)?;
    
    for entity in entities {
        println!("{:?}", entity);
    }
    
    Ok(())
}
```

### 5. Graph Relations

```rust
#[derive(Debug, Serialize)]
struct Relationship {
    description: String,
    keywords: Vec<String>,
    weight: f32,
}

async fn create_relationship(
    db: &Surreal<Db>,
    from: &str,
    to: &str,
    rel: Relationship,
) -> surrealdb::Result<()> {
    // Use RELATE statement for graph edges
    db.query("
        RELATE $from->uses->$to 
        CONTENT {
            description: $description,
            keywords: $keywords,
            weight: $weight
        }
    ")
    .bind(("from", format!("entity:{}", from)))
    .bind(("to", format!("entity:{}", to)))
    .bind(("description", rel.description))
    .bind(("keywords", rel.keywords))
    .bind(("weight", rel.weight))
    .await?;
    
    Ok(())
}

// Traverse graph
async fn get_related_entities(
    db: &Surreal<Db>,
    entity_id: &str,
) -> surrealdb::Result<Vec<Entity>> {
    let results: Vec<Entity> = db
        .query("
            SELECT ->uses->entity.* AS related
            FROM $entity
            FETCH related
        ")
        .bind(("entity", format!("entity:{}", entity_id)))
        .await?
        .take(0)?;
    
    Ok(results)
}
```

### 6. Vector Search

```rust
async fn vector_search(
    db: &Surreal<Db>,
    query_embedding: Vec<f32>,
    limit: usize,
) -> surrealdb::Result<Vec<Entity>> {
    // <|DIMENSION|> syntax for vector similarity
    let results: Vec<Entity> = db
        .query("
            SELECT * FROM entity
            WHERE embedding <|1536|> $query_vector
            LIMIT $limit
        ")
        .bind(("query_vector", query_embedding))
        .bind(("limit", limit))
        .await?
        .take(0)?;
    
    Ok(results)
}
```

### 7. Hybrid Query (Vector + Graph)

```rust
#[derive(Debug, Deserialize)]
struct HybridResult {
    entity: Entity,
    related: Vec<Entity>,
    similarity_score: f32,
}

async fn hybrid_search(
    db: &Surreal<Db>,
    query_embedding: Vec<f32>,
) -> surrealdb::Result<Vec<HybridResult>> {
    let results = db
        .query("
            SELECT 
                *,
                vector::similarity::cosine(embedding, $query) AS similarity_score,
                ->uses->entity.* AS related
            FROM entity
            WHERE embedding <|1536|> $query
            LIMIT 10
            FETCH related
        ")
        .bind(("query", query_embedding))
        .await?
        .take(0)?;
    
    Ok(results)
}
```

---

## Production Pattern: LightRAG Storage Adapter

### Complete Implementation

```rust
use surrealdb::{engine::remote::ws::Ws, Surreal, opt::auth::Root};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] surrealdb::Error),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Clone)]
pub struct SurrealStorage {
    db: Arc<Surreal<surrealdb::engine::any::Any>>,
}

impl SurrealStorage {
    pub async fn new(url: &str, namespace: &str, database: &str) -> Result<Self> {
        let db = Surreal::new::<Ws>(url).await?;
        
        db.signin(Root {
            username: "root",
            password: "root",
        }).await?;
        
        db.use_ns(namespace).use_db(database).await?;
        
        Ok(Self {
            db: Arc::new(db),
        })
    }
    
    pub async fn initialize_schema(&self) -> Result<()> {
        // Entity table
        self.db.query("
            DEFINE TABLE IF NOT EXISTS entity SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS name ON entity TYPE string;
            DEFINE FIELD IF NOT EXISTS entity_type ON entity TYPE string;
            DEFINE FIELD IF NOT EXISTS description ON entity TYPE string;
            DEFINE FIELD IF NOT EXISTS embedding ON entity TYPE array<float>;
            DEFINE FIELD IF NOT EXISTS source_id ON entity TYPE string;
            DEFINE INDEX IF NOT EXISTS entity_embedding_idx ON entity 
                FIELDS embedding MTREE DIMENSION 1536;
        ").await?;
        
        // Relationship table
        self.db.query("
            DEFINE TABLE IF NOT EXISTS relationship SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS in ON relationship TYPE record<entity>;
            DEFINE FIELD IF NOT EXISTS out ON relationship TYPE record<entity>;
            DEFINE FIELD IF NOT EXISTS description ON relationship TYPE string;
            DEFINE FIELD IF NOT EXISTS weight ON relationship TYPE float DEFAULT 1.0;
        ").await?;
        
        // Document table
        self.db.query("
            DEFINE TABLE IF NOT EXISTS document SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS content ON document TYPE string;
            DEFINE FIELD IF NOT EXISTS status ON document TYPE string;
            DEFINE FIELD IF NOT EXISTS track_id ON document TYPE string;
            DEFINE FIELD IF NOT EXISTS created_at ON document TYPE datetime DEFAULT time::now();
        ").await?;
        
        Ok(())
    }
    
    // Entity operations
    pub async fn insert_entity(&self, id: &str, entity: Entity) -> Result<Entity> {
        let created: Option<Entity> = self.db
            .create(("entity", id))
            .content(entity)
            .await?;
        
        created.ok_or_else(|| StorageError::NotFound("Failed to create entity".to_string()))
    }
    
    pub async fn get_entity(&self, id: &str) -> Result<Option<Entity>> {
        let entity: Option<Entity> = self.db
            .select(("entity", id))
            .await?;
        
        Ok(entity)
    }
    
    pub async fn update_entity(&self, id: &str, entity: Entity) -> Result<Entity> {
        let updated: Option<Entity> = self.db
            .update(("entity", id))
            .content(entity)
            .await?;
        
        updated.ok_or_else(|| StorageError::NotFound("Entity not found".to_string()))
    }
    
    // Vector search
    pub async fn search_entities(
        &self,
        query_embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<Entity>> {
        let results: Vec<Entity> = self.db
            .query("
                SELECT * FROM entity
                WHERE embedding <|1536|> $query
                LIMIT $limit
            ")
            .bind(("query", query_embedding))
            .bind(("limit", limit))
            .await?
            .take(0)?;
        
        Ok(results)
    }
    
    // Graph operations
    pub async fn create_relationship(
        &self,
        from: &str,
        to: &str,
        description: String,
        weight: f32,
    ) -> Result<()> {
        self.db.query("
            RELATE $from->connected_to->$to 
            SET description = $description, weight = $weight
        ")
        .bind(("from", format!("entity:{}", from)))
        .bind(("to", format!("entity:{}", to)))
        .bind(("description", description))
        .bind(("weight", weight))
        .await?;
        
        Ok(())
    }
    
    pub async fn get_related_entities(&self, entity_id: &str) -> Result<Vec<Entity>> {
        let results: Vec<Entity> = self.db
            .query("
                SELECT VALUE ->connected_to->entity 
                FROM $entity
            ")
            .bind(("entity", format!("entity:{}", entity_id)))
            .await?
            .take(0)?;
        
        Ok(results)
    }
    
    // Document operations
    pub async fn insert_document(&self, doc: Document) -> Result<Document> {
        let created: Vec<Document> = self.db
            .create("document")
            .content(doc)
            .await?;
        
        created.into_iter().next()
            .ok_or_else(|| StorageError::NotFound("Failed to create document".to_string()))
    }
    
    pub async fn get_documents_by_track_id(&self, track_id: &str) -> Result<Vec<Document>> {
        let docs: Vec<Document> = self.db
            .query("SELECT * FROM document WHERE track_id = $track_id")
            .bind(("track_id", track_id))
            .await?
            .take(0)?;
        
        Ok(docs)
    }
}

// Data types
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entity {
    pub name: String,
    pub entity_type: String,
    pub description: String,
    pub embedding: Vec<f32>,
    pub source_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Document {
    pub content: String,
    pub status: String,
    pub track_id: String,
}
```

---

## Best Practices (2025)

### Do's

✅ **Use SCHEMAFULL for production**
```sql
DEFINE TABLE entity SCHEMAFULL;
-- Ensures data consistency
```

✅ **Index vector fields**
```sql
DEFINE INDEX embedding_idx ON entity 
    FIELDS embedding MTREE DIMENSION 1536;
```

✅ **Use transactions for multi-step operations**
```sql
BEGIN TRANSACTION;
  CREATE entity:RUST ...;
  RELATE entity:RUST->uses->entity:TOKIO ...;
COMMIT TRANSACTION;
```

✅ **Leverage graph traversal**
```sql
-- Find all entities 2 hops away
SELECT * FROM entity:START->*->*;
```

✅ **Use parameterized queries**
```rust
db.query("SELECT * FROM entity WHERE name = $name")
  .bind(("name", entity_name))
```

### Don'ts

❌ **Don't skip namespaces/databases**
```rust
// Bad
let db = Surreal::new::<Ws>("localhost:8000").await?;
// Missing use_ns/use_db

// Good
db.use_ns("lightrag").use_db("production").await?;
```

❌ **Don't hardcode vector dimensions**
```rust
// Bad
WHERE embedding <|1536|> $query

// Good - use constant
const EMBEDDING_DIM: usize = 1536;
WHERE embedding <|{EMBEDDING_DIM}|> $query
```

❌ **Don't ignore connection pooling**
```rust
// Use Arc<Surreal> for sharing across tasks
let db = Arc::new(db);
```

---

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    async fn setup_test_db() -> SurrealStorage {
        let storage = SurrealStorage::new(
            "localhost:8000",
            "test",
            &format!("test_{}", uuid::Uuid::new_v4())
        ).await.unwrap();
        
        storage.initialize_schema().await.unwrap();
        storage
    }
    
    #[tokio::test]
    async fn test_insert_entity() {
        let storage = setup_test_db().await;
        
        let entity = Entity {
            name: "Rust".to_string(),
            entity_type: "language".to_string(),
            description: "Systems programming language".to_string(),
            embedding: vec![0.1; 1536],
            source_id: "test".to_string(),
        };
        
        let created = storage.insert_entity("RUST", entity).await.unwrap();
        assert_eq!(created.name, "Rust");
    }
    
    #[tokio::test]
    async fn test_vector_search() {
        let storage = setup_test_db().await;
        
        // Insert test entities
        // ... 
        
        let query = vec![0.1; 1536];
        let results = storage.search_entities(query, 10).await.unwrap();
        
        assert!(!results.is_empty());
    }
}
```

---

## Official Resources

- **Documentation**: https://surrealdb.com/docs
- **Rust SDK**: https://docs.rs/surrealdb/latest/surrealdb/
- **GitHub**: https://github.com/surrealdb/surrealdb
- **Discord**: https://discord.gg/surrealdb
- **Examples**: https://surrealdb.com/docs/sdk/rust

---

## Comparison: SurrealDB vs Traditional Stack

| Requirement | Traditional | SurrealDB |
|-------------|-------------|-----------|
| **Graph Storage** | Neo4j | Built-in |
| **Vector Search** | Qdrant/Pinecone | Built-in |
| **Documents** | MongoDB | Built-in |
| **KV Storage** | Redis | Built-in |
| **Total DBs** | 3-4 | 1 |
| **Query Language** | 3-4 different | SurrealQL |
| **Deployment** | Complex | Single binary |

---

**Last Updated**: December 20, 2025  
**Version**: 1.0
