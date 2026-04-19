# EdgeQuake

**High-Performance RAG with Knowledge Graph**

[![Rust](https://img.shields.io/badge/rust-1.95+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Docker](https://img.shields.io/badge/docker-GHCR-blue.svg)](https://github.com/raphaelmansuy/edgequake/pkgs/container/edgequake)

EdgeQuake is a next-generation Retrieval-Augmented Generation (RAG) system built in Rust, designed for high performance, reliability, and scalability. It combines vector similarity search with knowledge graph traversal to provide contextually rich answers.

---

## ⚡ Quickstart — One Command, Full Stack (~30 seconds)

> **No Rust toolchain, no Node.js, no build step required.**  
> Three prebuilt images (API, Web UI, PostgreSQL) are pulled from GitHub Container Registry.

```bash
# Clone (or just download the compose file)
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake

# Start everything
make stack
```

Or without `make`:

```bash
docker compose -f docker-compose.quickstart.yml up -d
```

**Access:**
| Service   | URL                              |
| --------- | -------------------------------- |
| 🌐 Web UI  | http://localhost:3000            |
| 🔗 API     | http://localhost:8080            |
| 📚 Swagger | http://localhost:8080/swagger-ui |
| 🏥 Health  | http://localhost:8080/health     |

**Stop:**
```bash
make stack-down
# or
docker compose -f docker-compose.quickstart.yml down
```

**Use OpenAI instead of the default Ollama provider:**
```bash
EDGEQUAKE_LLM_PROVIDER=openai OPENAI_API_KEY=sk-... make stack
```

**Pin to a specific version:**
```bash
EDGEQUAKE_VERSION=0.10.8 make stack
```

---

## Features

- 🚀 **High Performance**: Built in Rust for maximum speed and memory efficiency
- 🔗 **Knowledge Graph**: Entity extraction and relationship mapping
- 🔍 **Multiple Query Modes**: Naive, Local, Global, Hybrid, and Mix
- 📊 **OpenAPI Documentation**: Full Swagger UI support
- 🔧 **Modular Architecture**: Pluggable storage backends and LLM providers
- 🌐 **REST API**: Clean, versioned HTTP API
- ⛔ **Cooperative Cancellation**: Cancel long-running pipeline tasks mid-flight via API

## Quick Start

### Prerequisites

- Rust 1.95 or later
- OpenAI API key (or compatible API)

### Installation

```bash
# Clone the repository
git clone https://github.com/your-org/edgequake
cd edgequake

# Build
cargo build --release

# Run
OPENAI_API_KEY=your-key cargo run --release
```

### Docker

```bash
docker build -t edgequake .
docker run -p 8080:8080 -e OPENAI_API_KEY=your-key edgequake
```

## API Endpoints

### Health

- `GET /health` - Health check with component status
- `GET /ready` - Kubernetes readiness probe
- `GET /live` - Kubernetes liveness probe

### Documents

- `POST /api/v1/documents` - Upload a document for processing
- `GET /api/v1/documents` - List all documents

### Query

- `POST /api/v1/query` - Execute a query
- `POST /api/v1/query/stream` - Streaming query (SSE)

### Knowledge Graph

- `GET /api/v1/graph` - Get knowledge graph
- `GET /api/v1/graph/nodes/{id}` - Get specific node
- `GET /api/v1/graph/labels/search` - Search labels

## Query Modes

| Mode     | Description                       | Use Case                |
| -------- | --------------------------------- | ----------------------- |
| `naive`  | Simple vector similarity          | Fast, basic queries     |
| `local`  | Entity-centric with local context | Specific entity queries |
| `global` | Community-based search            | Broad topic exploration |
| `hybrid` | Combines local and global         | Balanced approach       |
| `mix`    | Weighted combination              | Maximum flexibility     |

## Project Structure

```
edgequake/
├── Cargo.toml              # Workspace manifest
├── src/main.rs             # Server entry point
└── crates/
    ├── edgequake-api/      # REST API server (Axum)
    ├── edgequake-audit/    # Audit logging and compliance
    ├── edgequake-auth/     # Authentication and authorization
    ├── edgequake-core/     # Core types, orchestration
    ├── edgequake-llm/      # LLM providers and reranking
    ├── edgequake-pdf/      # PDF parsing and extraction
    ├── edgequake-pipeline/ # Document processing pipeline
    ├── edgequake-query/    # SOTA query engine
    ├── edgequake-rate-limiter/ # Rate limiting middleware
    ├── edgequake-storage/  # Storage backends (Memory, PostgreSQL)
    └── edgequake-tasks/    # Background task processing
```

## Configuration

### Environment Variables

| Variable          | Description         | Default        |
| ----------------- | ------------------- | -------------- |
| `HOST`            | Server host         | `0.0.0.0`      |
| `PORT`            | Server port         | `8080`         |
| `OPENAI_API_KEY`  | OpenAI API key      | Required       |
| `OPENAI_BASE_URL` | Custom API base URL | OpenAI default |
| `LOG_LEVEL`       | Logging level       | `info`         |

## Development

```bash
# Run tests
cargo test --all

# Run with hot reload
cargo watch -x run

# Check formatting
cargo fmt --check

# Run lints
cargo clippy --all-targets
```

## Architecture

EdgeQuake follows a modular architecture:

1. **Document Ingestion**
   - Text chunking with overlap
   - Entity extraction via LLM
   - Relationship extraction
   - Embedding generation

2. **Storage Layer**
   - Key-value store for documents
   - Vector store for embeddings
   - Graph store for knowledge graph

3. **Query Engine**
   - Multi-mode retrieval
   - Context assembly
   - LLM answer generation

4. **API Layer**
   - RESTful endpoints
   - OpenAPI documentation
   - Authentication (coming soon)

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE).

## Acknowledgments

EdgeQuake is inspired by [LightRAG](https://github.com/HKUDS/LightRAG) and designed to bring its powerful concepts to production-grade Rust infrastructure.

---

**Built for the future of AI infrastructure in Europe and the Free Nations of the world. 🌍**
