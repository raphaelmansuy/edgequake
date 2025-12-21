# FalkorDB: Ultra-Fast Redis-Based Graph Database

**Version**: 4.14.10+  
**GitHub**: https://github.com/FalkorDB/FalkorDB (2.6k+ stars)  
**Purpose**: Redis module for low-latency graph database operations

---

## Overview

FalkorDB is a **Redis module** that transforms Redis into a graph database with **sub-millisecond query latency**. Built on GraphBLAS (sparse matrix operations), it's optimized for knowledge graphs in RAG applications.

### Why FalkorDB?

| Feature | Neo4j | PostgreSQL AGE | FalkorDB |
|---------|-------|----------------|----------|
| **Latency** | 10-50ms | 5-20ms | <1ms |
| **Query Language** | Cypher | OpenCypher | OpenCypher |
| **Deployment** | Standalone | PostgreSQL | Redis module |
| **Multi-Tenancy** | Complex | Manual | Built-in |
| **LLM-Optimized** | No | No | Yes |

**Use Case**: When you need **ultra-low latency** for real-time applications.

---

## Installation

### Docker

```bash
docker run -p 6379:6379 -p 3000:3000 \
  --rm -v ./data:/var/lib/falkordb/data \
  falkordb/falkordb:latest
```

### Redis Module

```bash
# Install FalkorDB module
wget https://github.com/FalkorDB/FalkorDB/releases/download/v4.14.10/falkordb.so
redis-server --loadmodule ./falkordb.so
```

---

## Quick Start

### Rust Integration

```toml
[dependencies]
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }
serde_json = "1"
```

```rust
use redis::{Client, Commands, AsyncCommands};

pub struct FalkorDBStorage {
    client: redis::Client,
    graph_name: String,
}

impl FalkorDBStorage {
    pub async fn new(redis_url: &str, graph_name: &str) -> Result<Self> {
        let client = Client::open(redis_url)?;
        Ok(Self {
            client,
            graph_name: graph_name.to_string(),
        })
    }

    pub async fn add_entity(
        &self,
        name: &str,
        entity_type: &str,
    ) -> Result<()> {
        let mut conn = self.client.get_async_connection().await?;
        
        let cypher = format!(
            "CREATE (e:Entity {{name: '{}', type: '{}'}})",
            name, entity_type
        );
        
        let _: () = redis::cmd("GRAPH.QUERY")
            .arg(&self.graph_name)
            .arg(cypher)
            .query_async(&mut conn)
            .await?;
        
        Ok(())
    }

    pub async fn find_related(
        &self,
        entity_name: &str,
    ) -> Result<Vec<String>> {
        let mut conn = self.client.get_async_connection().await?;
        
        let cypher = format!(
            "MATCH (a:Entity {{name: '{}'}})-[r]->(b) RETURN b.name",
            entity_name
        );
        
        let result: Vec<String> = redis::cmd("GRAPH.QUERY")
            .arg(&self.graph_name)
            .arg(cypher)
            .query_async(&mut conn)
            .await?;
        
        Ok(result)
    }
}
```

---

## Resources

- [FalkorDB Docs](https://docs.falkordb.com/)
- [GitHub](https://github.com/FalkorDB/FalkorDB)

---

**Status**: ✅ Production Ready for Low-Latency Use Cases
