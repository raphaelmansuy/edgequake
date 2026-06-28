# Iteration 03 - DECIDE

## Date: 2026-02-15

## Decision: Update Mission Baseline + Verify Remaining SDKs

### Rationale

The mission baseline is 70% incorrect regarding lineage support status. Continuing without correcting this would waste iterations implementing features that already exist.

### Priority Actions (This Iteration)

1. **Update SDK Coverage Matrix** with verified lineage status
2. **Run Ruby tests** to confirm lineage implementation
3. **Document total test count** across all SDKs

### Deferred Actions (Future Iterations)

- TypeScript E2E with live backend (needs CI/CD setup)
- PHP test execution (phpunit timeout issues)
- Swift test verification

### Implementation Plan

```text
Step 1: Update sdk_coverage_matrix.md
├── Change Lineage section to reflect 8/10 full support
├── Add Rust SDK lineage methods
└── Correct coverage percentages

Step 2: Run Ruby tests
├── bundle exec rake test
└── Capture pass/fail count

Step 3: Create test summary table
├── All 10 SDKs in one view
└── Pass/fail/skip counts
```

### Success Metrics

| Metric          | Target                 | Verification                  |
| --------------- | ---------------------- | ----------------------------- |
| Matrix updated  | Correct lineage status | Review sdk_coverage_matrix.md |
| Ruby tests run  | Pass/fail count known  | Terminal output               |
| Summary created | All 10 SDKs documented | act.md contains table         |

### Risk Mitigation

| Risk                 | Mitigation                               |
| -------------------- | ---------------------------------------- |
| Ruby tests fail      | Document failures, create follow-up task |
| Matrix becomes stale | Add timestamp, link to code evidence     |

### No-Go Criteria

- Do NOT implement lineage for SDKs that already have it
- Do NOT modify SDK source code this iteration (verification only)
- Do NOT attempt PHP tests (known timeout issue)
