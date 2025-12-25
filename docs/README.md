# EdgeQuake Documentation

**Version 2.0** | Graph-Enhanced Retrieval-Augmented Generation in Rust

> High-Performance Graph-Enhanced RAG System in Rust

> **New Feature**: Multi-tenancy support with namespace isolation, PostgreSQL with pgvector and Apache AGE.

---

## Documentation Overview

| Document                                                   | Description                                       |
| ---------------------------------------------------------- | ------------------------------------------------- |
| [Quick Start](0001-quick-start.md)                         | Get up and running in 5 minutes                   |
| [Architecture Overview](0002-architecture-overview.md)     | System design, crate structure, and core concepts |
| [API Reference](0003-api-reference.md)                     | Complete REST API documentation                   |
| [Storage Backends](0004-storage-backends.md)               | Configure KV, vector, and graph storage           |
| [LLM Integration](0005-llm-integration.md)                 | LLM providers and embedding models                |
| [Deployment Guide](0006-deployment-guide.md)               | Docker, Kubernetes, and production setup          |
| [Configuration Reference](0007-configuration-reference.md) | All environment variables and options             |
| [Multi-Tenancy](0008-multi-tenancy.md)                     | Tenant isolation and namespace management         |
| [Algorithms Reference](0009-algorithms-reference.md)       | Detailed pipeline and query algorithms            |
| [Production LLM](production-llm-integration.md)            | Real LLM provider integration guide               |

---

## Quick Links

### Getting Started

```bash
# Clone and build
git clone https://github.com/your-org/edgequake.git
cd edgequake/edgequake
cargo build --release

# Start server (default: http://0.0.0.0:8080)
export OPENAI_API_KEY=sk-xxx
./target/release/edgequake
```

### Rust Usage

```rust
use edgequake_core::{EdgeQuake, EdgeQuakeConfig, QueryParams, QueryMode};
use edgequake_llm::OpenAIProvider;
use edgequake_storage::adapters::memory::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup providers
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let provider = Arc::new(OpenAIProvider::new(api_key));

    // Setup in-memory storage
    let kv = Arc::new(MemoryKVStorage::new());
    let vector = Arc::new(MemoryVectorStorage::new(1536));
    let graph = Arc::new(MemoryGraphStorage::new());

    // Create EdgeQuake instance
    let config = EdgeQuakeConfig::new().with_namespace("demo");
    let mut eq = EdgeQuake::new(config)
        .with_storage_backends(kv, vector, graph)
        .with_providers(provider.clone(), provider);

    eq.initialize().await?;

    // Insert document
    let result = eq.insert("Your document text...", Some("doc-001")).await?;
    println!("Extracted {} entities", result.entities_extracted);

    // Query
    let params = QueryParams::new().with_mode(QueryMode::Hybrid);
    let response = eq.query("Your question?", Some(params)).await?;
    println!("{}", response.response);

    Ok(())
}
```

### REST API

```bash
# Health check
curl http://localhost:8080/health

# Insert document
curl -X POST http://localhost:8080/api/v1/documents/text \
  -H "Content-Type: application/json" \
  -d '{"text": "Document content..."}'

# Query
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "Your question?", "mode": "hybrid"}'
```

### Docker

```bash
cd edgequake/docker
docker compose up -d
```

---

## Query Modes

| Mode     | Description                       | Best For          |
| -------- | --------------------------------- | ----------------- |
| `naive`  | Basic vector similarity search    | Simple lookups    |
| `local`  | Entity-focused retrieval          | Specific facts    |
| `global` | High-level community summaries    | Broad questions   |
| `hybrid` | Combined local + global (default) | Balanced queries  |
| `mix`    | Full KG + vector integration      | Complex reasoning |
| `bypass` | Skip RAG, direct LLM              | Testing/fallback  |

> **Code Reference**: See [types/query.rs](../edgequake/crates/edgequake-core/src/types/query.rs#L4-L24)

---

## Deployment Options

| Option         | Use Case           | Guide                                                              |
| -------------- | ------------------ | ------------------------------------------------------------------ |
| **Local**      | Development        | `cargo build && cargo run`                                         |
| **Docker**     | Staging/Production | [Docker Guide](0006-deployment-guide.md#2-docker-deployment)       |
| **Kubernetes** | Production/Scale   | [K8s Guide](0006-deployment-guide.md#3-kubernetes-deployment-helm) |

### Storage Topology

| Environment | KV         | Vector   | Graph      |
| ----------- | ---------- | -------- | ---------- |
| Development | Memory     | Memory   | Memory     |
| Production  | PostgreSQL | pgvector | Apache AGE |

---

## Configuration Quick Reference

### Essential Environment Variables

```bash
# LLM (required for production)
OPENAI_API_KEY=sk-xxx

# Server
HOST=0.0.0.0      # Bind address
PORT=8080         # Server port
WORKER_THREADS=4  # Tokio worker threads

# Database (production)
DATABASE_URL=postgresql://user:pass@localhost:5432/edgequake
```

> **Code Reference**: See [main.rs](../edgequake/src/main.rs#L69-L73) for environment variable loading.

See [Configuration Reference](0007-configuration-reference.md) for all options.

---

## Project Structure

```
edgequake/
├── Cargo.toml          # Workspace manifest
├── src/main.rs         # API server binary
├── crates/
│   ├── edgequake-core/     # Orchestrator, types, config
│   ├── edgequake-api/      # Axum REST API routes
│   ├── edgequake-llm/      # LLM providers (OpenAI, Mock)
│   ├── edgequake-storage/  # Storage adapters (Memory, PG)
│   ├── edgequake-pipeline/ # Document processing
│   └── edgequake-query/    # Query engine
├── examples/           # Working examples
├── tests/              # Integration tests
└── docker/             # Docker configuration
```

---

## Document Index

1. **[Quick Start Guide](0001-quick-start.md)** - Build, run, basic usage
2. **[Architecture Overview](0002-architecture-overview.md)** - Crate structure, data flow
3. **[API Reference](0003-api-reference.md)** - REST endpoints documentation
4. **[Storage Backends](0004-storage-backends.md)** - Memory, PostgreSQL adapters
5. **[LLM Integration](0005-llm-integration.md)** - OpenAI, Mock providers
6. **[Deployment Guide](0006-deployment-guide.md)** - Docker, K8s, production
7. **[Configuration Reference](0007-configuration-reference.md)** - Environment variables
8. **[Multi-Tenancy Guide](0008-multi-tenancy.md)** - Namespace isolation
9. **[Production LLM](production-llm-integration.md)** - Real LLM provider guide

---

_Built with Rust 🦀 for performance and reliability_
