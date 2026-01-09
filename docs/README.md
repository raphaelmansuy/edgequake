# EdgeQuake Documentation

**Version 2.0** | Graph-Enhanced Retrieval-Augmented Generation in Rust

[![Rust](https://img.shields.io/badge/Rust-1.78+-orange.svg)](https://rust-lang.org)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](../LICENSE)
[![Tests](https://img.shields.io/badge/Tests-2120%2B%20passing-green.svg)](../edgequake/crates/)

> **EdgeQuake** is a high-performance Graph-Enhanced RAG system that combines knowledge graphs with vector search for superior context retrieval and question answering.

---

## 🚀 Start Here

| Your Goal | Start With |
|-----------|------------|
| **Get running in 5 minutes** | [Quick Start Guide](0001-quick-start.md) |
| **Understand how it works** | [Architecture Overview](0002-architecture-overview.md) |
| **Integrate via REST API** | [API Reference](0003-api-reference.md) |
| **Deploy to production** | [Deployment Guide](0006-deployment-guide.md) |

---

## 📚 Documentation Overview

### Core Registries (Traceability)

| Document | Description | Use When |
|----------|-------------|----------|
| [**Features Registry**](features.md) | Central registry of all features (FEAT0001-XXXX) | Understanding system capabilities |
| [**Business Rules**](business_rules.md) | All business rules enforced (BR0001-XXXX) | Validating system behavior |
| [**Use Cases**](use_cases.md) | Complete use case catalog (UC0001-XXXX) | Understanding user journeys |

### Technical Documentation (Numbered Guides)

| # | Document | Description | Audience |
|---|----------|-------------|----------|
| 01 | [Quick Start](0001-quick-start.md) | Get up and running in 5 minutes | All developers |
| 02 | [Architecture Overview](0002-architecture-overview.md) | System design, crate structure, data flow | Architects, senior devs |
| 03 | [API Reference](0003-api-reference.md) | Complete REST API documentation (1700+ lines) | API consumers |
| 04 | [Storage Backends](0004-storage-backends.md) | Configure KV, vector, and graph storage | DevOps, backend devs |
| 05 | [LLM Integration](0005-llm-integration.md) | LLM providers, embeddings, cost optimization | ML engineers |
| 06 | [Deployment Guide](0006-deployment-guide.md) | Docker, Kubernetes, and production setup | DevOps, SREs |
| 07 | [Configuration Reference](0007-configuration-reference.md) | All environment variables and options | All developers |
| 08 | [Multi-Tenancy](0008-multi-tenancy.md) | Tenant isolation and namespace management | Platform engineers |
| 09 | [Algorithms Reference](0009-algorithms-reference.md) | Detailed pipeline and query algorithms | Researchers, core devs |

### Supplementary Guides

| Document | Description |
|----------|-------------|
| [Production LLM](production-llm-integration.md) | Real LLM provider integration with cost tracking |
| [SOTA Implementation](sota-implementation-summary.md) | State-of-the-art query engine details |
| [Source Citations](source-citations-status.md) | Citation tracking and provenance features |

---

## 🏃 Quick Links

### Getting Started (30 seconds)

```bash
# Clone and build
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake/edgequake
cargo build --release

# Run tests (no API key needed - uses mock provider)
cargo test

# Start server with OpenAI
export OPENAI_API_KEY=sk-xxx
./target/release/edgequake
# Server running at http://0.0.0.0:8080
```

### Verify Installation

```bash
# Health check
curl http://localhost:8080/health
# Expected: {"status":"healthy","version":"2.0.0"}

# WebUI (if built)
open http://localhost:3000
```

### Rust Usage (Programmatic API)

> **Implements**: [FEAT0001](features.md#feat0001) Document Ingestion, [FEAT0002](features.md#feat0002) Knowledge Graph Query

```rust
use edgequake_core::{EdgeQuake, EdgeQuakeConfig, QueryParams, QueryMode};
use edgequake_llm::OpenAIProvider;
use edgequake_storage::adapters::memory::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup LLM provider (or use MockProvider for testing)
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let provider = Arc::new(OpenAIProvider::new(api_key));

    // 2. Setup in-memory storage (or PostgreSQL for production)
    let kv = Arc::new(MemoryKVStorage::new());
    let vector = Arc::new(MemoryVectorStorage::new(1536));
    let graph = Arc::new(MemoryGraphStorage::new());

    // 3. Create EdgeQuake instance with namespace isolation
    let config = EdgeQuakeConfig::new().with_namespace("demo");
    let mut eq = EdgeQuake::new(config)
        .with_storage_backends(kv, vector, graph)
        .with_providers(provider.clone(), provider);

    eq.initialize().await?;

    // 4. Insert document (extracts entities + relationships automatically)
    let result = eq.insert("Your document text...", Some("doc-001")).await?;
    println!("Extracted {} entities, {} relationships", 
        result.entities_extracted, result.relationships_extracted);

    // 5. Query with hybrid mode (best for most use cases)
    let params = QueryParams::new().with_mode(QueryMode::Hybrid);
    let response = eq.query("Your question?", Some(params)).await?;
    println!("Answer: {}", response.response);
    println!("Sources: {:?}", response.sources);

    Ok(())
}
```

> **See Also**: [Full API examples](0001-quick-start.md#rust-api-examples)

### REST API (HTTP Integration)

> **Implements**: [FEAT0003](features.md#feat0003) REST API, [UC0001](use_cases.md#uc0001) Document Upload

```bash
# Health check
curl http://localhost:8080/health

# Insert document (returns 201 Created)
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -H "X-Tenant-ID: default" \
  -d '{"content": "Marie Curie discovered radium in 1898.", "title": "Curie Bio"}'

# Query with hybrid mode
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "X-Tenant-ID: default" \
  -d '{"query": "What did Marie Curie discover?", "mode": "hybrid"}'

# Stream response (SSE)
curl -N http://localhost:8080/api/v1/query/stream \
  -H "Content-Type: application/json" \
  -d '{"query": "Explain radium discovery", "mode": "hybrid"}'
```

> **See Also**: [Complete API Reference](0003-api-reference.md) with all 25+ endpoints

### Docker (Recommended for Production)

```bash
# Quick start with Docker Compose
cd edgequake/docker
docker compose up -d

# Includes: EdgeQuake API + PostgreSQL + pgvector + Apache AGE
# API: http://localhost:8080
# WebUI: http://localhost:3000 (if included)
```

> **See Also**: [Full Docker configuration](0006-deployment-guide.md#docker-deployment)

---

## 🔍 Query Modes Explained

EdgeQuake supports 6 query modes optimized for different use cases:

| Mode | Description | Use Case | Performance |
|------|-------------|----------|-------------|
| `naive` | Direct vector similarity search | Simple factual lookups | ⚡ Fastest |
| `local` | Entity-focused graph traversal | Specific entity facts | ⚡ Fast |
| `global` | Community summary aggregation | Broad thematic questions | 🔄 Medium |
| `hybrid` | Combined local + global **(default)** | Balanced accuracy | 🔄 Medium |
| `mix` | Full KG + vector + summary | Complex reasoning | 🐢 Slower |
| `bypass` | Skip RAG, direct LLM | Testing, fallback | ⚡ Fastest |

### Mode Selection Guide

```
Is your question about a specific entity?
├── YES → Use `local` mode
└── NO → Is it a broad thematic question?
    ├── YES → Use `global` mode
    └── NO → Use `hybrid` mode (recommended default)
```

> **Code Reference**: [`QueryMode` enum](../edgequake/crates/edgequake-core/src/types/query.rs)

---

## 🏗️ Deployment Options

| Environment | Recommended Stack | Guide |
|-------------|-------------------|-------|
| **Development** | Memory storage + Mock LLM | `cargo run` |
| **Staging** | Docker Compose + PostgreSQL | [Docker Guide](0006-deployment-guide.md#docker-deployment) |
| **Production** | Kubernetes + PostgreSQL + Real LLM | [K8s Guide](0006-deployment-guide.md#kubernetes-deployment) |

### Storage Topology by Environment

| Environment | KV Store | Vector Store | Graph Store | LLM Provider |
|-------------|----------|--------------|-------------|--------------|
| Development | Memory | Memory | Memory | Mock (free) |
| Staging | PostgreSQL | pgvector | Memory | OpenAI |
| Production | PostgreSQL | pgvector | Apache AGE | OpenAI/Azure |

> **Enforces**: [BR0001](business_rules.md#br0001) Tenant Isolation, [BR0002](business_rules.md#br0002) Data Persistence

---

## ⚙️ Configuration Quick Reference

### Essential Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `OPENAI_API_KEY` | Production | - | OpenAI API key for LLM/embeddings |
| `HOST` | No | `0.0.0.0` | Server bind address |
| `PORT` | No | `8080` | Server port |
| `WORKER_THREADS` | No | `4` | Tokio async worker threads |
| `DATABASE_URL` | Production | - | PostgreSQL connection string |
| `DEFAULT_NAMESPACE` | No | `default` | Default tenant namespace |

```bash
# Minimal production configuration
export OPENAI_API_KEY=sk-xxx
export DATABASE_URL=postgresql://user:pass@localhost:5432/edgequake
export PORT=8080
./target/release/edgequake
```

> **See Also**: [Complete Configuration Reference](0007-configuration-reference.md) with 40+ options

---

## 📁 Project Structure

```
edgequake/
├── Cargo.toml              # Workspace manifest
├── src/main.rs             # API server binary entry point
├── crates/
│   ├── edgequake-core/     # 🎯 Orchestrator, types, config (FEAT0001-0010)
│   ├── edgequake-api/      # 🌐 Axum REST API routes (FEAT0003)
│   ├── edgequake-llm/      # 🤖 LLM providers: OpenAI, Mock (FEAT0020-0025)
│   ├── edgequake-storage/  # 💾 Storage adapters: Memory, PostgreSQL (FEAT0030-0035)
│   ├── edgequake-pipeline/ # ⚙️ Document processing pipeline (FEAT0040-0045)
│   ├── edgequake-query/    # 🔍 SOTA query engine (FEAT0050-0055)
│   ├── edgequake-pdf/      # 📄 PDF extraction (FEAT0060)
│   ├── edgequake-auth/     # 🔐 Authentication (FEAT0070-0075)
│   ├── edgequake-audit/    # 📝 Audit logging (FEAT0080)
│   ├── edgequake-tasks/    # ⏰ Background tasks (FEAT0090)
│   └── edgequake-rate-limiter/ # 🚦 Rate limiting (FEAT0095)
├── edgequake_webui/        # 🖥️ Next.js WebUI (React 19 + TypeScript)
├── examples/               # Working Rust examples
├── tests/                  # Integration tests
└── docker/                 # Docker Compose configuration
```

> **See Also**: [Architecture Overview](0002-architecture-overview.md) for detailed crate descriptions

---

## 📖 Document Index (Complete Navigation)

### Getting Started
1. **[Quick Start Guide](0001-quick-start.md)** - Build, run, basic usage (15 min)
2. **[Architecture Overview](0002-architecture-overview.md)** - Crate structure, data flow

### API & Integration
3. **[API Reference](0003-api-reference.md)** - Complete REST API (25+ endpoints)
4. **[LLM Integration](0005-llm-integration.md)** - OpenAI, Mock, custom providers

### Infrastructure
5. **[Storage Backends](0004-storage-backends.md)** - Memory, PostgreSQL, pgvector, AGE
6. **[Deployment Guide](0006-deployment-guide.md)** - Docker, Kubernetes, production
7. **[Configuration Reference](0007-configuration-reference.md)** - All environment variables

### Advanced
8. **[Multi-Tenancy Guide](0008-multi-tenancy.md)** - Namespace isolation, tenant management
9. **[Algorithms Reference](0009-algorithms-reference.md)** - Entity extraction, graph algorithms
10. **[Production LLM](production-llm-integration.md)** - Real LLM provider integration

### Reference
- **[Features Registry](features.md)** - FEAT0001-XXXX catalog
- **[Business Rules](business_rules.md)** - BR0001-XXXX catalog
- **[Use Cases](use_cases.md)** - UC0001-XXXX catalog
- **[SOTA Query Comparison](sota-graph-query-comparison.md)** - Performance benchmarks

---

## 🆘 Troubleshooting

| Problem | Solution |
|---------|----------|
| `OPENAI_API_KEY not set` | Export the variable: `export OPENAI_API_KEY=sk-xxx` |
| Connection refused on :8080 | Ensure server is running: `./target/release/edgequake` |
| PostgreSQL connection failed | Check `DATABASE_URL` format and database existence |
| Out of memory on large docs | Increase chunk size or use streaming ingestion |

> **See Also**: [Deployment Troubleshooting](0006-deployment-guide.md#troubleshooting)

---

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Run tests: `cargo test && cargo clippy`
4. Submit a PR with FEAT/BR/UC references

---

_Built with Rust 🦀 for performance and reliability_

**License**: MIT | **Version**: 2.0.0 | **Tests**: 2120+ passing
