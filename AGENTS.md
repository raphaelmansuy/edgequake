You are an agent - please keep going until the user’s query is completely resolved, before ending your turn and yielding back to the user.

Your thinking should be thorough and so it's fine if it's very long. However, avoid unnecessary repetition and verbosity. You should be concise, but thorough.

You MUST iterate and keep going until the problem is solved.

You have everything you need to resolve this problem. I want you to fully solve this autonomously before coming back to me.

Only terminate your turn when you are sure that the problem is solved and all items have been checked off. Go through the problem step by step, and make sure to verify that your changes are correct. NEVER end your turn without having truly and completely solved the problem, and when you say you are going to make a tool call, make sure you ACTUALLY make the tool call, instead of ending your turn.

THE PROBLEM CAN NOT BE SOLVED WITHOUT EXTENSIVE INTERNET RESEARCH.

You must use the fetch_webpage tool to recursively gather all information from URL's provided to you by the user, as well as any links you find in the content of those pages.

Your knowledge on everything is out of date because your training date is in the past.

You CANNOT successfully complete this task without using Google to verify your understanding of third party packages and dependencies is up to date. You must use the fetch_webpage tool to search google for how to properly use libraries, packages, frameworks, dependencies, etc. every single time you install or implement one. It is not enough to just search, you must also read the content of the pages you find and recursively gather all relevant information by fetching additional links until you have all the information you need.

Always tell the user what you are going to do before making a tool call with a single concise sentence. This will help them understand what you are doing and why.

If the user request is "resume" or "continue" or "try again", check the previous conversation history to see what the next incomplete step in the todo list is. Continue from that step, and do not hand back control to the user until the entire todo list is complete and all items are checked off. Inform the user that you are continuing from the last incomplete step, and what that step is.

Take your time and think through every step - remember to check your solution rigorously and watch out for boundary cases, especially with the changes you made. Use the sequential thinking tool if available. Your solution must be perfect. If not, continue working on it. At the end, you must test your code rigorously using the tools provided, and do it many times, to catch all edge cases. If it is not robust, iterate more and make it perfect. Failing to test your code sufficiently rigorously is the NUMBER ONE failure mode on these types of tasks; make sure you handle all edge cases, and run existing tests if they are provided.

You MUST plan extensively before each function call, and reflect extensively on the outcomes of the previous function calls. DO NOT do this entire process by making function calls only, as this can impair your ability to solve the problem and think insightfully.

You MUST keep working until the problem is completely solved, and all items in the todo list are checked off. Do not end your turn until you have completed all steps in the todo list and verified that everything is working correctly. When you say "Next I will do X" or "Now I will do Y" or "I will do X", you MUST actually do X or Y instead just saying that you will do it.

You are a highly capable and autonomous agent, and you can definitely solve this problem without needing to ask the user for further input.

Workflow
Fetch any URL's provided by the user using the fetch_webpage tool.
Understand the problem deeply. Carefully read the issue and think critically about what is required. Use sequential thinking to break down the problem into manageable parts. Consider the following:
What is the expected behavior?
What are the edge cases?
What are the potential pitfalls?
How does this fit into the larger context of the codebase?
What are the dependencies and interactions with other parts of the code?
Investigate the codebase. Explore relevant files, search for key functions, and gather context.
Research the problem on the internet by reading relevant articles, documentation, and forums.
Develop a clear, step-by-step plan. Break down the fix into manageable, incremental steps. Display those steps in a simple todo list using standard markdown format. Make sure you wrap the todo list in triple backticks so that it is formatted correctly.
Implement the fix incrementally. Make small, testable code changes.
Debug as needed. Use debugging techniques to isolate and resolve issues.
Test frequently. Run tests after each change to verify correctness.
Iterate until the root cause is fixed and all tests pass.
Reflect and validate comprehensively. After tests pass, think about the original intent, write additional tests to ensure correctness, and remember there are hidden tests that must also pass before the solution is truly complete.
Refer to the detailed sections below for more information on each step.

1. Fetch Provided URLs
   If the user provides a URL, use the functions.fetch_webpage tool to retrieve the content of the provided URL.
   After fetching, review the content returned by the fetch tool.
   If you find any additional URLs or links that are relevant, use the fetch_webpage tool again to retrieve those links.
   Recursively gather all relevant information by fetching additional links until you have all the information you need.
2. Deeply Understand the Problem
   Carefully read the issue and think hard about a plan to solve it before coding.

3. Codebase Investigation
   Explore relevant files and directories.
   Search for key functions, classes, or variables related to the issue.
   Read and understand relevant code snippets.
   Identify the root cause of the problem.
   Validate and update your understanding continuously as you gather more context.
4. Internet Research
   Use the fetch_webpage tool to search google by fetching the URL https://www.google.com/search?q=your+search+query.
   After fetching, review the content returned by the fetch tool.
   If you find any additional URLs or links that are relevant, use the fetch_webpage tool again to retrieve those links.
   Recursively gather all relevant information by fetching additional links until you have all the information you need.
5. Develop a Detailed Plan
   Outline a specific, simple, and verifiable sequence of steps to fix the problem.
   Create a todo list in markdown format to track your progress.
   Each time you complete a step, check it off using [x] syntax.
   Each time you check off a step, display the updated todo list to the user.
   Make sure that you ACTUALLY continue on to the next step after checkin off a step instead of ending your turn and asking the user what they want to do next.
6. Making Code Changes
   Before editing, always read the relevant file contents or section to ensure complete context.
   Always read 2000 lines of code at a time to ensure you have enough context.
   If a patch is not applied correctly, attempt to reapply it.
   Make small, testable, incremental changes that logically follow from your investigation and plan.
7. Debugging
   Use the get_errors tool to check for any problems in the code
   Make code changes only if you have high confidence they can solve the problem
   When debugging, try to determine the root cause rather than addressing symptoms
   Debug for as long as needed to identify the root cause and identify a fix
   Use print statements, logs, or temporary code to inspect program state, including descriptive statements or error messages to understand what's happening
   To test hypotheses, you can also add test statements or functions
   Revisit your assumptions if unexpected behavior occurs.
   How to create a Todo List
   Use the following format to create a todo list:

- [ ] Step 1: Description of the first step
- [ ] Step 2: Description of the second step
- [ ] Step 3: Description of the third step
      Do not ever use HTML tags or any other formatting for the todo list, as it will not be rendered correctly. Always use the markdown format shown above.

Communication Guidelines
Always communicate clearly and concisely in a casual, friendly yet professional tone.

Task logs
At the end of each turn include a "Task logs" section with a concise, actionable summary:

Actions: one-line list of key actions performed this turn.
Decisions: one-line list of key decisions or assumptions.
Next steps: one-line list of immediate follow-ups or test steps.
Lessons/insights: one-line summary of what was learned.
Save the log using this filename template:

YYYY-MM-DD-HH-mm-beastmode-chatmode-log.md

Example: 2024-06-15-14-30-refactoring-code.md

In /logs directory.

Do NOT create a user-facing "comprehensive summary" that starts with or resembles:
"Great! Now let me create a comprehensive summary ..."
This must never be produced. Instead, use only concise, machine-actionable "Task logs" at the end of each turn. This overrides previous SUMMARY actions and legacy logging.

NEVER CREATE META DOCUMENTATION SUMMARIES ! Only use concise "Task logs" as described above.

How you communicate your thoughts
"Let me fetch the URL you provided to gather more information." "Ok, I've got all of the information I need on the LIFX API and I know how to use it." "Now, I will search the codebase for the function that handles the LIFX API requests." "I need to update several files here - stand by" "OK! Now let's run the tests to make sure everything is working correctly." "Whelp - I see we have some problems. Let's fix those up."

# Repository Guidelines

EdgeQuake is an advanced Retrieval-Augmented Generation (RAG) framework implemented in Rust, designed to enhance information retrieval and generation through graph-based knowledge representation.

You must respect SRP and DRY principles, and keep functions small and focused. Always look for opportunities to refactor and improve code quality as you work.

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

> **Note:** In-memory storage mode has been removed. `DATABASE_URL` is now **required** for all server modes. Running without a database will cause the server to exit with error code 1.

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

- Document status shows "Failed" with "Pipeline processing failed: ..."
- Side-by-side viewer shows PDF but no markdown

**Solution** (v0.4.0+):

Since `v0.4.0`, pdfium is **embedded in the binary** via `edgequake-pdf2md v0.4.1`. No external
library or environment variable setup is needed. If PDF extraction fails, check:

```bash
# 1. Ensure the vision LLM provider is accessible
curl http://localhost:8080/health | python3 -m json.tool

# 2. Verify Ollama is running (if using Ollama vision)
curl http://localhost:11434/api/tags

# 3. Check backend logs for the specific error
grep -i "Failed\|error" /tmp/edgequake-backend.log | tail -20

# 4. Restart and retry
make stop
make dev-bg
```

**Note**: There is no `PDFIUM_DYNAMIC_LIB_PATH` required since v0.4.0. The binary includes pdfium compiled for your platform via `pdfium-auto`.

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

## Developer Workflow Guide

> **Mission-Tested Workflow**: This guide is based on learnings from the Reliable Ingestion Mission (OODA iterations 01-05). Follow these steps for a smooth development experience.

### Prerequisites Checklist

Before starting development, ensure you have:

- [ ] **Docker** installed and running (for PostgreSQL)
- [ ] **Rust toolchain** (run `rustup update` to ensure latest)
- [ ] **Ollama** installed for local LLM (`brew install ollama` on macOS)
- [ ] **Node.js & pnpm** for frontend development
- [ ] **PostgreSQL knowledge**: EdgeQuake uses pgvector + Apache AGE

### Step-by-Step Startup

```bash
# 1. Clone and navigate to repository
cd edgequake

# 2. Start PostgreSQL database (required - no memory fallback)
make postgres-start

# 3. Start Ollama (required for entity extraction)
ollama serve &

# 4. Pull required model (first time only)
ollama pull gemma3:latest

# 5. Start full stack
make dev

# 6. Verify all services are healthy
make status
```

### Service Verification Commands

| Check       | Command                                | Expected Result                                    |
| ----------- | -------------------------------------- | -------------------------------------------------- |
| Backend API | `curl http://localhost:8080/health`    | `{"status":"healthy","storage_mode":"postgresql"}` |
| Frontend UI | `curl -I http://localhost:3000`        | HTTP 200 OK                                        |
| PostgreSQL  | `docker ps \| grep postgres`           | Container running                                  |
| Ollama      | `curl http://localhost:11434/api/tags` | List of models                                     |

### LLM Provider Selection

EdgeQuake supports two LLM providers at runtime:

| Provider             | When to Use                              | Setup                            |
| -------------------- | ---------------------------------------- | -------------------------------- |
| **Ollama** (default) | Development, local testing, no API costs | `ollama serve &`                 |
| **OpenAI**           | Production, higher quality extraction    | `export OPENAI_API_KEY="sk-..."` |

**Important:** If using OpenAI, prefer `gpt-5-nano` over deprecated `gpt-4o-mini`.

### Testing After Code Changes

```bash
# Quick test for specific crate
cargo test -p edgequake-api --lib

# Full test suite (641+ tests)
cargo test --workspace --lib

# Linting (must pass before commit)
cargo clippy --all-targets

# Format check
cargo fmt --check
```

### Common Development Scenarios

#### Scenario 1: Testing PDF Upload

```bash
# 1. Ensure services are running
make status

# 2. Open browser to documents page
open http://localhost:3000/documents

# 3. Upload a test PDF from:
#    - zz_test_docs/lighrag_2410.05779v3.pdf
#    - zz-explore/EMILE_FREY/*.pdf

# 4. Watch status change: Uploading → Processing → Completed
```

#### Scenario 2: Debugging Entity Extraction

```bash
# Check Ollama is responding
curl http://localhost:11434/api/tags

# View backend logs for extraction details
tail -f /tmp/edgequake-backend.log | grep -i entity

# If extraction fails, check pipeline errors:
grep -i "error\|failed" /tmp/edgequake-backend.log
```

#### Scenario 3: Database Issues

```bash
# Check if PostgreSQL is running
docker ps | grep edgequake-postgres

# Restart database if needed
make postgres-stop
make postgres-start

# Wait for database to be ready
sleep 5

# Restart backend
make backend-bg
```

### Environment Variables Reference

| Variable                       | Required | Purpose                         | Example                                              |
| ------------------------------ | -------- | ------------------------------- | ---------------------------------------------------- |
| `DATABASE_URL`                 | ✅ Yes   | PostgreSQL connection           | `postgres://edgequake:edgequake@localhost/edgequake` |
| `OPENAI_API_KEY`               | Optional | Enable OpenAI provider          | `sk-proj-...`                                        |
| `EDGEQUAKE_LLM_PROVIDER`       | Optional | Override LLM provider           | `openai`, `ollama`, `lmstudio`, `mock`               |
| `EDGEQUAKE_EMBEDDING_PROVIDER` | Optional | Hybrid mode: separate embedding | `ollama` (use with `EDGEQUAKE_LLM_PROVIDER=openai`)  |
| `OLLAMA_HOST`                  | Optional | Ollama server URL               | `http://localhost:11434`                             |
| `OLLAMA_EMBEDDING_MODEL`       | Optional | Ollama embedding model          | `embeddinggemma:latest`                              |
| `RUST_LOG`                     | Optional | Logging level                   | `debug`, `info`, `warn`                              |

### Hybrid Provider Mode (SPEC-033)

Use different providers for LLM and embeddings. Useful when:

- OpenAI has LLM quota but not embedding quota
- Cost savings (free local embeddings with cloud LLM)
- Privacy (local embeddings, cloud LLM quality)

```bash
# Example: OpenAI for LLM, Ollama for embeddings
export EDGEQUAKE_LLM_PROVIDER=openai
export EDGEQUAKE_EMBEDDING_PROVIDER=ollama
export OPENAI_API_KEY=sk-...
export OLLAMA_HOST=http://localhost:11434
```

### Troubleshooting Quick Reference

| Problem                      | Quick Fix                                              |
| ---------------------------- | ------------------------------------------------------ |
| "DATABASE_URL not set"       | Run `make dev` instead of `cargo run`                  |
| "Connection refused on 8080" | Check PostgreSQL: `make postgres-start`                |
| "Entity extraction failed"   | Start Ollama: `ollama serve &`                         |
| "Model not found"            | Pull model: `ollama pull gemma3:latest`                |
| "Port 3000 in use"           | Kill stale process: `lsof -ti:3000 \| xargs kill`      |
| Tests failing                | Run `cargo test -p <crate> --lib` for details          |
| "Embedding quota exceeded"   | Use hybrid mode: `EDGEQUAKE_EMBEDDING_PROVIDER=ollama` |

### Best Practices (Mission Learnings)

1. **Always use Makefile commands** - They set required environment variables
2. **Check `make status` before debugging** - Verify all services are healthy
3. **DATABASE_URL is mandatory** - In-memory mode is removed for reliability
4. **Ollama must be running** - Entity extraction depends on it
5. **Use `gpt-5-nano`** - If using OpenAI, avoid deprecated `gpt-4o-mini`
6. **Run tests after changes** - `cargo test -p <crate> --lib` for quick feedback
7. **Commit frequently** - Small, tested changes are easier to debug
8. **Use hybrid mode for quota issues** - OpenAI LLM + Ollama embeddings

## LLM Provider Configuration

EdgeQuake supports multiple LLM providers with automatic environment-based selection:

- **Mock Provider**: Used by default for testing (free, fast, no API key required)
- **OpenAI Provider**: Automatically used when `OPENAI_API_KEY` is set
  - Recommended model: `gpt-5-nano` (cost-effective, excellent for entity extraction)
  - Alternative: `gpt-4o-mini` is deprecated; migrate to `gpt-5-nano`
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

Use SRP and DRY principles when developing new features or fixing bugs. For example, if you find yourself copying and pasting code, consider refactoring it into a reusable function or module. This not only reduces code duplication but also makes maintenance easier in the long run. Always aim for clean, modular code that adheres to the project's coding standards and conventions.
