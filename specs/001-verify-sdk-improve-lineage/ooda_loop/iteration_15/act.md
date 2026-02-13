# OODA-15 Act: Rust SDK Lineage Tests

## Actions Taken
1. Added 18 lineage/metadata tests to integration_tests.rs
2. Fixed issues:
   - Renamed duplicate `test_provenance_for_entity` → `test_provenance_for_entity_with_confidence`
   - Fixed `client.lineage()` → `client.provenance().lineage()` 
   - Fixed mock paths from `/api/v1/graph/entities/` → `/api/v1/entities/`
3. All 70 tests pass (was 52, +18)

## Commit
- Hash: `feae789a`
- Message: `feat(rust-sdk): add 18 lineage/metadata tests (70 total)`
