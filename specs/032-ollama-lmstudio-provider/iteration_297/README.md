# OODA-297: E2E Quality Gates CI Workflow

## Date: 2026-01-16
## Status: ✅ COMPLETE

## Objective
Create GitHub Actions workflow for E2E Playwright tests with quality gates.

## Created Workflow

**File:** `.github/workflows/e2e-quality-gates.yml`

## Workflow Structure

```
e2e-setup (build backend)
    ↓
e2e-critical → e2e-dialogs
    ↓              ↓
         e2e-full
              ↓
       e2e-summary
```

## Quality Gates

### 1. Critical Path Tests
- **File:** `ooda-228-critical-path.spec.ts`
- **Target:** <30s
- **Pass Rate:** 100% required
- **Tests:** 3 critical path validations

### 2. Dialog Tests
- **File:** `spec032-tenant-workspace-dialogs.spec.ts`
- **Tests:** 17 dialog interactions
- **Pass Rate:** 100% required

### 3. Full Suite
- **Target:** <5 minutes (300s)
- **Pass Rate:** >85% required
- **Tests:** 534+ tests

## Service Configuration

```yaml
services:
  postgres:
    image: apache/age:v1.5.0-pg16
    ports: 5432:5432
```

- Backend: Release build with mock LLM
- Frontend: pnpm dev server

## Timing Thresholds

| Suite | Target | Baseline |
|-------|--------|----------|
| Critical Path | 30s | 995ms |
| Dialogs | 30s | 5.5s |
| Full Suite | 300s | 256s |

## Pass Rate Requirements

| Suite | Minimum | Baseline |
|-------|---------|----------|
| Critical | 100% | 100% |
| Dialogs | 100% | 100% |
| Full | 85% | 90.4% |

## Artifacts

```
e2e-results/
├── e2e_output.json    # Full test results
└── screenshots/       # Failure screenshots
```

## Trigger Conditions

Only runs when relevant paths change:
- `edgequake_webui/**`
- `edgequake/crates/edgequake-api/**`

## Next Steps (OODA 298+)

1. Add visual regression testing
2. Add performance budget checks
3. Create nightly full E2E run
4. Add cross-browser testing

## Invariants Protected

- INV-012: Critical path tests always pass
- INV-013: E2E pass rate >85%
- INV-014: E2E suite <5 minutes
