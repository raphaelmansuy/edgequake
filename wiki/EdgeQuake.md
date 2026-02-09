# EdgeQuake Wiki

## Overview
EdgeQuake is an advanced Retrieval-Augmented Generation (RAG) framework implemented in Rust, designed to enhance information retrieval and generation through graph-based knowledge representation. It supports production-grade document ingestion, entity extraction, and knowledge graph construction, with a modern web UI and robust backend architecture.

## Key Features
- **Graph-Based RAG**: Utilizes a knowledge graph for context-rich retrieval and generation.
- **Modular Rust Architecture**: Core, LLM, storage, API, pipeline, and query crates for maintainability.
- **Multi-Provider LLM Support**: Switch between Ollama (local), OpenAI, and mock providers at runtime.
- **PostgreSQL + Apache AGE**: Graph and vector storage with pgvector and AGE extensions.
- **PDF to Markdown Extraction**: High-fidelity PDF ingestion and markdown conversion.
- **Entity Extraction**: Automated entity and relationship extraction using LLMs.
- **Web UI**: React 19 + TypeScript frontend with E2E Playwright tests.
- **Production Ready**: CI/CD, robust testing, and hybrid LLM/embedding provider support.

## Project Structure
- `edgequake/crates/` — Core Rust crates (API, LLM, storage, pipeline, query)
- `edgequake/examples/` — Production examples and demos
- `edgequake/tests/` — Integration and E2E tests
- `lightrag/` — Legacy Python implementation
- `lightrag_webui/` — React 19 + TypeScript client
- `docs/` — Documentation and guides

## Quick Start
1. **Clone the repository**
2. **Start PostgreSQL**: `make postgres-start`
3. **Start Ollama**: `ollama serve &`
4. **Start the stack**: `make dev`
5. **Verify health**: `make status`
6. **Access UI**: [http://localhost:3000](http://localhost:3000)

## LLM Provider Configuration
- **Ollama** (default): Local, free, fast
- **OpenAI**: Set `OPENAI_API_KEY` for production
- **Hybrid**: Use different providers for LLM and embeddings

## Testing & Development
- `cargo test` — Run all Rust tests
- `cargo clippy` — Lint code
- `bun test` — Run frontend tests
- `pnpm exec playwright test` — E2E UI tests

## Documentation
- See `/docs` for architecture, API reference, and troubleshooting
- `/articles` for deep dives and design rationale

## Useful Links
- [GitHub Repository](https://github.com/raphaelmansuy/edgequake)
- [Production LLM Integration Guide](docs/production-llm-integration.md)
- [Troubleshooting](docs/troubleshooting/)

## License
EdgeQuake is licensed under the MIT License.

---
For more details, see the README and documentation in the repository.
