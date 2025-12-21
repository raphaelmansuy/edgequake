# LightRAG Rust Technology Stack Documentation

**Date**: December 20, 2025  
**Status**: Complete  
**Purpose**: Technology selection and implementation guides for LightRAG Rust rewrite

---

## Overview

This directory contains comprehensive documentation for the technology stack selected for rebuilding LightRAG in Rust. The stack is designed for **type safety**, **performance**, **developer productivity**, and **long-term maintainability**.

---

## Quick Navigation

### Core Documentation

| Document | Description | Audience |
|----------|-------------|----------|
| **[technology_choice.md](./technology_choice.md)** | Architecture Decision Record (ADR) with justifications | Architects, Technical Leads |
| **[axum.md](./axum.md)** | Web framework guide | Backend Developers |
| **[surrealdb.md](./surrealdb.md)** | Multi-model database guide | Backend, Database Engineers |
| **[postgresql-age-pgvector.md](./postgresql-age-pgvector.md)** | PostgreSQL AGE + pgvector (primary database) | Backend, Database Engineers |
| **[falkordb.md](./falkordb.md)** | Redis-based graph database (alternative) | Backend, Database Engineers |
| **[async-openai.md](./async-openai.md)** | LLM client integration guide | ML/AI Engineers |
| **[open-webui.md](./open-webui.md)** | Production-ready web interface | Frontend, Full-Stack Developers |
| **[cytoscape.md](./cytoscape.md)** | Knowledge graph visualization | Frontend Developers |
| **[openapi-swagger.md](./openapi-swagger.md)** | API documentation with utoipa | Backend, API Developers |

---

## Technology Stack Summary

### Core Platform

| Layer | Technology | Version | Purpose |
|-------|-----------|---------|---------|
| **Language** | Rust | 2021 Edition | Systems programming |
| **Async Runtime** | Tokio | 1.x | Async I/O foundation |
| **Web Framework** | Axum | 0.8+ | REST API |
| **API Documentation** | utoipa | 5.0+ | OpenAPI/Swagger specs |
| **Primary Database** | PostgreSQL AGE + pgvector | PG16+, AGE 1.5+, pgvector 0.7+ | Graph + Vector + Relational |
| **Alternative Graph DB** | FalkorDB | 4.14+ | Ultra-low latency (Redis-based) |
| **Alternative Multi-Model DB** | SurrealDB | 2.x | All-in-one (Graph + Vector + Document) |
| **LLM Client** | async-openai | 0.32+ | OpenAI API |
| **Text Processing** | tiktoken-rs + text-splitter | Latest | Tokenization + chunking |
| **Frontend** | Open WebUI | 0.6+ | Production LLM interface |
| **Graph Visualization** | Cytoscape.js | 3.30+ | Interactive graph rendering |
| **Error Handling** | thiserror + anyhow | Latest | Type-safe errors |
| **Observability** | tracing | Latest | Structured logging |
| **Testing** | cargo-nextest | Latest | Fast test runner |

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     LightRAG Rust System                     │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌───────────────┐     ┌──────────────┐                     │
│  │  Leptos UI    │────▶│  Axum API    │                     │
│  │  (Frontend)   │     │  (REST/HTTP) │                     │
│  └───────────────┘     └──────┬───────┘                     │
│                                │                              │
│                                ▼                              │
│                    ┌────────────────────┐                    │
│                    │   LightRAG Core    │                    │
│                    │   (Orchestrator)   │                    │
│                    └────────┬───────────┘                    │
│                             │                                 │
│       ┌─────────────────────┼─────────────────────┐         │
│       │                     │                     │          │
│       ▼                     ▼                     ▼          │
│  ┌─────────┐          ┌──────────┐        ┌──────────┐     │
│  │Pipeline │          │  Query   │        │ Storage  │     │
│  │ Engine  │          │  Engine  │        │ Layer    │     │
│  └────┬────┘          └────┬─────┘        └────┬─────┘     │
│       │                    │                    │            │
│       ▼                    ▼                    ▼            │
│  ┌─────────────────────────────────────────────────────┐   │
│  │           LLM Providers (Trait)                      │   │
│  │  ┌──────────┐  ┌───────────┐  ┌─────────┐          │   │
│  │  │ OpenAI   │  │ Anthropic │  │ Ollama  │          │   │
│  │  └──────────┘  └───────────┘  └─────────┘          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │               SurrealDB (Multi-Model)                │   │
│  │  ┌───────────┐  ┌────────┐  ┌───────────┐          │   │
│  │  │   Graph   │  │ Vector │  │ Document  │          │   │
│  │  │ Relations │  │ Search │  │  Storage  │          │   │
│  │  └───────────┘  └────────┘  └───────────┘          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

---

## Project Structure

### Recommended Workspace Layout

```
lightrag-rust/
├── Cargo.toml                    # Workspace root
├── .cargo/config.toml
├── .env.example
├── config.toml.example
├── Dockerfile
├── docker-compose.yml
├── README.md
│
├── crates/
│   ├── lightrag-core/            # Core orchestrator & types
│   ├── lightrag-storage/         # Storage abstractions
│   ├── lightrag-llm/             # LLM provider traits
│   ├── lightrag-pipeline/        # Document processing
│   ├── lightrag-query/           # Query engine
│   └── lightrag-api/             # REST API (Axum)
│
├── lightrag-ui/                  # Leptos frontend
│
├── examples/                     # Usage examples
│   ├── simple_insert.rs
│   ├── query_modes.rs
│   └── custom_provider.rs
│
├── tests/                        # Integration tests
│   └── integration/
│
└── docs/                         # Additional documentation
```

---

## Getting Started

### Prerequisites

- **Rust**: 1.75+ (2021 edition)
- **SurrealDB**: Latest (for development)
- **OpenAI API Key**: For LLM operations
- **Node.js/Bun**: For frontend (Leptos)

### Installation Steps

1. **Clone and setup Rust workspace**
   ```bash
   cargo new --lib lightrag-core
   cargo new --lib lightrag-storage
   cargo new --bin lightrag-api
   ```

2. **Start SurrealDB**
   ```bash
   docker run -p 8000:8000 surrealdb/surrealdb:latest start
   ```

3. **Configure environment**
   ```bash
   cp .env.example .env
   # Edit .env with API keys
   ```

4. **Build and run**
   ```bash
   cargo build --workspace
   cargo run --bin lightrag-api
   ```

---

## Implementation Phases

### Phase 1: Core Foundation (Weeks 1-3)
- ✅ Core types (`Document`, `Chunk`, `Entity`, `Relationship`)
- ✅ SurrealDB storage adapter
- ✅ Storage trait abstractions
- ✅ Comprehensive unit tests

### Phase 2: Pipeline (Weeks 4-6)
- ✅ tiktoken-rs + text-splitter integration
- ✅ Entity extraction with async-openai
- ✅ Graph merging logic
- ✅ Embedding generation

### Phase 3: Query Engine (Weeks 7-8)
- ✅ Naive mode (vector search)
- ✅ Local mode (entity-centric)
- ✅ Global mode (graph-centric)
- ✅ Hybrid mode

### Phase 4: API Layer (Weeks 9-10)
- ✅ Axum routes
- ✅ Request/response types
- ✅ OpenAPI documentation
- ✅ Integration tests

### Phase 5: Frontend (Weeks 11-12)
- ✅ Leptos UI components
- ✅ Document upload
- ✅ Query interface
- ✅ Graph visualization

### Phase 6: Production (Weeks 13-14)
- ✅ Docker deployment
- ✅ Kubernetes manifests
- ✅ Performance optimization
- ✅ Security audit

---

## Key Design Decisions

### 1. Why PostgreSQL AGE + pgvector Over Neo4j + Qdrant?

**Consolidation**: Reduces operational complexity
- Neo4j + Qdrant setup: 2+ databases
- PostgreSQL AGE + pgvector: 1 database

**Native Integration**: Better performance, unified queries  
**OpenCypher Support**: Same query language as Neo4j  
**Ecosystem**: PostgreSQL's massive tooling and community  
**Cost**: Open source vs Neo4j Enterprise licensing

### 2. Why Open WebUI Over Custom Leptos Frontend?

**Production-Ready**: 118k+ stars, battle-tested  
**Zero Development Time**: Deploy in hours, not months  
**Feature-Complete**: RAG, document management, auth built-in  
**Active Community**: 690+ contributors, continuous updates  
**Focus on Core**: Spend Rust time on RAG engine, not UI

### 3. Why FalkorDB as Alternative?

**Ultra-Low Latency**: Sub-millisecond queries (Redis-based)  
**Multi-Tenancy**: Built-in graph isolation  
**LLM-Optimized**: Designed for knowledge graphs in RAG  
**Use When**: Real-time applications need <1ms latency

---

## Performance Targets

| Metric | Python Baseline | Rust Target | Improvement |
|--------|----------------|-------------|-------------|
| **Document Chunking** | 100ms | <10ms | 10x |
| **Graph Insertion** | 200ms | <20ms | 10x |
| **Vector Search** | 50ms | <10ms | 5x |
| **Query (Hybrid)** | 2s | <500ms | 4x |
| **Memory Usage** | 500MB | <100MB | 5x |

---

## Best Practices

### Code Style
- Follow `rustfmt` defaults
- Enable `clippy` pedantic mode
- Use `cargo-nextest` for tests
- Write integration tests for all public APIs

### Error Handling
- Use `thiserror` for library crates
- Use `anyhow` for application crates
- Implement `IntoResponse` for API errors

### Testing
- Unit tests in same file as code
- Integration tests in `tests/` directory
- Use property-based testing for algorithms
- Mock external dependencies

### Documentation
- Rustdoc comments on all public items
- Examples in doc comments
- README in each crate

---

## Migration from Python

### Key Mappings

| Python | Rust |
|--------|------|
| `async def` | `async fn` |
| `Dict` | `HashMap` |
| `List` | `Vec` |
| `str` | `String` / `&str` |
| `Optional[T]` | `Option<T>` |
| `Exception` | `Result<T, E>` |
| `@dataclass` | `#[derive(Serialize, Deserialize)]` |
| FastAPI `@app.get` | Axum `route("/", get(handler))` |
| NetworkX graph | petgraph or SurrealDB relations |

---

## Resources

### Official Documentation
- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Axum Examples](https://github.com/tokio-rs/axum/tree/main/examples)
- [SurrealDB Docs](https://surrealdb.com/docs)
- [async-openai Docs](https://docs.rs/async-openai/latest/async_openai/)

### Community
- [Rust Discord](https://discord.gg/rust-lang)
- [Tokio Discord](https://discord.gg/tokio)
- [SurrealDB Discord](https://discord.gg/surrealdb)

### Learning Resources
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Async Book](https://rust-lang.github.io/async-book/)
- [Axum Web Development](https://github.com/tokio-rs/axum/tree/main/examples)

---

## FAQ

### Q: Why Rust over Python for RAG?
**A**: 10-100x performance improvement, type safety, true parallelism, single binary deployment.

### Q: Is SurrealDB production-ready?
**A**: Yes, as of 2025 it's battle-tested with major deployments. Monitor GitHub for updates.

### Q: Can I use other LLM providers?
**A**: Yes! Implement the `LLMProvider` trait for any provider (Anthropic, Gemini, etc.).

### Q: How does this compare to LangChain?
**A**: LightRAG focuses specifically on graph-based RAG with knowledge extraction, optimized for performance.

### Q: What about Python interop?
**A**: Use PyO3 to expose Rust functions to Python if needed.

---

## Maintenance

### Updating Dependencies
```bash
cargo update
cargo audit
cargo outdated
```

### Security Audits
```bash
cargo audit
cargo deny check
```

### Performance Profiling
```bash
cargo flamegraph --bin lightrag-api
cargo bench
```

---

## Contributing

### Code Review Checklist
- [ ] All tests pass (`cargo nextest run`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Formatted (`cargo fmt`)
- [ ] Documentation updated
- [ ] Examples added for new features

---

## License

Same as LightRAG (MIT or as specified by upstream project)

---

## Contact

For questions about technology choices, open an issue or discussion in the repository.

---

**Last Updated**: December 20, 2025  
**Version**: 1.0  
**Status**: ✅ Complete and Ready for Implementation
