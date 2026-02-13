# OODA Iteration 02 — Observe: Complete Test Baseline for All 10 SDKs

**Date**: 2026-02-13

## Complete Test Baseline

| SDK        | Total Tests | Passed | Failed | Errors | Skipped | Status |
|------------|-------------|--------|--------|--------|---------|--------|
| Python     | 467         | 435    | 0      | 0      | 32      | ✅     |
| TypeScript | 312         | 247    | 0      | 0      | 65      | ✅     |
| Rust       | 55          | 55     | 0      | 0      | 0       | ✅     |
| C#         | 71          | 71     | 0      | 0      | 0       | ✅     |
| Go         | 186         | 186    | 0      | 0      | 0       | ✅     |
| Java       | —           | —      | —      | BUILD  | —       | ❌     |
| Kotlin     | —           | —      | —      | BUILD  | —       | ❌     |
| PHP        | 62          | 7      | 0      | 55     | 0       | ❌     |
| Ruby       | 59          | 59     | 0      | 0      | 0       | ✅     |
| Swift      | 70          | 70     | 0      | 0      | 0       | ✅     |

**Totals**: 1282 tests across 8 buildable SDKs, 1130 passing

## Issues Found

1. **Java**: Build fails — `invalid target release: 17` (system has JDK 8)
2. **Kotlin**: Build fails — Jackson readValue ambiguity (likely JDK version issue)
3. **PHP**: 55 out of 62 tests error with `Class "EdgeQuake\HealthService" not found` — services not properly autoloaded

## Endpoint Coverage Counts (from grep of API paths)

Need to count actual `/api/v1/` paths referenced in each SDK.
