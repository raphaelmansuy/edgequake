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

Important Ensure to keep the files small and modular for maintainability.

## Build, Test, and Development Commands

- `cargo build`: Build the entire workspace
- `cargo test`: Run all tests (uses mock provider by default)
- `export OPENAI_API_KEY="sk-..." && cargo test`: Run tests with real OpenAI provider
- `cargo run --example production_pipeline`: Run production example with real LLM
- `cargo clippy`: Lint Rust code before committing
- `cargo fmt`: Format Rust code
- `bun install`, `bun run dev`, `bun run build`, `bun test`: Manage web UI workflow

### Quick Start with make

The `make dev` command starts the full stack with Ollama as the default provider:

```bash
# Start with Ollama (default)
make dev

# Start with OpenAI provider available for runtime switching
export OPENAI_API_KEY="sk-your-key"
make dev

# Check service status
make status
```

When OPENAI_API_KEY is set, you can switch between Ollama and OpenAI providers at runtime via the query UI or API.

### Background Testing (Agentic Mode)

For automated testing or continuous integration, use background mode to run services non-interactively:

```bash
# Start full stack in background (database + backend + frontend)
make dev-bg

# Check service health
make status

# View logs
tail -f /tmp/edgequake-backend.log
tail -f /tmp/edgequake-frontend.log

# Stop all services
make stop
```

**Alternative commands:**

- `make backend-bg`: Start backend only in background with PostgreSQL
- `make backend-memory`: Start backend with ephemeral in-memory storage (testing only)

Storage mode is automatically selected: PostgreSQL if `DATABASE_URL` is set, Memory otherwise.

## Service Management & E2E Testing

### Service Health Checks

After starting services with `make dev-bg`, verify each component is healthy:

```bash
# Backend health check (should return JSON with "status":"healthy")
curl http://localhost:8080/health

# Frontend health check (should return HTML)
curl -I http://localhost:3000

# PostgreSQL health check
docker ps | grep edgequake-postgres
```

**Expected Backend Response**:

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "storage_mode": "postgresql",
  "workspace_id": "default",
  "components": {
    "kv_storage": true,
    "vector_storage": true,
    "graph_storage": true,
    "llm_provider": true
  },
  "llm_provider_name": "ollama"
}
```

### Log File Locations

When services run in background mode, logs are written to:

- **Backend**: `/tmp/edgequake-backend.log`
- **Frontend**: `/tmp/edgequake-frontend.log`

**Viewing Logs**:

```bash
# Tail backend logs
tail -f /tmp/edgequake-backend.log

# Tail frontend logs
tail -f /tmp/edgequake-frontend.log

# Search for errors
grep -i error /tmp/edgequake-backend.log
grep -i "failed\|error" /tmp/edgequake-frontend.log
```

### Port Mappings

| Service            | Port  | Purpose            |
| ------------------ | ----- | ------------------ |
| Frontend (Next.js) | 3000  | Web UI             |
| Backend (Axum)     | 8080  | REST API           |
| PostgreSQL         | 5432  | Database           |
| Ollama (optional)  | 11434 | Local LLM provider |

### Known Issues & Workarounds

#### Frontend PID Management

**Issue**: Frontend process may die but PID file (`edgequake_webui/build_pid.txt`) remains, causing `make stop` to fail silently.

**Workaround**:

```bash
# Check if frontend is actually running
lsof -i :3000

# If port is free but PID file exists, manually restart:
cd edgequake_webui
rm -f build_pid.txt
bun run dev &
echo $! > build_pid.txt
```

**Permanent Fix**: See `specs/001-e2e-upload-pdf/ooda/iteration_03/` (planned enhancement).

#### Ollama Service Required

**Issue**: Entity extraction fails with "Network error" if Ollama is not running.

**Workaround**:

```bash
# Check Ollama status
curl http://localhost:11434/api/tags

# Start Ollama if not running
ollama serve &

# Or use OpenAI instead:
export OPENAI_API_KEY="sk-your-key"
make dev-bg
```

**Error Symptom**: Documents show status "Failed" with message "Pipeline processing failed: Entity extraction e...".

### MCP Playwright E2E Testing

EdgeQuake uses **MCP Playwright** for interactive E2E testing. This allows AI agents to automate browser interactions.

#### Prerequisites

```bash
# Install Playwright browsers (via MCP tool or manually)
cd edgequake_webui
pnpm install
npx playwright install chrome
```

#### Test Execution

**Via MCP Tool** (for AI agents):

```javascript
// Navigate to documents page
mcp_microsoft_pla_browser_navigate({ url: "http://localhost:3000/documents" });

// Take snapshot
mcp_microsoft_pla_browser_snapshot({});

// Click element
mcp_microsoft_pla_browser_click({ ref: "e175", element: "First document row" });
```

**Via Command Line** (for humans):

```bash
cd edgequake_webui
pnpm exec playwright test
pnpm exec playwright test --ui  # Interactive mode
pnpm exec playwright show-report  # View last run
```

#### Test Structure

```
edgequake_webui/e2e/
  ├── markdown-test.spec.ts     # Markdown rendering tests
  ├── upload-pdf.spec.ts        # PDF upload flow (planned)
  └── side-by-side-viewer.spec.ts # Side-by-side viewer (planned)
```

#### Common E2E Test Scenarios

**1. Verify PDF Upload & Display**:

```typescript
test("upload PDF and view side-by-side", async ({ page }) => {
  await page.goto("http://localhost:3000/documents");
  await page.click('button:has-text("Upload PDF")');
  await page.setInputFiles(
    'input[type="file"]',
    "zz_test_docs/lighrag_2410.05779v3.pdf",
  );
  await page.waitForSelector('[data-testid="side-by-side-viewer"]');

  // Verify PDF panel
  await expect(page.locator('[data-testid="pdf-viewer"]')).toBeVisible();

  // Verify markdown panel
  await expect(page.locator('[data-testid="markdown-renderer"]')).toBeVisible();
});
```

**2. Check Entity Extraction Progress**:

```typescript
test("monitor entity extraction", async ({ page }) => {
  await page.goto(
    "http://localhost:3000/documents/f6fa9cad-bbff-4892-a855-3bd7d70da044",
  );

  // Wait for processing to complete (may take 5-10 minutes)
  await page.waitForSelector('text="Completed"', { timeout: 600000 });

  // Verify entities extracted
  const entityCount = await page
    .locator('[data-testid="entity-count"]')
    .textContent();
  expect(parseInt(entityCount)).toBeGreaterThan(0);
});
```

### Troubleshooting Guide

#### Problem: Frontend Won't Start

**Symptoms**:

- `make dev-bg` completes but http://localhost:3000 returns "Connection refused"
- `/tmp/edgequake-frontend.log` shows compilation errors or empty

**Solution**:

```bash
# Check if process is running
ps aux | grep "bun run dev"

# Kill stale process
killall -9 node bun

# Remove PID file
rm -f edgequake_webui/build_pid.txt

# Restart manually
cd edgequake_webui
bun install  # Ensure dependencies are installed
bun run dev &
echo $! > build_pid.txt

# Verify it started
curl -I http://localhost:3000
```

#### Problem: Backend Won't Start

**Symptoms**:

- `make dev-bg` hangs or fails
- http://localhost:8080/health returns "Connection refused"
- `/tmp/edgequake-backend.log` shows database errors

**Solution**:

```bash
# Check PostgreSQL container
docker ps | grep edgequake-postgres

# If not running, start it:
make postgres-start

# Wait 5 seconds for DB to be ready
sleep 5

# Restart backend
make backend-bg

# Verify it started
curl http://localhost:8080/health
```

#### Problem: PDF Extraction Fails

**Symptoms**:

- Document status shows "Failed" with "Failed to load pdfium library"
- Side-by-side viewer shows PDF but no markdown

**Solution**:

```bash
# Verify libpdfium.dylib exists
ls -lh edgequake/crates/edgequake-pdf/lib/lib/libpdfium.dylib

# If missing, download it:
cd edgequake/crates/edgequake-pdf
./scripts/download-pdfium.sh

# Set environment variable manually:
export PDFIUM_DYNAMIC_LIB_PATH="$(pwd)/edgequake/crates/edgequake-pdf/lib/lib/libpdfium.dylib"

# Restart backend
make stop
make dev-bg
```

**Note**: `PDFIUM_DYNAMIC_LIB_PATH` is set automatically by Makefile since iteration 01 (commit b1611b45). This issue should not occur with current code.

#### Problem: Entity Extraction Fails

**Symptoms**:

- Document status shows "Failed" with "Network error: error sending request for url (http://localhost:11434/api/chat)"
- PDF and markdown display correctly, but no entities extracted

**Solution**:

```bash
# Check if Ollama is running
curl http://localhost:11434/api/tags

# If not running:
ollama serve &

# Verify models are pulled:
ollama list

# If qwen2.5 is missing:
ollama pull qwen2.5:latest

# Re-upload document to retry extraction
# (or wait for automatic retry in future iteration)
```

**Alternative**: Use OpenAI instead of Ollama:

```bash
export OPENAI_API_KEY="sk-your-key"
make stop
make dev-bg
```

#### Problem: Stale Frontend Cache

**Symptoms**:

- Document shows "Processing..." indefinitely even though backend shows "Completed"
- Side-by-side viewer displays old content

**Solution**:

```bash
# Hard refresh in browser
# Chrome/Firefox: Cmd+Shift+R (macOS) or Ctrl+Shift+R (Windows/Linux)

# Or clear React Query cache by restarting frontend:
make stop
make dev-bg

# Or use incognito/private browsing mode
```

### OODA Loop Documentation

This service management guide was created during **OODA Iteration 02** of the PDF upload/extraction fix.

**Reference**: `specs/001-e2e-upload-pdf/ooda/iteration_02/`

**Key Learnings**:

1. `make dev-bg` reliably starts all services with correct environment variables
2. MCP Playwright enables AI-driven E2E testing for verification
3. Frontend PID management needs improvement (see iteration 03 plan)
4. Ollama service must be running for entity extraction (separate from PDF extraction)

**Mission Status**: ✅ PDF extraction and side-by-side display verified working (2026-02-06)

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

## Claude Skills

This repository includes reusable SKILL definitions in `.github/skills/` for common development workflows:

### Available Skills

| Skill                             | Location                                                                                                       | Purpose                                                                                                                                                                                                                                                               |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **makefile-dev-workflow**         | [.github/skills/makefile-dev-workflow/SKILL.md](.github/skills/makefile-dev-workflow/SKILL.md)                 | Unified development workflow using Makefile commands. Use for starting services, running E2E tests, and managing the full development stack (database, backend, frontend). **Start here for dev setup.**                                                              |
| **doc-traceability-validator**    | [.github/skills/doc-traceability-validator/SKILL.md](.github/skills/doc-traceability-validator/SKILL.md)       | Validate FEAT/BR/UC traceability chain (224 features, 100% coverage). Detect undocumented features, duplicate IDs, namespace violations, broken references. Distinguishes cross-cutting duplicates (OK) from true collisions (FIX). **Use for documentation audits.** |
| **pdf-markdown-validator**        | [.github/skills/pdf-markdown-validator/SKILL.md](.github/skills/pdf-markdown-validator/SKILL.md)               | Validate PDF to Markdown conversion quality using multi-dimensional metrics (table accuracy, style preservation, robustness, performance). Use when measuring conversion fidelity and tracking improvements.                                                          |
| **playwright-ux-ui-capture**      | [.github/skills/playwright-ux-ui-capture/SKILL.md](.github/skills/playwright-ux-ui-capture/SKILL.md)           | Capture EdgeQuake WebUI routes with Playwright and write artifacts (screenshots + request JSON). Use when automating UI screenshot collection or updating E2E capture specs.                                                                                          |
| **reverse-documentation**         | [.github/skills/reverse-documentation/SKILL.md](.github/skills/reverse-documentation/SKILL.md)                 | Automatically generate comprehensive documentation for Rust and TypeScript codebases by analyzing code structure, patterns, and relationships. Supports trait-based patterns, async operations, and React components.                                                 |
| **ux-ui-analyze-single-page**     | [.github/skills/ux-ui-analyze-single-page/SKILL.md](.github/skills/ux-ui-analyze-single-page/SKILL.md)         | Analyze individual pages with Playwright for UX/UI improvements. Use when evaluating specific routes or components.                                                                                                                                                   |
| **ux-ui-map-page-by-page**        | [.github/skills/ux-ui-map-page-by-page/SKILL.md](.github/skills/ux-ui-map-page-by-page/SKILL.md)               | Map entire application UI across all pages with Playwright. Use when auditing complete application UX/UI.                                                                                                                                                             |
| **copilotkit-nextjs-integration** | [.github/skills/copilotkit-nextjs-integration/SKILL.md](.github/skills/copilotkit-nextjs-integration/SKILL.md) | Integrate CopilotKit AI components into Next.js frontend. Use when adding AI-powered UI features.                                                                                                                                                                     |

### Quick reference for common tasks

**Getting started with development:**

```bash
make dev              # Start full stack (database + backend + frontend)
make status           # Check service health
make stop             # Stop all services
```

See: [makefile-dev-workflow SKILL](.github/skills/makefile-dev-workflow/SKILL.md)

**Validating documentation traceability:**

```bash
# Validate FEAT IDs in code match docs/features.md
python3 .github/skills/doc-traceability-validator/scripts/validate_features.py \
  --code-dir edgequake_webui/src \
  --docs-file docs/features.md \
  --verbose

# Check namespace violations (wrong team IDs)
python3 .github/skills/doc-traceability-validator/scripts/check_namespace.py \
  --code-dir edgequake_webui/src

# Generate missing feature entries from code
python3 .github/skills/doc-traceability-validator/scripts/generate_registry.py \
  --code-dir edgequake_webui/src \
  --existing docs/features.md
```

See: [doc-traceability-validator SKILL](.github/skills/doc-traceability-validator/SKILL.md)

**Running E2E tests:**

```bash
cd edgequake_webui && pnpm exec playwright test markdown-test.spec.ts
```

See: [makefile-dev-workflow SKILL](.github/skills/makefile-dev-workflow/SKILL.md) → E2E Testing section

**Validating PDF → Markdown conversions:**

```bash
python3 .github/skills/pdf-markdown-validator/scripts/validate.py \
  --pdf-dir edgequake/crates/edgequake-pdf/test-data \
  --gold-dir edgequake/crates/edgequake-pdf/test-data \
  --verbose
```

See: [pdf-markdown-validator SKILL](.github/skills/pdf-markdown-validator/SKILL.md)

**Capturing UI screenshots:**

```bash
cd edgequake_webui && npx playwright test e2e/<spec>.spec.ts
```

See: [playwright-ux-ui-capture SKILL](.github/skills/playwright-ux-ui-capture/SKILL.md)
