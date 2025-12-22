# Repository Guidelines

EdgeQuake is an advanced Retrieval-Augmented Generation (RAG) framework implemented in Rust, designed to enhance information retrieval and generation through graph-based knowledge representation.

## Project Structure & Module Organization
- `edgequake/crates/`: Core Rust crates
  - `edgequake-core/`: Orchestration layer with pipeline and EdgeQuake API
  - `edgequake-llm/`: LLM provider implementations (OpenAI, Mock)
  - `edgequake-storage/`: Storage adapters (Memory, PostgreSQL AGE)
  - `edgequake-api/`: REST API service with Axum
  - `edgequake-pipeline/`: Document processing pipeline
  - `edgequake-query/`: Query engine for knowledge graph
- `edgequake/examples/`: Production examples and demos
- `edgequake/tests/`: Integration and E2E tests
- `lightrag/`: Legacy Python implementation (being replaced)
- `lightrag_webui/`: React 19 + TypeScript client driven by Bun + Vite
- `docs/`: Comprehensive documentation including production guides

## Build, Test, and Development Commands
- `cargo build`: Build the entire workspace
- `cargo test`: Run all tests (uses mock provider by default)
- `export OPENAI_API_KEY="sk-..." && cargo test`: Run tests with real OpenAI provider
- `cargo run --example production_pipeline`: Run production example with real LLM
- `cargo clippy`: Lint Rust code before committing
- `cargo fmt`: Format Rust code
- `bun install`, `bun run dev`, `bun run build`, `bun test`: Manage web UI workflow

## LLM Provider Configuration
EdgeQuake supports multiple LLM providers with automatic environment-based selection:
- **Mock Provider**: Used by default for testing (free, fast, no API key required)
- **OpenAI Provider**: Automatically used when `OPENAI_API_KEY` is set
  - Recommended model: `gpt-4o-mini` (cost-effective: $0.0014 per document)
  - Recommended embedding: `text-embedding-3-small` (1536 dimensions)
- **Ollama/LM Studio**: Use OpenAI-compatible API mode

## Coding Style & Naming Conventions
- Follow Rust standard style guide and formatting with `rustfmt`
- Use `clippy` for linting and follow its suggestions
- Prefer idiomatic Rust patterns: Result<T>, Option<T>, async/await
- Use `tracing` crate for logging, not `println!`
- Entity names should be normalized: UPPERCASE with underscores (e.g., "SARAH_CHEN")
- Module names: lowercase with underscores (e.g., `entity_extraction`)
- Struct/Enum names: PascalCase (e.g., `EntityExtractor`, `GraphStorage`)
- Front-end code: TypeScript with two-space indentation, functional React components

## Testing Guidelines
- Tests live in `tests/` directories within each crate
- E2E tests in `edgequake/crates/edgequake-core/tests/`
- Use `#[tokio::test]` for async tests
- Tests automatically use mock provider unless `OPENAI_API_KEY` is set
- Integration tests can be marked with `#[cfg(feature = "integration")]`
- Run specific test: `cargo test --package edgequake-core --test e2e_pipeline`
- UI tests: `bun test`

## Production LLM Integration
✅ **Status: PRODUCTION READY**

The system now supports real LLM providers for production deployment:

1. **Environment-Based Selection:**
   ```bash
   # Development/CI: Uses mock provider (free, fast)
   cargo test
   
   # Production: Uses real OpenAI provider
   export OPENAI_API_KEY="sk-your-key"
   cargo test
   ```

2. **Provider Factory Pattern:**
   - Automatically detects `OPENAI_API_KEY` environment variable
   - Falls back to smart mock if no API key present
   - No code changes needed between dev and prod

3. **Quality Validation:**
   - Real LLM: 20 entities → 12 unique nodes (40% deduplication)
   - Mock LLM: 9 entities → 6 unique nodes (33% deduplication)
   - Real LLM extracts 2-3x more entities with better quality

4. **Documentation:**
   - Complete guide: `docs/production-llm-integration.md` (900+ lines)
   - Production readiness: `docs/PRODUCTION_READY.md`
   - Working example: `examples/production_pipeline.rs`

## Commit & Pull Request Guidelines
- Use concise, imperative commit subjects (e.g., `Fix entity normalization`)
- PRs should include summary, operational impact, and linked issues
- Verify `cargo clippy`, `cargo test`, and `cargo fmt --check` pass
- For UI changes, ensure `bun test` passes
- Document any new environment variables in `.env.example`

## Security & Configuration Tips
- Never commit API keys or secrets
- Use environment variables for configuration (OPENAI_API_KEY, DATABASE_URL, etc.)
- Copy `.env.example` to `.env` for local development
- PostgreSQL connections should use connection pooling
- Rate limit API calls to LLM providers
- Monitor costs and usage for production deployments

## Automation & Agent Workflow
- Use absolute paths for file operations
- Prefer `cargo test` over manual `rustc` invocations
- Run `cargo clippy` before suggesting code changes
- For LLM testing, check for `OPENAI_API_KEY` environment variable
- Validate changes by running relevant test suite
- Keep generated code idiomatic Rust (use Result<T>, avoid unwrap() in production)
- Follow the LightRAG entity extraction algorithm for consistency
