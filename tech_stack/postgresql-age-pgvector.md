# PostgreSQL AGE + pgvector: Graph Database with Vector Search

**AGE Version**: 1.5.0+ (PostgreSQL 16+)  
**pgvector Version**: 0.7.0+  
**Purpose**: Unified graph, vector, and relational database for LightRAG

---

## Overview

Apache AGE (A Graph Extension) + pgvector provides a **unified database solution** combining:
- **Graph Database** (OpenCypher queries)
- **Vector Search** (HNSW indexing)
- **Relational Storage** (PostgreSQL ACID guarantees)
- **JSON Documents** (JSONB support)

### Why PostgreSQL AGE + pgvector?

| Feature | Neo4j + Qdrant | PostgreSQL AGE + pgvector |
|---------|----------------|---------------------------|
| **Databases** | 2 separate | 1 unified |
| **Query Language** | Cypher + Custom | OpenCypher + SQL |
| **ACID** | Per database | Global transactions |
| **Ecosystem** | Medium | Massive (PostgreSQL) |
| **Ops Complexity** | High | Low |
| **Cost** | High (licensing) | Low (open source) |

**Verdict**: AGE + pgvector = **one database** to rule them all.

---

## Installation

### Docker (Recommended)

```bash
# Pull image with AGE and pgvector
docker pull apache/age-postgres:latest

# Run PostgreSQL with AGE
docker run -d \
  --name lightrag-postgres \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_DB=lightrag \
  -p 5432:5432 \
  apache/age-postgres:latest
```

### Manual Installation

```bash
# Install PostgreSQL 16+
sudo apt install postgresql-16

# Install AGE extension
git clone https://github.com/apache/age.git
cd age
make install

# Install pgvector
git clone https://github.com/pgvector/pgvector.git
cd pgvector
make install
```

### Enable Extensions

```sql
CREATE EXTENSION IF NOT EXISTS age;
CREATE EXTENSION IF NOT EXISTS vector;

-- Load AGE into search path
SET search_path = ag_catalog, "$user", public;
```

---

## Core Concepts

### 1. Graph Model (AGE)

AGE implements the **Property Graph Model**:

- **Vertices (Nodes)**: Entities with properties
- **Edges (Relationships)**: Connections with properties
- **Labels**: Node/edge types
- **Properties**: Key-value pairs (JSONB)

```sql
-- Create graph
SELECT * FROM ag_catalog.create_graph('knowledge_graph');

-- Add node
SELECT * FROM ag_catalog.cypher('knowledge_graph', $$
    CREATE (e:Entity {
        id: '123',
        name: 'Albert Einstein',
        type: 'person',
        description: 'Physicist'
    })
    RETURN e
$$) AS (entity ag_catalog.agtype);

-- Add relationship
SELECT * FROM ag_catalog.cypher('knowledge_graph', $$
    MATCH (a:Entity {name: 'Albert Einstein'})
    MATCH (b:Entity {name: 'Theory of Relativity'})
    CREATE (a)-[r:DEVELOPED {year: 1915}]->(b)
    RETURN r
$$) AS (rel ag_catalog.agtype);
```

### 2. Vector Search (pgvector)

pgvector adds vector similarity search to PostgreSQL:

```sql
-- Create table with vector column
CREATE TABLE embeddings (
    id UUID PRIMARY KEY,
    entity_id TEXT,
    content TEXT,
    embedding VECTOR(1536)  -- OpenAI embedding dimension
);

-- Create HNSW index for fast search
CREATE INDEX ON embeddings 
USING hnsw (embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);

-- Vector similarity search
SELECT id, content,
       1 - (embedding <=> $1) AS similarity
FROM embeddings
ORDER BY embedding <=> $1
LIMIT 10;
```

### 3. Hybrid Queries (Graph + Vector)

```sql
-- Find entities related to vector-similar content
WITH similar_chunks AS (
    SELECT entity_id, content,
           1 - (embedding <=> $1) AS similarity
    FROM embeddings
    ORDER BY embedding <=> $1
    LIMIT 5
)
SELECT 
    e.name,
    e.type,
    r.relationship_type,
    e2.name AS related_entity,
    sc.content,
    sc.similarity
FROM similar_chunks sc
JOIN ag_catalog.entity e ON e.id = sc.entity_id
JOIN ag_catalog.relationship r ON r.source_id = e.id
JOIN ag_catalog.entity e2 ON e2.id = r.target_id;
```

---

## Rust Integration

### Dependencies

```toml
[dependencies]
sqlx = { version = "0.7", features = ["runtime-tokio-native-tls", "postgres", "uuid", "chrono", "json"] }
pgvector = { version = "0.4", features = ["sqlx"] }
uuid = { version = "1", features = ["v4", "serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### Storage Implementation

```rust
use sqlx::{PgPool, postgres::PgPoolOptions};
use pgvector::Vector;
use uuid::Uuid;

pub struct AGEStorage {
    pool: PgPool,
    graph_name: String,
}

impl AGEStorage {
    pub async fn new(database_url: &str, graph_name: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(50)
            .connect(database_url)
            .await?;
        
        // Enable extensions
        sqlx::query("CREATE EXTENSION IF NOT EXISTS age;")
            .execute(&pool)
            .await?;
        sqlx::query("CREATE EXTENSION IF NOT EXISTS vector;")
            .execute(&pool)
            .await?;
        
        // Create graph
        sqlx::query(&format!(
            "SELECT * FROM ag_catalog.create_graph_if_not_exists('{}');",
            graph_name
        ))
        .execute(&pool)
        .await?;
        
        Ok(Self {
            pool,
            graph_name: graph_name.to_string(),
        })
    }

    pub async fn insert_entity(
        &self,
        name: &str,
        entity_type: &str,
        description: Option<&str>,
        embedding: &[f32],
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let vector = Vector::from(embedding.to_vec());
        
        // Insert into graph (AGE)
        let cypher = format!(
            "CREATE (e:Entity {{id: '{}', name: '{}', type: '{}', description: '{}'}}) RETURN e",
            id, name, entity_type, description.unwrap_or("")
        );
        
        sqlx::query(&format!(
            "SELECT * FROM ag_catalog.cypher('{}', $${}$$) AS (entity ag_catalog.agtype);",
            self.graph_name, cypher
        ))
        .execute(&self.pool)
        .await?;
        
        // Insert embedding (pgvector)
        sqlx::query(
            "INSERT INTO embeddings (id, entity_id, content, embedding)
             VALUES ($1, $2, $3, $4)"
        )
        .bind(Uuid::new_v4())
        .bind(&id)
        .bind(name)
        .bind(vector)
        .execute(&self.pool)
        .await?;
        
        Ok(id)
    }

    pub async fn search_similar(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<SearchResult>> {
        let vector = Vector::from(query_embedding.to_vec());
        
        sqlx::query_as::<_, SearchResult>(
            "SELECT entity_id, content, 
                    1 - (embedding <=> $1) AS similarity
             FROM embeddings
             ORDER BY embedding <=> $1
             LIMIT $2"
        )
        .bind(vector)
        .bind(top_k as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_related_entities(
        &self,
        entity_id: &str,
        max_depth: usize,
    ) -> Result<Vec<Entity>> {
        let cypher = format!(
            "MATCH (a:Entity {{id: '{}'}})-[r*1..{}]->(b:Entity)
             RETURN DISTINCT b",
            entity_id, max_depth
        );
        
        let rows = sqlx::query(&format!(
            "SELECT * FROM ag_catalog.cypher('{}', $${}$$) AS (entity ag_catalog.agtype);",
            self.graph_name, cypher
        ))
        .fetch_all(&self.pool)
        .await?;
        
        // Parse AGE agtype to Entity
        rows.into_iter()
            .map(|row| parse_agtype_to_entity(&row))
            .collect()
    }
}
```

---

## Production Patterns

### Schema Design

```sql
-- AGE graph schema
SELECT * FROM ag_catalog.create_graph('knowledge_graph');

-- Create node labels
SELECT * FROM ag_catalog.create_vlabel('knowledge_graph', 'Entity');
SELECT * FROM ag_catalog.create_vlabel('knowledge_graph', 'Document');

-- Create edge labels
SELECT * FROM ag_catalog.create_elabel('knowledge_graph', 'RELATES_TO');
SELECT * FROM ag_catalog.create_elabel('knowledge_graph', 'MENTIONS');

-- Vector table with indexes
CREATE TABLE embeddings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id TEXT NOT NULL,
    content TEXT NOT NULL,
    embedding VECTOR(1536) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_embeddings_entity ON embeddings(entity_id);
CREATE INDEX idx_embeddings_hnsw ON embeddings 
USING hnsw (embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);
```

### Performance Tuning

```sql
-- Increase work memory for large graphs
SET work_mem = '256MB';

-- Parallel query execution
SET max_parallel_workers_per_gather = 4;

-- HNSW index tuning
ALTER INDEX idx_embeddings_hnsw SET (ef_search = 40);

-- Connection pooling
ALTER SYSTEM SET max_connections = 200;
```

---

## Best Practices

1. **Use HNSW for vectors** (not IVFFlat) - better recall
2. **Batch inserts** for performance
3. **Separate read/write pools**
4. **Monitor query performance** with `EXPLAIN ANALYZE`
5. **Regular VACUUM** for AGE tables

---

## Resources

- [Apache AGE Docs](https://age.apache.org/age-manual/master/intro/overview.html)
- [pgvector GitHub](https://github.com/pgvector/pgvector)
- [PostgreSQL Docs](https://www.postgresql.org/docs/current/)

---

**Last Updated**: December 20, 2025  
**Status**: ✅ Production Ready
