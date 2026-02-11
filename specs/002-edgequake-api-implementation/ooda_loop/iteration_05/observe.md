# Iteration 05: Observe — E2E Tests & Examples

## Date: 2026-02-11

## Observations

### Design Spec Testing Strategy

Read `specs/api_design/typescript/10-testing-strategy.md` (225 lines). It defines 3 test layers:

```
Unit Tests (mock transport) → 243 tests ✅ DONE
Integration Tests (mock HTTP server) → MSW-based, moderate complexity
E2E Tests (real server) → Gated by EDGEQUAKE_E2E_URL env var
```

### Rust E2E Test Reference

Analyzed `edgequake/crates/edgequake-api/tests/e2e_pipeline_robustness.rs` — tests include:

- Health check structure & provider detection
- Pipeline status & queue metrics
- Cost estimation, pricing, summary, history, budget
- Provider status
- Document upload status tracking

### Current Test State

- 243 unit tests with mock transport, 98.52% line coverage
- Zero integration or E2E tests

### Examples Gap

- 8 examples exist (basic, upload, query, graph, streaming, websocket, multi-tenant, batch)
- Mission spec targets "10+ working examples per SDK"
- Missing: error handling patterns, configuration patterns

### Backend Availability

- `make dev` starts database + backend + frontend
- Backend health endpoint: `http://localhost:8080/health`
- E2E tests need real server — must be gated by environment variable
