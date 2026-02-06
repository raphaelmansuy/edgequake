# OODA-19: Orient — DRY Analysis

## Analysis

### Problem
8 test files duplicate ~80 lines of identical helper code each, violating DRY.
This makes maintenance harder — any change to helper behavior requires updating all files.

### Solution: tests/common/mod.rs
Standard Rust pattern for shared test utilities. Contains:
- `create_test_app()` — app factory
- `with_timeout()` — timeout wrapper
- `extract_json()` — response body extraction
- `post_json()` — basic POST
- `post_json_with_tenant()` — POST with auth headers
- `get_endpoint()` — basic GET
- `get_with_tenant()` — GET with auth headers
- `delete_endpoint()` — basic DELETE
- `upload_document()` / `upload_document_assert()` — convenience wrappers

### Migration Strategy
1. Create `tests/common/mod.rs` with all shared helpers
2. Refactor `e2e_query_engine.rs` as proof of concept (latest, cleanest file)
3. Other files can be migrated incrementally (not blocking)
