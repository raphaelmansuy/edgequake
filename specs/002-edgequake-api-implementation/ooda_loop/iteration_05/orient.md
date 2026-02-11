# Iteration 05: Orient — E2E Tests & Examples

## Date: 2026-02-11

## Analysis

### E2E Test Architecture Decision

```
Option A: MSW (Mock Service Worker)  → Adds devDependency, intercepts HTTP
Option B: Environment-gated real E2E → No new deps, tests real server
Option C: Both layers               → Most complete, more complexity
```

**Decision**: Option B — environment-gated E2E tests.

**Rationale**:

- Unit tests already thoroughly cover SDK logic with mock transport
- MSW would add another abstraction layer testing similar paths
- Real E2E tests against `make dev` backend catch serialization/auth/timing issues
- Zero new dependencies — clean and lightweight
- `describe.skip` when no backend → `npm test` passes in CI without server

### E2E Test Coverage Plan

```
tests/e2e/
├── helpers.ts        → Client factory, waitFor, testId, sleep utilities
├── health.test.ts    → Health, readiness, liveness, provider status (8 tests)
├── documents.test.ts → Upload, status, list, get, delete lifecycle (5 tests)
├── query.test.ts     → Execute, stream, chat, with modes (6 tests)
└── graph.test.ts     → Entity CRUD, search, neighborhood, relationships (7 tests)
```

Total: 26 E2E tests covering 4 major feature areas.

### Examples Gap Analysis

Need 2 more to hit 10+ target:

- `error_handling.ts` — 5 patterns: specific types, retry, degradation, validation, catch-all
- `configuration.ts` — 6 patterns: minimal, explicit, env-based, multi-tenant, factory, health check
