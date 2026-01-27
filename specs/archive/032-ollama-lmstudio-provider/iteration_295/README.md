# OODA-295: CI Quality Gates Workflow

## Date: 2026-01-16

## Status: ✅ COMPLETE

## Objective

Create GitHub Actions workflow with timing gates and invariant verification.

## Created Workflow

**File:** `.github/workflows/test-quality-gates.yml`

## Quality Gates Implemented

### 1. Unit Tests Gate

- **Target:** <30s execution time
- **Timeout:** 5 minutes
- **Warning:** If exceeds threshold

### 2. Invariant Tests Gate

- **Tests:** INV-001 to INV-010 (12 unit tests)
- **Tests:** Edge case invariants (32 tests)
- **Tests:** Integration invariants (7 tests)
- **Failure Mode:** CRITICAL - blocks merge

### 3. API Tests Gate

- **Target:** <30s execution time
- **Scope:** edgequake-api crate (415+ tests)

### 4. Optimized Tests Gate

- **LLM Tests:** Must stay <10s (was 4.69s)
- **Rate Limiter:** Must stay <5s (was 2.6s)
- **Purpose:** Detect performance regressions

### 5. Test Count Gate

- **Minimum:** 2,600 tests
- **Baseline:** OODA-286 measured 2,665+
- **Purpose:** Prevent test deletion

## Workflow Jobs

```yaml
jobs:
  unit-tests: # Fast feedback (<30s target)
  invariant-tests: # Reliability verification
  api-tests: # API layer coverage
  optimized-tests: # Regression detection
  test-count-gate: # Test coverage maintenance
  quality-summary: # Aggregate results
```

## Timing Thresholds

| Gate         | Threshold | Baseline      |
| ------------ | --------- | ------------- |
| Unit Tests   | 30s       | ~8s           |
| LLM Tests    | 10s       | 4.69s → 2.13s |
| Rate Limiter | 5s        | 2.6s → 0.1s   |
| Full Suite   | 5min      | ~2.5min       |

## Failure Modes

1. **Invariant Failure** → CRITICAL, blocks merge
2. **Timing Regression** → Warning
3. **Test Count Drop** → Failure, blocks merge

## Integration with Existing CI

This workflow complements the existing `ci.yml`:

- `ci.yml`: Format, clippy, build, coverage
- `test-quality-gates.yml`: Timing, invariants, test count

## Next Steps (OODA 296+)

1. Add E2E Playwright gates
2. Add flaky test detection
3. Add coverage thresholds
4. Create PR status checks

## Invariants Protected

- INV-001 to INV-010: All tested
- Test count: ≥2,600 enforced
- Performance: Regression detection active
