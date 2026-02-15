# OODA-30 Decide: Create SDK Coverage Matrix Deliverable

## Date: 2026-02-14

## Decision

1. **Update** `sdk_coverage_matrix.md` with current test counts and coverage data
2. **Document** all 135 API endpoints mapped to 30 categories
3. **Identify** priority gaps for next iterations
4. **Commit** the matrix as a deliverable

## Priority for Next Iterations

| Priority | OODA | SDK         | Focus                                       |
| -------- | ---- | ----------- | ------------------------------------------- |
| P0       | 31   | Python      | Add Tenants, Workspaces, Settings resources |
| P0       | 32   | Python      | Add Models, Costs, Folders resources        |
| P1       | 33   | TypeScript  | Quality sweep — verify 95%+ target          |
| P1       | 34   | Java/Kotlin | Add missing endpoint tests                  |
| P2       | 35   | C#/Swift    | Add missing endpoint tests                  |
| P2       | 36   | PHP/Ruby/Go | Add Models, Settings services               |
| P3       | 37+  | All         | Conversation bulk ops, Document ops         |

## Rationale

Python is the most-used SDK (520 tests = most community usage). Closing its multi-tenancy gap has highest user impact. TypeScript is already at 96% and mostly needs verification. Secondary SDKs need incremental endpoint additions.
