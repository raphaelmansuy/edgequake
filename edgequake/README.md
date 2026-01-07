# EdgeQuake

**High-Performance RAG with Knowledge Graph**

[![Rust](https://img.shields.io/badge/rust-1.78+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

EdgeQuake is a next-generation Retrieval-Augmented Generation (RAG) system built in Rust, designed for high performance, reliability, and scalability. It combines vector similarity search with knowledge graph traversal to provide contextually rich answers.

## Features

- 🚀 **High Performance**: Built in Rust for maximum speed and memory efficiency
- 🔗 **Knowledge Graph**: Entity extraction and relationship mapping
- 🔍 **Multiple Query Modes**: Naive, Local, Global, Hybrid, and Mix
- 📊 **OpenAPI Documentation**: Full Swagger UI support
- 🔧 **Modular Architecture**: Pluggable storage backends and LLM providers
- 🌐 **REST API**: Clean, versioned HTTP API

## Quick Start

### Prerequisites

- Rust 1.78 or later
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

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

EdgeQuake is inspired by [LightRAG](https://github.com/HKUDS/LightRAG) and designed to bring its powerful concepts to production-grade Rust infrastructure.

---

**Built for the future of AI infrastructure in Europe and the Free Nations of the world. 🌍**
