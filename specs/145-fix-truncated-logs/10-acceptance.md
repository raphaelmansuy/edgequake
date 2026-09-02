# 10 — Acceptance

| ID | Criterion | Proof |
|----|-----------|-------|
| A1 | Spec pack complete under `specs/145-fix-truncated-logs/` with no personal data | Doc review / LAW-145-10 |
| A2 | Generation + query-root I/O no longer capped at 512 | U-145-01 |
| A3 | Dual-write keys equal and complete | U-145-08 |
| A4 | Multibyte safe | U-145-02 |
| A5 | Secrets redacted | U-145-05 |
| A6 | Honest ceiling | U-145-06 |
| A7 | Structured / Preview unchanged intent | U-145-03 / U-145-04 |
| A8 | Stream path records I/O | C-145-01 |
| A9 | `make spec145-proof` green in CI | Makefile |
| A10 | Live Langfuse GET shows tail marker (when stack up) | E-145-01 |
| A11 | Operator docs updated | `docs/OBSERVABILITY.md` |
| A12 | Non-blocking export preserved | LAW-124-4 unchanged |

## Done when

All A1–A12 checked; status board in README flipped to Done for I*/T*/A1.

## Cross-refs

- E2E: [08-e2e-test-matrix.md](08-e2e-test-matrix.md)
- Honest: [11-honest-assessment.md](11-honest-assessment.md)
