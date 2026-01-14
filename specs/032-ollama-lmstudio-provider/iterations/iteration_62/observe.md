# OODA 62 - Observe: E2E Test Robustness Audit

## Current State

With OODA 61 complete, the TenantGuard race condition has been resolved. Now focusing on test robustness.

## Test Suite Analysis

### Current E2E Test Status
- 9/9 tests passing
- Total run time: ~11.2s
- Slowest tests: deeplink tests (~6-10s each)

### Observations

1. **Deeplink Test Timing**
   - `workspace deeplink by slug resolves correctly`: 9.3s
   - `invalid workspace slug shows error state`: 10.3s
   - `/w/[slug] redirects to /w/[slug]/query`: 6.8s
   - These tests are slower than API tests (< 1s each)

2. **Test Assertions**
   - Some tests use breadcrumb fallback assertions (from OODA 60)
   - With TenantGuard fix (OODA 61), these fallbacks may no longer be needed

3. **Test Coverage Gaps**
   - No test for streaming fallback behavior (Focus 8)
   - No test for model switching at runtime
   - No test for concurrent queries

4. **Potential Improvements**
   - Remove overly defensive assertions now that race condition is fixed
   - Add streaming-specific tests
   - Add model selection UI tests

## Questions to Answer

1. Can we simplify deeplink test assertions now that TenantGuard is removed?
2. Should we add streaming tests to the E2E suite?
3. Are there any flaky test patterns we should address?
