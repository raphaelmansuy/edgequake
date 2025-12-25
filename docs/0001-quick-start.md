# EdgeQuake Quick Start Guide

> **Code Reference**: Main implementation in [edgequake/crates/edgequake-core/](../edgequake/crates/edgequake-core/)

Get up and running with EdgeQuake in 5 minutes.

## What is EdgeQuake?

EdgeQuake is a high-performance **Graph-Enhanced Retrieval-Augmented Generation (RAG)** system implemented in Rust, combining knowledge graphs with vector search for superior context retrieval. It features a Next.js WebUI for visual exploration and management.

```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│   Document   │───▶│  EdgeQuake   │───▶│   Knowledge  │
│    Input     │    │   Pipeline   │    │    Graph     │
└──────────────┘    └──────────────┘    └──────────────┘
                           │                    │
                           ▼                    ▼
                    ┌──────────────┐    ┌──────────────┐
                    │   Vector     │    │   Entity     │
                    │   Chunks     │    │   Relations  │
                    └──────────────┘    └──────────────┘
                           │                    │
                           └────────┬───────────┘
                                    ▼
                           ┌──────────────┐
                           │   Hybrid     │
                           │   Query      │
                           └──────────────┘
```

---

## Installation

### Prerequisites

- Rust 1.78+ (via [rustup](https://rustup.rs/))
- Node.js 20+ and npm (for WebUI)
- PostgreSQL 15+ (optional, for production)
- OpenAI API key (or compatible provider)

### Option 1: Build from Source

```bash
# Clone the repository
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake

# Build the Rust backend
cargo build --release

# Build the WebUI
cd edgequake_webui
npm install
npm run build
cd ..
```

### Option 2: Development Setup

```bash
# Clone repository
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake

# Run tests (uses mock provider - no API key needed)
cargo test

# Run with real OpenAI (requires API key)
export OPENAI_API_KEY="sk-your-key"
cargo test
```

---

## Quick Start (Rust API)

### 1. Basic Usage

> **Code Reference**: See [edgequake/crates/edgequake-core/src/orchestrator.rs](../edgequake/crates/edgequake-core/src/orchestrator.rs) for `EdgeQuake` implementation

```rust
use edgequake_core::{EdgeQuake, EdgeQuakeConfig, StorageBackend, StorageConfig};
use edgequake_llm::OpenAIProvider;
use edgequake_storage::{MemoryKVStorage, MemoryVectorStorage, MemoryGraphStorage};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create LLM provider
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let provider = Arc::new(
        OpenAIProvider::new(&api_key)
            .with_model("gpt-4o-mini")
            .with_embedding_model("text-embedding-3-small")
    );

    // Create storage backends
    let namespace = "default";
    let kv = Arc::new(MemoryKVStorage::new(namespace));
    let vector = Arc::new(MemoryVectorStorage::new(namespace, 1536));
    let graph = Arc::new(MemoryGraphStorage::new(namespace));

    // Initialize EdgeQuake
    let config = EdgeQuakeConfig::new()
        .with_namespace(namespace)
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });

    let mut eq = EdgeQuake::new(config)
        .with_providers(provider.clone(), provider.clone())
        .with_storage_backends(kv, vector, graph);

    eq.initialize().await?;

    // Insert a document
    let result = eq.insert(
        "Marie Curie was a physicist who discovered radium.
         She was born in Poland and later moved to France.
         She won the Nobel Prize in Physics in 1903.",
        None  // Auto-generate document ID
    ).await?;

    println!("Inserted: {} entities, {} relationships",
        result.entities_extracted, result.relationships_extracted);

    // Query the knowledge graph
    let response = eq.query("What did Marie Curie discover?", None).await?;
    println!("Answer: {}", response.response);

    Ok(())
}
```

### 2. Query Modes Explained

> **Code Reference**: See [edgequake/crates/edgequake-core/src/types/query.rs](../edgequake/crates/edgequake-core/src/types/query.rs) for `QueryMode` enum

EdgeQuake supports 6 query modes, each optimized for different use cases:

```rust
use edgequake_core::{QueryMode, QueryParams};

// NAIVE: Direct vector similarity search (fastest)
// Best for: Simple factual lookups
let params = QueryParams::new().with_mode(QueryMode::Naive);
let response = eq.query("question", Some(params)).await?;

// LOCAL: Entity-centric search with local neighborhood
// Best for: Questions about specific entities
let params = QueryParams::new().with_mode(QueryMode::Local);
let response = eq.query("question", Some(params)).await?;

// GLOBAL: Community-based search using graph clusters
// Best for: Broad topic questions
let params = QueryParams::new().with_mode(QueryMode::Global);
let response = eq.query("question", Some(params)).await?;

// HYBRID: Combines local and global approaches (recommended, default)
// Best for: General-purpose queries
let response = eq.query("question", None).await?;  // Default is Hybrid

// MIX: Adaptive combination based on query type
// Best for: Maximum flexibility
let params = QueryParams::new().with_mode(QueryMode::Mix);
let response = eq.query("question", Some(params)).await?;

// BYPASS: Skip retrieval, direct LLM query
// Best for: General questions without RAG
let params = QueryParams::new().with_mode(QueryMode::Bypass);
let response = eq.query("question", Some(params)).await?;
```

---

## Quick Start (REST API)

> **Code Reference**: See [edgequake/crates/edgequake-api/src/routes.rs](../edgequake/crates/edgequake-api/src/routes.rs) for API routes

### 1. Start the Server

```bash
# Set environment variables
export OPENAI_API_KEY="sk-your-key"
export HOST=0.0.0.0  # Optional, defaults to 0.0.0.0
export PORT=8080     # Optional, defaults to 8080

# Start the API server
cargo run --release

# Server runs at http://localhost:8080
# Swagger UI available at http://localhost:8080/swagger-ui
```

### 2. Insert Documents

```bash
# Insert text document
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Albert Einstein developed the theory of relativity.",
    "title": "Einstein",
    "async_processing": false
  }'

# Upload file
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@document.txt"

# Batch upload
curl -X POST http://localhost:8080/api/v1/documents/upload/batch \
  -F "files=@doc1.txt" \
  -F "files=@doc2.txt"
```

### 3. Query

```bash
# Query with hybrid mode
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What is the theory of relativity?",
    "mode": "hybrid"
  }'

# Streaming query (SSE)
curl -X POST http://localhost:8080/api/v1/query/stream \
  -H "Content-Type: application/json" \
  -d '{
    "query": "Explain quantum mechanics",
    "mode": "hybrid"
  }'
```

### 4. Explore Knowledge Graph

```bash
# Get graph overview
curl http://localhost:8080/api/v1/graph

# Get specific node
curl http://localhost:8080/api/v1/graph/nodes/ALBERT_EINSTEIN

# Search labels
curl "http://localhost:8080/api/v1/graph/labels/search?q=einstein"
```

---

## Quick Start (WebUI)

### 1. Start the WebUI

```bash
cd edgequake_webui

# Install dependencies
npm install

# Set API URL
export NEXT_PUBLIC_API_URL=http://localhost:8080

# Start development server
npm run dev

# WebUI runs at http://localhost:3000
```

### 2. WebUI Features

| Feature       | Description                                |
| ------------- | ------------------------------------------ |
| **Dashboard** | System overview, stats, quick actions      |
| **Documents** | Upload, manage, track processing status    |
| **Query**     | Interactive query interface with streaming |
| **Graph**     | Visual knowledge graph exploration         |
| **Settings**  | Configure API, themes, language            |

---

## Configuration

> **Code Reference**: See [edgequake/crates/edgequake-core/src/orchestrator.rs](../edgequake/crates/edgequake-core/src/orchestrator.rs) for `EdgeQuakeConfig`

### Environment Variables

```bash
# Required
OPENAI_API_KEY=sk-xxx              # OpenAI API key

# Server Configuration (see edgequake/src/main.rs)
HOST=0.0.0.0                       # Server host (default: 0.0.0.0)
PORT=8080                          # Server port (default: 8080)
WORKER_THREADS=4                   # Number of worker threads (default: CPU cores)

# Storage (for PostgreSQL)
DATABASE_URL=postgres://localhost/edgequake
```

### Rust Configuration

```rust
use edgequake_core::{EdgeQuakeConfig, StorageBackend, StorageConfig};

// Create with defaults
let config = EdgeQuakeConfig::default();

// Or configure via builder pattern
let config = EdgeQuakeConfig::new()
    .with_namespace("my-workspace")
    .with_llm_model("gpt-4o-mini")
    .with_embedding_model("text-embedding-3-small", 1536)
    .with_chunk_config(1200, 100);

// For PostgreSQL storage
let config = EdgeQuakeConfig::new()
    .with_postgres("postgres://user:pass@localhost/edgequake");
```

---

## Storage Backends

> **Code Reference**: See [edgequake/crates/edgequake-storage/src/lib.rs](../edgequake/crates/edgequake-storage/src/lib.rs) for storage traits and adapters

### Memory Storage (Development)

```rust
use edgequake_storage::{MemoryKVStorage, MemoryVectorStorage, MemoryGraphStorage};
use std::sync::Arc;

let namespace = "my-workspace";
let kv = Arc::new(MemoryKVStorage::new(namespace));
let vector = Arc::new(MemoryVectorStorage::new(namespace, 1536));  // 1536 = embedding dim
let graph = Arc::new(MemoryGraphStorage::new(namespace));
```

### PostgreSQL Storage (Production)

> **Code Reference**: See [edgequake/crates/edgequake-storage/src/adapters/postgres/](../edgequake/crates/edgequake-storage/src/adapters/postgres/) for PostgreSQL adapters

```rust
use edgequake_storage::{PostgresKVStorage, PgVectorStorage, PostgresAGEGraphStorage, PostgresConfig};
use std::sync::Arc;

// Create configuration
let config = PostgresConfig {
    host: "localhost".to_string(),
    port: 5432,
    database: "edgequake".to_string(),
    user: "postgres".to_string(),
    password: "password".to_string(),
    namespace: "my-namespace".to_string(),
    ..Default::default()
};

// Create individual storage adapters (each manages its own connection pool)
let kv = Arc::new(PostgresKVStorage::new(config.clone()));
let vector = Arc::new(PgVectorStorage::new(config.clone()));
let graph = Arc::new(PostgresAGEGraphStorage::new(config));
```

---

## Run Production Example

```bash
# Set API key
export OPENAI_API_KEY="sk-your-actual-key"

# Run the production pipeline example
cargo run --example production_pipeline

# Expected output:
# 🚀 EdgeQuake Production Pipeline Example
# ==========================================
# ✓ API key found
# ✓ LLM Provider: openai (model: gpt-4o-mini)
# ✓ Embedding Provider: openai (model: text-embedding-3-small)
# ✓ Storage backends ready
# ✓ EdgeQuake initialized
# 📄 Ingesting documents...
# 📊 Processing Complete!
```

---

## Verify Installation

```bash
# Run all tests (uses mock provider)
cargo test

# Run with real OpenAI provider
export OPENAI_API_KEY="sk-xxx"
cargo test -- --nocapture

# Run specific E2E test
cargo test --package edgequake-core --test e2e_pipeline

# Lint and format
cargo clippy
cargo fmt --check
```

---

## Next Steps

1. **[Architecture Overview](0002-architecture-overview.md)** - Understand EdgeQuake internals
2. **[API Reference](0003-api-reference.md)** - Complete REST API documentation
3. **[Storage Backends](0004-storage-backends.md)** - Configure production storage
4. **[LLM Integration](0005-llm-integration.md)** - Configure LLM providers
5. **[Deployment Guide](0006-deployment-guide.md)** - Deploy to production
6. **[Configuration Reference](0007-configuration-reference.md)** - All config options

---

## Troubleshooting

| Issue                           | Solution                                       |
| ------------------------------- | ---------------------------------------------- |
| `OPENAI_API_KEY not set`        | Export API key: `export OPENAI_API_KEY=sk-xxx` |
| Build fails                     | Ensure Rust 1.78+: `rustup update`             |
| Connection refused on port 8080 | Check if server is running                     |
| WebUI shows "Network Error"     | Set `NEXT_PUBLIC_API_URL` correctly            |
| Slow processing                 | Use async_processing: true for large docs      |

---

**Need Help?**

- GitHub Issues: https://github.com/raphaelmansuy/edgequake/issues
- Documentation: [docs/](.)
