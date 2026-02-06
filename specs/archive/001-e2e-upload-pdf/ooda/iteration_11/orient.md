# OODA-11 Orient: Timeout Strategy Analysis

## Root Cause

No timeout enforcement exists in any test file. Tests rely on tokio's default behavior (no timeout).

## Options Considered

### Option A: Modify all 591 tests with `tokio::time::timeout`

- **Pro**: Complete coverage
- **Con**: Extremely high risk, massive diff, fragile
- **Verdict**: Rejected — too invasive

### Option B: Create `#[timeout]` proc macro

- **Pro**: Clean API
- **Con**: Adds build dependency, maintenance overhead
- **Verdict**: Rejected — overengineered

### Option C: Create dedicated timeout enforcement test file (CHOSEN)

- **Pro**: Zero risk to existing tests, demonstrates pattern, validates critical paths
- **Con**: Doesn't modify existing tests directly
- **Verdict**: Best balance of safety and coverage

### Option D: Add `with_timeout()` helper to shared test utils

- **Pro**: Reusable, can be adopted incrementally
- **Con**: Requires shared crate for test utilities
- **Verdict**: Future enhancement (OODA-19)

## Decision Matrix

| Criterion              | A    | B      | C              | D           |
| ---------------------- | ---- | ------ | -------------- | ----------- |
| Risk to existing tests | High | Medium | None           | Low         |
| Coverage               | 100% | 100%   | Critical paths | Incremental |
| Implementation effort  | Days | Hours  | 1 hour         | Hours       |
| Maintenance            | High | Medium | Low            | Low         |

## First Principles

- Tests should fail fast, not hang forever
- Mock tests should complete in <1s
- Real LLM tests need 120s budget
- CI/CD needs bounded execution time
