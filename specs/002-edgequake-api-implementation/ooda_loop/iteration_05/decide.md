# Iteration 05: Decide — E2E Tests & Examples

## Date: 2026-02-11

## Decisions

### Priority Actions

1. Create E2E test helpers (client factory, utilities) ✅
2. Create 4 E2E test suites (health, documents, query, graph) ✅
3. Add 2 examples (error_handling, configuration) → 10 total ✅
4. Verify all unit tests still pass with E2E tests skipped ✅
5. Commit IMPL-05

### Design Choices

- **Environment gating**: `EDGEQUAKE_E2E_URL` env var → `describe.skip` when absent
- **Test isolation**: `testId()` generates unique names per run
- **Cleanup**: `afterAll` hooks delete test resources
- **No new dependencies**: Pure vitest + native fetch, no MSW
- **Examples are runnable**: Use `npx tsx examples/error_handling.ts`

### What NOT to Do

- No MSW integration tests (unit mocks sufficient)
- No `make dev` in CI (E2E tests are opt-in)
- No npm publish (needs NPM_TOKEN, out of scope)
