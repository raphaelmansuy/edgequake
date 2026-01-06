# Task Log: Test Fixtures for Fresh Workspace

**Date:** 2026-01-06-19-44

## Actions

- Created `tests/fixtures/` directory with 16 embedded test documents
- Created `test_fixtures.rs` module (565 lines) with:
  - `test_data` module embedding all documents via `include_str!()`
  - API client functions: `check_health()`, `list_documents()`, `delete_document()`, `ingest_document()`
  - `setup_fresh_workspace()` with `SetupOptions` for configuration
  - `clear_all_documents()` for fresh refresh
- Updated `api_integration_tests.rs` to use test_fixtures module
- Added `test_00_setup_fresh_workspace` test (runs first alphabetically)
- Total: 223 passing tests + 7 ignored API tests

## Decisions

- Embedded documents via `include_str!()` for portability (no filesystem dependency)
- Used `core_only` option (5 docs) for faster CI runs
- Named setup test `test_00_*` to run first alphabetically
- API tests remain `#[ignore]` to avoid CI failures without server

## Next Steps

- Run `make dev` to start the server
- Run `cargo test --package edgequake-query --test api_integration_tests -- --ignored --nocapture`
- Consider adding CI workflow to run API tests against test server

## Lessons/Insights

- Test isolation requires fresh data ingestion before API tests
- Embedding 16 markdown files adds ~4KB to test binary (acceptable)
- API task polling may not be needed if server processes synchronously
