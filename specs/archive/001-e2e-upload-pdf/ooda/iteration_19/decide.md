# OODA-19: Decide — Create Shared Module, Refactor One File

## Decision

1. Create `tests/common/mod.rs` with all shared test helper functions
2. Refactor `e2e_query_engine.rs` to use `mod common; use common::*;`
3. Remove 112 lines of duplicated helpers from that file
4. Verify all 11 tests still pass
5. Other files keep their local helpers for now (incremental migration)

## Rationale
- Zero-risk approach: creating new module doesn't break existing tests
- One file refactored as proof of concept demonstrates the pattern
- Future OODA iterations can migrate remaining files incrementally
