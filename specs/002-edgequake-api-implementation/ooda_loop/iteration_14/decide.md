# OODA Iteration 14 — Decide: TypeScript SDK Action Plan

## Priority Actions

1. **P1**: Fix conversation/folder test failures — make them skip when EDGEQUAKE_TENANT_ID is not set
2. **P2**: Create a quick-verification test that runs the 28-step mandatory E2E sequence from the spec
3. **P3**: Verify test report with evidence

## Decision

- The conversation/folder tests already have `skipUnless(E2E_TENANT_ID)` logic but it's not working properly for some tests — need to verify and fix
- Since 48/62 tests pass (77%) and the failures are all in one test file with a clear root cause, the TypeScript SDK is in good shape
- Focus remediation on making conversation/folder tests properly skip instead of fail
