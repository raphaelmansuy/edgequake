---
name: e2e-test-service-management
description: Service management for E2E testing in EdgeQuake. Start, stop, and monitor PostgreSQL, backend API, and frontend services. Includes health checks and logging utilities for interactive testing workflows.
license: Proprietary (repository internal)
compatibility: Requires make, cargo, Node.js/pnpm, Docker Desktop with docker-compose support, and curl for health checks.
metadata:
  repo: raphaelmansuy/edgequake
  area: testing
  dependencies: makefile-dev-workflow
  related_skills:
    - makefile-dev-workflow
    - playwright-ux-ui-capture
    - ux-ui-analyze-single-page
  file: TESTING_SERVICES.sh (utility script, optional)
---

# E2E Test Service Management

## When to Use This Skill

Use this skill when you need to:

- **Start all services** for interactive E2E testing
- **Verify service health** before running tests
- **Monitor logs** during test execution
- **Stop services cleanly** after testing
- **Debug test failures** with real backend/database
- **Test with different configurations** (mock LLM vs real LLM)
- **Manage service lifecycle** in CI/CD pipelines

### Perfect For

✅ **E2E Testing Workflows** - Running Playwright tests with real services  
✅ **Interactive Debugging** - Testing features manually with live backend  
✅ **Service Isolation** - Starting only specific services  
✅ **Health Monitoring** - Detecting service crashes/failures  
✅ **Log Inspection** - Troubleshooting backend/database issues  
✅ **Test Isolation** - Cleaning services between test runs

---

## Service Architecture

### Services Overview

```
┌─────────────────────────────────────────────┐
│           E2E Testing Environment            │
├─────────────────────────────────────────────┤
│                                              │
│  Frontend (Next.js)         Port 3000        │
│  ├─ Playwright Tests                        │
│  ├─ Hot Reload Development                  │
│  └─ TypeScript/React Components              │
│                                              │
│  Backend (Rust)             Port 8080        │
│  ├─ RAG Query Engine                        │
│  ├─ Document Processing                     │
│  ├─ Knowledge Graph API                     │
│  └─ LLM Integration                         │
│                                              │
│  Database (PostgreSQL)       Port 5432       │
│  ├─ pgvector Extension (embeddings)         │
│  ├─ Apache AGE (graph storage)              │
│  ├─ Conversation History                    │
│  └─ Document Metadata                       │
│                                              │
└─────────────────────────────────────────────┘
```

### Service Dependencies

```
Frontend (3000)
    ↓
Backend (8080)
    ↓
Database (5432)
```

Services should be started in reverse dependency order: Database → Backend → Frontend  
Services should be stopped in dependency order: Frontend → Backend → Database

---

## Quick Start

### Start Full Stack (Recommended for Testing)

```bash
make dev
```

**What this does:**

1. Starts PostgreSQL container (port 5432)
2. Runs backend in development mode (port 8080)
3. Starts frontend dev server (port 3000)
4. Displays URLs and health status

**Output example:**

```
✓ PostgreSQL started on localhost:5432
✓ Backend running on http://localhost:8080
✓ Frontend ready on http://localhost:3000
```

**Wait time:** 15-30 seconds (depends on system)

### Stop All Services

```bash
make stop
```

Stops all services gracefully in the correct order.

### Check Service Status

```bash
make status
```

Returns health status of all three services:

```
🟢 Frontend: http://localhost:3000 (200 OK)
🟢 Backend:  http://localhost:8080 (200 OK)
🟢 Database: localhost:5432 (responding)
```

---

## Service-Specific Management

### Database (PostgreSQL)

#### Start PostgreSQL Only

```bash
make db-start
```

Starts PostgreSQL container with pgvector and Apache AGE extensions.

**Configuration:**

- Container name: `edgequake-postgres`
- Image: `postgres:15-alpine` with custom extensions
- Volume: `edgequake_postgres_data` (persistent)
- Port mapping: `5432:5432`

#### Stop PostgreSQL

```bash
make db-stop
```

Stops container gracefully (saves state).

#### View Database Logs

```bash
make db-logs
```

Streams PostgreSQL logs in real-time for debugging connection issues.

#### Open Database Shell

```bash
make db-shell
```

Opens interactive `psql` terminal to:

- Run SQL queries directly
- Inspect conversation history
- Check embeddings storage
- View entity relationships

#### Reset Database (⚠️ DANGER)

```bash
make db-reset
```

**WARNING:** Deletes ALL data and reinitializes database. Use only for:

- Cleaning up between test cycles
- Starting with fresh state
- Removing test artifacts

---

### Backend (Rust API)

#### Start Backend in Development Mode

```bash
make backend-dev
```

Starts backend with:

- Hot reload on code changes
- Debug logging enabled
- Port 8080 (configurable via `BACKEND_PORT`)
- Automatic database migration

**Important Environment Variables:**

```bash
# Test with real OpenAI
export OPENAI_API_KEY="sk-your-key-here"
make backend-dev

# Test with mock provider (default)
make backend-dev  # Uses mock by default
```

#### Build Backend

```bash
make backend-build
```

Creates optimized release binary (slower build, faster runtime).

#### Run Backend Binary

```bash
make backend-run
```

Runs pre-compiled backend binary (requires `backend-build` first).

#### Run Backend Tests

```bash
make backend-test
```

Runs all Rust tests:

- Uses mock LLM provider by default (fast, free)
- To use real OpenAI: `OPENAI_API_KEY=sk-... make backend-test`

#### Format & Lint Backend

```bash
make backend-fmt      # Format code with rustfmt
make backend-clippy   # Run clippy linter
```

---

### Frontend (Next.js)

#### Start Frontend Dev Server

```bash
make frontend-dev
```

Starts development server with:

- Turbopack for fast hot reload
- Port 3000
- TypeScript compilation
- Live error overlay

#### Build Frontend

```bash
make frontend-build
```

Creates optimized production build in `.next/`.

#### Start Production Frontend

```bash
make frontend-start
```

Runs production-optimized server (requires `frontend-build` first).

#### Run Frontend Tests

```bash
make frontend-test
```

Runs Jest unit tests for React components.

#### Lint Frontend Code

```bash
make frontend-lint
```

Runs ESLint to check for code quality issues.

---

## E2E Testing Workflow

### Typical Test Execution

```bash
# 1. Start all services
make dev
make status              # Verify all are running

# 2. Run Playwright tests
cd edgequake_webui
pnpm exec playwright test

# 3. View test results
# Reports generated in: playwright-report/

# 4. Stop services (when done)
make stop
```

### Running Specific E2E Tests

```bash
# Test a specific file
cd edgequake_webui
pnpm exec playwright test e2e/query-page.spec.ts

# Test with specific browser
pnpm exec playwright test --project=chromium

# Run in headed mode (see browser)
pnpm exec playwright test --headed

# Debug mode (pause on failures)
pnpm exec playwright test --debug
```

### Interactive E2E Testing (with MCP Tools)

```bash
# 1. Start services
make dev

# 2. Use Playwright MCP tools for interactive testing
# - mcp_microsoft_pla_browser_navigate
# - mcp_microsoft_pla_browser_click
# - mcp_microsoft_pla_browser_type
# - mcp_microsoft_pla_browser_take_screenshot

# 3. Inspect results and failures
# 4. Stop when complete
make stop
```

### Monitor Services During Testing

In separate terminal:

```bash
# Watch logs continuously
make db-logs &      # Terminal 1
make backend-logs & # Terminal 2
make frontend-logs &# Terminal 3

# Or view combined logs
tail -f edgequake/target/debug/backend.log
```

---

## Health Checks & Verification

### Manual Health Checks

```bash
# Frontend
curl -s http://localhost:3000 | head -20

# Backend API
curl -s http://localhost:8080/api/v1/health | jq

# Database connection
psql -h localhost -U edgequake -d edgequake -c "SELECT 1"
```

### Service Readiness Script

```bash
#!/bin/bash
# Wait for all services to be ready

echo "Waiting for services..."

# Frontend
until curl -s http://localhost:3000 > /dev/null 2>&1; do
  echo "  ⏳ Frontend not ready..."
  sleep 1
done
echo "✓ Frontend ready"

# Backend
until curl -s http://localhost:8080/api/v1/health > /dev/null 2>&1; do
  echo "  ⏳ Backend not ready..."
  sleep 1
done
echo "✓ Backend ready"

# Database
until psql -h localhost -U edgequake -d edgequake -c "SELECT 1" > /dev/null 2>&1; do
  echo "  ⏳ Database not ready..."
  sleep 1
done
echo "✓ Database ready"

echo "✅ All services ready for testing"
```

---

## Troubleshooting

### Services Won't Start

**Check logs:**

```bash
make db-logs      # Database issues
make backend-logs # Backend/API issues
make frontend-logs # Frontend issues
```

**Common issues:**

| Issue                                | Solution                                                                            |
| ------------------------------------ | ----------------------------------------------------------------------------------- |
| Port already in use (3000/8080/5432) | `make stop` then `make dev` OR change ports in `.env`                               |
| Database won't connect               | Ensure Docker is running: `docker ps`                                               |
| Backend panic on startup             | Check migrations: `make db-reset`                                                   |
| Frontend build errors                | Clear cache: `cd edgequake_webui && rm -rf .next node_modules && make frontend-dev` |

### Database Connection Issues

```bash
# Check PostgreSQL is running
docker ps | grep postgres

# Check logs
docker logs edgequake-postgres

# Test connection directly
psql -h localhost -U edgequake -d edgequake -c "SELECT version()"
```

### Backend Test Failures

```bash
# Run backend tests with real LLM
export OPENAI_API_KEY="sk-..."
make backend-test

# Run specific test
cargo test --package edgequake-core --test e2e_pipeline

# See detailed output
cargo test -- --nocapture
```

### Playwright Test Failures

```bash
# Run in headed mode to see what's happening
cd edgequake_webui && pnpm exec playwright test --headed

# Run with debug tracing
pnpm exec playwright test --debug

# View test reports
open playwright-report/index.html
```

---

## Environment Variables

### Key Configuration

```bash
# Backend
BACKEND_PORT=8080                    # Default
RUST_LOG=debug                       # Log level
OPENAI_API_KEY=sk-...               # Optional (mock by default)

# Frontend
NEXT_PUBLIC_API_URL=http://localhost:8080

# Database
POSTGRES_USER=edgequake
POSTGRES_PASSWORD=edgequake
POSTGRES_DB=edgequake
DATABASE_URL=postgresql://edgequake:edgequake@localhost:5432/edgequake
```

### Create `.env.local` for overrides

```bash
cat > .env.local << EOF
BACKEND_PORT=8080
OPENAI_API_KEY=sk-your-real-key
RUST_LOG=debug
EOF
```

---

## Best Practices for E2E Testing

### 1. Always Start with Fresh Services

```bash
make stop
make dev
sleep 5  # Wait for startup
```

### 2. Monitor Logs During Tests

```bash
# Terminal 1: Run tests
cd edgequake_webui && pnpm exec playwright test

# Terminal 2: Monitor backend
make backend-logs

# Terminal 3: Monitor database
make db-logs
```

### 3. Use Database Snapshots

```bash
# Before risky tests
docker exec edgequake-postgres \
  pg_dump -U edgequake edgequake > /tmp/backup.sql

# After failures, restore
docker exec -i edgequake-postgres \
  psql -U edgequake edgequake < /tmp/backup.sql
```

### 4. Test with Different LLM Providers

```bash
# Test with mock (fast, deterministic)
make backend-dev

# Test with real OpenAI (costs money, real behavior)
OPENAI_API_KEY=sk-... make backend-dev
```

### 5. Check Service Health Before Tests

```bash
make status
# All should show 🟢 (green)
# If not, wait and retry
```

---

## Common E2E Test Patterns

### Pattern 1: Test Initial Page Load

```bash
# Start services
make dev

# Run focused test
cd edgequake_webui
pnpm exec playwright test --grep "initial load"
```

### Pattern 2: Test with Real Data

```bash
# 1. Start services
make dev

# 2. Upload documents via UI or API
curl -X POST http://localhost:8080/api/v1/documents \
  -F "file=@document.pdf"

# 3. Run tests
cd edgequake_webui && pnpm exec playwright test
```

### Pattern 3: Test Error Handling

```bash
# 1. Start services
make dev

# 2. Stop backend to simulate failure
make backend-stop

# 3. Run tests to verify error handling
cd edgequake_webui && pnpm exec playwright test

# 4. Restart backend
make backend-dev
```

### Pattern 4: Performance Testing

```bash
# 1. Start services with resource monitoring
make dev

# 2. Monitor during test
watch -n 1 'docker stats edgequake-postgres edgequake-backend'

# 3. Run load tests
cd edgequake_webui
pnpm exec playwright test --workers=5  # Parallel execution
```

---

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: E2E Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Start services
        run: make dev &

      - name: Wait for services
        run: |
          until curl -s http://localhost:3000; do sleep 1; done
          until curl -s http://localhost:8080/api/v1/health; do sleep 1; done

      - name: Run E2E tests
        run: |
          cd edgequake_webui
          pnpm exec playwright test

      - name: Upload results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: playwright-report
          path: edgequake_webui/playwright-report/

      - name: Stop services
        if: always()
        run: make stop
```

---

## Related Skills

- **[makefile-dev-workflow](../makefile-dev-workflow/SKILL.md)** - Complete development command reference
- **[playwright-ux-ui-capture](../playwright-ux-ui-capture/SKILL.md)** - Capture and document UI workflows
- **[ux-ui-analyze-single-page](../ux-ui-analyze-single-page/SKILL.md)** - Analyze individual pages

---

## Command Reference Quick Lookup

| Task               | Command                                                   |
| ------------------ | --------------------------------------------------------- |
| Start all services | `make dev`                                                |
| Stop all services  | `make stop`                                               |
| Check status       | `make status`                                             |
| Backend dev        | `make backend-dev`                                        |
| Frontend dev       | `make frontend-dev`                                       |
| Database shell     | `make db-shell`                                           |
| Run E2E tests      | `cd edgequake_webui && pnpm exec playwright test`         |
| View test reports  | `open edgequake_webui/playwright-report/index.html`       |
| Debug test         | `cd edgequake_webui && pnpm exec playwright test --debug` |
| View backend logs  | `make backend-logs`                                       |
| View database logs | `make db-logs`                                            |
| Reset database     | `make db-reset`                                           |

---

## Session Reflection: Testing Insights

### What Worked Well

✅ **Service Start/Stop Simplification** - Using Make commands instead of manual docker/cargo calls saves time and prevents mistakes  
✅ **Browser Automation** - Interactive E2E testing with MCP tools provides fast feedback and debugging visibility  
✅ **Console Log Analysis** - Monitoring browser and backend logs during tests reveals issues quickly  
✅ **Isolated Service Management** - Starting only needed services reduces resource usage and startup time

### Challenges & Solutions

| Challenge                                 | Solution                                    | Benefit                 |
| ----------------------------------------- | ------------------------------------------- | ----------------------- |
| Services not responding when tests start  | Wait for health checks before running tests | Prevents flaky tests    |
| Stale data between test runs              | `make db-reset` between cycles              | Clean test isolation    |
| Difficult to track issues across services | Monitor logs in parallel terminals          | Faster debugging        |
| Forgetting to stop services               | `make stop` creates habit                   | Prevents port conflicts |
| Tests dependent on real LLM costs         | Default to mock, override with env var      | Faster feedback loops   |

### Recommendations

1. **Always check `make status`** before running tests
2. **Monitor logs in separate terminals** during test execution
3. **Use `db-reset` between test cycles** for clean state
4. **Start small** - test one feature, then expand
5. **Keep services running** for interactive debugging
6. **Use MCP tools** for browser automation and interactive testing

---

**Last Updated:** December 27, 2025  
**Tested in Session:** E2E Testing New Query Button Fix  
**Verified with:** Playwright MCP Tools + Service Management Commands
