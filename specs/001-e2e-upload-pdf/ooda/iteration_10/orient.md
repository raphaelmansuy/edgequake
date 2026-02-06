# OODA-10 Orient: Clean Tenant Strategy

## Analysis

### Option A: Workspace-scoped headers (rejected)

- Send X-Tenant-ID + X-Workspace-ID with every request
- Pro: Full multi-tenancy isolation at API level
- Con: Workspace pipeline tries to create real LLM providers → fails in test mode
- Verdict: NOT suitable for in-memory mock tests

### Option B: Fresh AppState per test (selected)

- Each TestContext creates `AppState::test_state()` → separate in-memory store
- Pro: Complete isolation by construction, no external dependencies
- Pro: Follows existing 444-test pattern that already works
- Con: Doesn't test multi-tenant scoping at storage layer
- Verdict: BEST for unit/integration tests

### Option C: Hybrid approach (implemented)

- Fresh AppState per test (isolation)
- ALSO create tenant (proves API works)
- Don't send workspace headers for document ops (uses global mock pipeline)
- Pro: Tests both isolation AND tenant API
- Verdict: SELECTED

## Decision

Use Option C: Create fresh state + tenant per test, but use global mock pipeline
for document operations. This proves the tenant/workspace API while maintaining
test compatibility with the mock provider.
