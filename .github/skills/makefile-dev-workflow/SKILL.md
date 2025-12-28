---
name: makefile-dev-workflow
description: Unified development workflow for EdgeQuake using Makefile commands. Use when starting services, running tests, or managing the full development stack (database, backend, frontend). Provides simplified alternatives to raw cargo/npm commands.
license: Proprietary (repository internal)
compatibility: Requires make, cargo, Node.js/pnpm, and Docker Desktop with docker-compose support.
metadata:
  repo: raphaelmansuy/edgequake
  area: dev-infrastructure
  file: Makefile (333 lines, comprehensive)
---

# Makefile Development Workflow

## When to use

Use this skill when you need to:

- Start development services (database, backend API, frontend)
- Stop services cleanly
- Run end-to-end tests with Playwright
- Check service health status
- Build and deploy applications
- Clean build artifacts
- Manage the full development stack with a single command

## Core concepts

The repository includes a unified **Makefile** (at repo root) that wraps complex shell commands into simple targets. This replaces scattered docker-compose commands and cargo/npm invocations with a consistent interface.

### Key directories

- `edgequake/`: Rust backend API (port 8080)
- `edgequake_webui/`: Next.js frontend (port 3000)
- `edgequake/docker/`: Docker configuration with docker-compose.yml (contains PostgreSQL + pgvector + Apache AGE)

## Quick start commands

### Start full development stack (all services)

```bash
make dev
```

Starts PostgreSQL, Rust backend, and Next.js frontend in parallel. Displays URLs for all services.

### Stop all services

```bash
make stop
```

Kills all running processes and stops Docker containers gracefully.

### Check service status

```bash
make status
```

Shows health of backend, frontend, and database with actual health checks and endpoints.

## Service-specific commands

### Database management

```bash
make db-start     # Start PostgreSQL container (port 5432)
make db-stop      # Stop PostgreSQL gracefully
make db-logs      # View PostgreSQL logs in real-time
make db-shell     # Open psql shell into the database
make db-reset     # DANGER: Delete all data and reinitialize database
```

### Backend (Rust) management

```bash
make backend-dev     # Run backend in development mode with hot reload
make backend-build   # Build backend for release
make backend-test    # Run backend tests (uses mock LLM provider by default)
make backend-run     # Run compiled backend binary
make backend-clippy  # Lint backend code (strict)
make backend-fmt     # Format backend code with rustfmt
```

### Frontend (Next.js) management

```bash
make frontend-dev    # Start frontend dev server with Turbopack (hot reload)
make frontend-build  # Build frontend for production
make frontend-start  # Start production frontend server
make frontend-lint   # Lint frontend code (ESLint)
make frontend-test   # Run frontend unit tests
```

## E2E Testing with Playwright

### Run all E2E tests

```bash
cd edgequake_webui && pnpm exec playwright test
```

Runs all test specs in `e2e/` directory with HTML reporter.

### Run specific E2E test

```bash
cd edgequake_webui && pnpm exec playwright test markdown-test.spec.ts
```

### Core query page tests

```bash
cd edgequake_webui && pnpm exec playwright test \
  markdown-test.spec.ts \
  streaming-test.spec.ts \
  live-query-test.spec.ts \
  final-validation.spec.ts
```

These tests verify:

- **markdown-test**: Markdown rendering in responses (no raw `**` showing)
- **streaming-test**: Streaming text handling without concatenation issues
- **live-query-test**: End-to-end query execution with real LLM response
- **final-validation**: Complete query interface functionality

### View test report

```bash
cd edgequake_webui && pnpm exec playwright show-report
```

Opens HTML report of last test run showing pass/fail, screenshots, traces.

## Docker stack commands

### Build all Docker images

```bash
make docker-build
```

### Start full Docker stack (API + PostgreSQL)

```bash
make docker-up
```

Starts containerized EdgeQuake API and PostgreSQL with automatic health checks.

### Stop Docker stack

```bash
make docker-down
```

### View Docker logs

```bash
make docker-logs
```

### Check Docker container status

```bash
make docker-ps
```

## Code quality commands

### Lint all code

```bash
make lint
```

Runs `cargo clippy` (Rust) and ESLint (frontend) with strict warnings-as-errors.

### Format all code

```bash
make format
```

Applies rustfmt to Rust code and prettier to frontend code.

### Run all tests

```bash
make test
```

Runs both backend tests (with mock LLM) and frontend tests in parallel.

### Build all projects

```bash
make build
```

Produces release binaries for both backend and frontend.

## Dependency checks

### Verify dependencies

```bash
make check-deps
```

Verifies that required tools are installed:

- `cargo` (Rust toolchain)
- `bun` or `npm` (Node.js)
- `docker` (optional, required for db-start/Docker commands)

If dependencies are missing, installation instructions are provided.

## Installation

### Install all dependencies

```bash
make install
```

Installs Rust crates and frontend npm packages.

## Cleanup

### Clean build artifacts

```bash
make clean
```

Removes `target/` directory and `.next/` build caches.

### Clean everything (including dependencies)

```bash
make clean-all
```

Also removes `node_modules/` - forces full reinstall on next run.

## Utilities

### Open Swagger UI

```bash
make swagger
```

Opens browser to `http://localhost:8080/swagger-ui` (API documentation).

### View recent logs

```bash
make logs
```

Shows recent backend logs and Docker container status.

## Common workflows

### Development workflow

```bash
# 1. Install dependencies
make install

# 2. Start full stack
make dev

# 3. Make changes to code...

# 4. Check code quality before committing
make lint
make format
make test

# 5. Run E2E tests to verify UI
cd edgequake_webui && pnpm exec playwright test

# 6. Stop when done
make stop
```

### Testing workflow

```bash
# Start database (for integration tests)
make db-start

# Run backend tests
make backend-test

# Run frontend tests
make frontend-test

# Run E2E tests
cd edgequake_webui && pnpm exec playwright test markdown-test.spec.ts

# View results
make status
```

### Production build workflow

```bash
# Build both projects
make build

# Start Docker stack with built images
make docker-up

# Check it's healthy
make status

# Stop when done
make docker-down
```

## Service URLs

| Service     | URL                              | Purpose                      |
| ----------- | -------------------------------- | ---------------------------- |
| Frontend    | http://localhost:3000            | Main application UI          |
| Backend API | http://localhost:8080            | REST API endpoints           |
| Swagger UI  | http://localhost:8080/swagger-ui | API documentation            |
| Database    | localhost:5432                   | PostgreSQL with pgvector/AGE |

## Environment variables

Configure these in `.env` file (copy from `.env.example`):

```bash
# LLM Provider (auto-detected from this)
OPENAI_API_KEY=sk-...

# Database (defaults to Docker PostgreSQL)
DATABASE_URL=postgres://edgequake:edgequake_secret@localhost:5432/edgequake

# Server config
EDGEQUAKE_PORT=8080
EDGEQUAKE_HOST=0.0.0.0

# Logging
RUST_LOG=info,edgequake=debug
```

Without `OPENAI_API_KEY`, the system uses a mock LLM provider (free, fast, no API key).

## Troubleshooting

### Port already in use

If port 3000 or 8080 is in use:

```bash
# Kill existing process on port 3000
lsof -ti :3000 | xargs kill -9

# Kill existing process on port 8080
lsof -ti :8080 | xargs kill -9
```

### Database connection issues

```bash
# Reset database (deletes all data!)
make db-reset

# Check if database is accepting connections
make status
```

### Frontend build issues

```bash
# Clean and reinstall
make clean-all
make install
make frontend-dev
```

### Tests timing out

```bash
# Increase Playwright timeout in playwright.config.ts
# Or run tests with more time:
cd edgequake_webui && pnpm exec playwright test --timeout=60000
```

## Implementation notes

- **No docker-compose.yml at root**: Use `edgequake/docker/docker-compose.yml` instead
- **Package manager**: Frontend uses pnpm (fast, space-efficient) but falls back to npm
- **Rust hot reload**: Uses `cargo run` for development (recompiles on save)
- **Next.js dev**: Uses Turbopack for faster iterations than Webpack
- **Port isolation**: Services communicate via localhost ports; no environment routing needed

## Help

```bash
make help
```

Shows formatted help with all available targets and descriptions.

---

## Related Skills

- **[playwright-ux-ui-capture](./../playwright-ux-ui-capture/SKILL.md)**: Using Playwright to capture screenshots and artifacts
- **[ux-ui-analyze-single-page](./../ux-ui-analyze-single-page/SKILL.md)**: Analyzing individual pages with Playwright
