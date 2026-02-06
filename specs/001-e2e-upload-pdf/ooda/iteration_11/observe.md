# OODA-11 Observe: Test Timeout Enforcement

## Mission Re-read
Re-read `specs/001-e2e-upload-pdf.md` — Requirement #4: "Focused Tests: All tests must have timeouts (30s for unit, 120s for E2E)"

## Current State

### Test Suite Survey
- **50 test files** with `#[tokio::test]` annotations
- **~591 total tests** across all files
- **0 tests** have explicit timeout enforcement

### Top Files by Test Count
| File | Test Count |
|------|-----------|
| e2e_document_deletion.rs | 73 |
| e2e_api_comprehensive.rs | 46 |
| integration_tests.rs | 30 |
| e2e_query.rs | 26 |
| e2e_auth.rs | 25 |
| e2e_entities.rs | 24 |
| e2e_costs.rs | 22 |
| e2e_pipeline_comprehensive.rs | 17 |

### Risk Assessment
- Mock pipeline completes in ~0.02s → low risk of timeout in unit tests
- Real LLM tests could hang indefinitely without timeouts
- CI/CD pipelines need bounded execution time

## Evidence
- `cargo test --lib` → 444 tests pass in 9.90s (no hangs)
- `e2e_pipeline_comprehensive` → 17 tests, no timeouts, all fast with mock
- `e2e_clean_tenant.rs` → 2 tests already have 30s timeouts (OODA-10)
