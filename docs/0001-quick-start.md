# EdgeQuake Quick Start Guide

> **Implements**: [FEAT0001](features.md#feat0001) Document Ingestion, [FEAT0002](features.md#feat0002) Knowledge Graph Query
>
> **Code Reference**: Main implementation in [edgequake/crates/edgequake-core/](../edgequake/crates/edgequake-core/)

Get up and running with EdgeQuake in **5 minutes**.

## Prerequisites Checklist

Before you begin, ensure you have:

- [ ] **Rust 1.78+** - Install via [rustup](https://rustup.rs/): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- [ ] **Node.js 20+** - For WebUI (optional): [nodejs.org](https://nodejs.org/)
- [ ] **PostgreSQL 15+** - For production (optional): [postgresql.org](https://postgresql.org/)
- [ ] **OpenAI API key** - For production LLM: [platform.openai.com](https://platform.openai.com/)

> **Note**: For development/testing, no API key is needed - EdgeQuake uses a mock provider automatically.

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

// MIX: Weighted combination of naive and graph-based search
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

> **Implements**: [FEAT0003](features.md#feat0003) REST API, [UC0001](use_cases.md#uc0001) Document Upload
>
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

> **Implements**: [FEAT0010](features.md#feat0010) WebUI Dashboard, [UC0005](use_cases.md#uc0005) Visual Graph Exploration

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

## More Examples

Explore these examples in the `edgequake/examples/` directory:

- **[Graph Exploration](../edgequake/examples/graph_exploration.rs)**: Advanced graph traversal and entity analysis.
- **[Streaming Query](../edgequake/examples/streaming_query.rs)**: Real-time response streaming for interactive applications.
- **[Multi-Tenant](../edgequake/examples/multi_tenant.rs)**: Setting up isolated namespaces for different users.

---

## Next Steps

Once you have EdgeQuake running, explore these guides:

| Your Goal | Next Document |
|-----------|---------------|
| Understand the architecture | [Architecture Overview](0002-architecture-overview.md) |
| Integrate via REST API | [API Reference](0003-api-reference.md) |
| Configure storage for production | [Storage Backends](0004-storage-backends.md) |
| Optimize LLM costs | [LLM Integration](0005-llm-integration.md) |
| Deploy to production | [Deployment Guide](0006-deployment-guide.md) |
| Set up multi-tenant isolation | [Multi-Tenancy](0008-multi-tenancy.md) |

> **Implements**: [UC0001](use_cases.md#uc0001) Document Upload, [UC0002](use_cases.md#uc0002) Knowledge Graph Query

---

## Troubleshooting

### Common Issues and Solutions

| Issue | Cause | Solution |
|-------|-------|----------|
| `OPENAI_API_KEY not set` | Environment variable missing | `export OPENAI_API_KEY=sk-xxx` |
| Build fails with "rustc version" | Rust too old | `rustup update && rustup default stable` |
| Connection refused on port 8080 | Server not running | `./target/release/edgequake` in another terminal |
| WebUI shows "Network Error" | CORS or wrong URL | Set `NEXT_PUBLIC_API_URL=http://localhost:8080` |
| Slow processing | Large documents | Use `async_processing: true` for docs > 10KB |
| "No entities extracted" | Text too short | Minimum ~100 words recommended per document |
| PostgreSQL connection failed | Wrong credentials | Verify `DATABASE_URL` format and DB exists |
| Out of memory | Large batch | Reduce batch size or increase system memory |

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=edgequake=debug ./target/release/edgequake

# Trace all HTTP requests
RUST_LOG=edgequake_api=trace ./target/release/edgequake

# Check LLM provider status
curl http://localhost:8080/api/v1/health/providers
```

### Verify Your Installation

```bash
# 1. Check Rust version (should be 1.78+)
rustc --version

# 2. Build the project
cargo build --release

# 3. Run tests (should pass without API key)
cargo test

# 4. Start server and verify health
./target/release/edgequake &
curl http://localhost:8080/health
# Expected: {"status":"healthy","version":"2.0.0"}

# 5. Test document insertion
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{"content": "Test document", "title": "Test"}'
# Expected: 201 Created with document ID
```

---

**Need Help?**

- 📖 [Full Documentation](README.md)
- 🐛 [GitHub Issues](https://github.com/raphaelmansuy/edgequake/issues)
- 💬 [Discussions](https://github.com/raphaelmansuy/edgequake/discussions)

> **See Also**: [Deployment Troubleshooting](0006-deployment-guide.md#troubleshooting) for production issues
