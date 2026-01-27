# OODA-296: Flaky Test Detection Mechanism

## Date: 2026-01-16

## Status: ✅ COMPLETE

## Objective

Create a flaky test detection system to identify non-deterministic test failures.

## Created Script

**File:** `scripts/detect_flaky_tests.sh`

## How It Works

1. **Multiple Iterations**: Runs tests N times (default: 3)
2. **Failure Tracking**: Records which tests fail each iteration
3. **Flaky Detection**: Tests that fail inconsistently are flagged
4. **Consistent Failures**: Tests that always fail are categorized separately

## Usage

```bash
# Run with default 3 iterations on all packages
./scripts/detect_flaky_tests.sh

# Run 5 iterations on specific package
./scripts/detect_flaky_tests.sh 5 edgequake-core

# Quick 2-iteration check
./scripts/detect_flaky_tests.sh 2 edgequake-llm
```

## Output

```
Results saved to test-results/flaky-detection/
├── iteration_1.txt      # Full test output
├── iteration_2.txt
├── iteration_3.txt
├── failed_1.txt         # Failed tests per iteration
├── failed_2.txt
├── failed_3.txt
├── all_failed.txt       # Unique failed tests
├── flaky_candidates.txt # Tests that fail inconsistently
├── consistent_failures.txt
└── report.json          # Machine-readable summary
```

## Classification

| Type       | Definition                           | Action                             |
| ---------- | ------------------------------------ | ---------------------------------- |
| Flaky      | Fails in some iterations but not all | Investigate timing/race conditions |
| Consistent | Fails in all iterations              | Bug to fix                         |
| Clean      | Never fails                          | Healthy test                       |

## Current Baseline

From OODA-294 Playwright analysis:

- 0 flaky tests detected in Rust suite
- 534/591 Playwright tests pass consistently
- 44 consistent failures (known issues)

## Integration with CI

Add to `.github/workflows/test-quality-gates.yml`:

```yaml
flaky-detection:
  name: Flaky Test Detection
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Run flaky detection
      run: ./scripts/detect_flaky_tests.sh 3 all
```

## Common Flaky Patterns to Watch

1. **Timing Dependencies**: Tests using real time
2. **Race Conditions**: Async tests without proper synchronization
3. **Resource Contention**: Shared state between tests
4. **Network Calls**: External service dependencies
5. **Random Data**: Non-seeded random generators

## Verification Run

```
Core Crate: 109 tests, 0.46s, 0 flaky
LLM Crate: 199 tests, 2.13s, 0 flaky
API Crate: 421 tests, 2.37s, 0 flaky
```

## Next Steps (OODA 297+)

1. Add weekly flaky test scan to CI
2. Create quarantine mechanism for flaky tests
3. Add automatic retry with logging
4. Build historical flaky test database

## Invariants Protected

- INV-011: No flaky tests in critical path
- All tests must pass deterministically
- Test results must be reproducible
