# E2E Test Service Management SKILL

**Purpose**: Manage PostgreSQL, Rust backend, and Next.js frontend services for interactive E2E testing.

## Quick Links

- **[Complete SKILL Documentation](SKILL.md)** - Full reference with examples
- **[Utility Script](test-services.sh)** - Bash script for common operations

## 30-Second Quick Start

```bash
# Start all services
make dev

# Check they're running
make status

# Stop when done
make stop
```

## Common Tasks

| Task                              | Command                                                   |
| --------------------------------- | --------------------------------------------------------- |
| Start frontend, backend, database | `make dev`                                                |
| Stop all services                 | `make stop`                                               |
| Check service health              | `make status`                                             |
| View backend logs                 | `make backend-logs`                                       |
| Open database shell               | `make db-shell`                                           |
| Run E2E tests                     | `cd edgequake_webui && pnpm exec playwright test`         |
| Debug test                        | `cd edgequake_webui && pnpm exec playwright test --debug` |

## When to Use This SKILL

✅ Running E2E tests with Playwright  
✅ Interactive manual testing  
✅ Debugging backend issues  
✅ Database inspection  
✅ Testing with real vs mock LLM providers  
✅ Service health monitoring

## Service Architecture

```
Frontend (Next.js)      Port 3000
    ↓
Backend (Rust API)      Port 8080
    ↓
Database (PostgreSQL)   Port 5432
```

## Key Features

- 🚀 **One-command startup**: `make dev` starts all three services
- ✅ **Health checks**: `make status` verifies everything works
- 📋 **Service commands**: Individual control of each service
- 🔍 **Log monitoring**: Real-time logs for debugging
- 🛠️ **Utility script**: `test-services.sh` for automation
- 🐛 **Troubleshooting**: Complete guide for common issues
- 📚 **Examples**: Real code samples for common patterns

## Related SKILLs

- [makefile-dev-workflow](../makefile-dev-workflow/) - Complete development reference
- [playwright-ux-ui-capture](../playwright-ux-ui-capture/) - Capture UI workflows
- [ux-ui-analyze-single-page](../ux-ui-analyze-single-page/) - Analyze individual pages

## Session Context

Created during E2E testing of the "New Query Button" bug fix. This SKILL encapsulates best practices and lessons learned from an intensive testing session.

**Date**: December 27, 2025  
**Status**: Production Ready ✅

---

For complete documentation, see [SKILL.md](SKILL.md)
