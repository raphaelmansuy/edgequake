# OODA-19: Observe — Cleanup & Shared Test Helpers

## Observation Date
2025-07-13

## What Was Examined

### Test Helper Duplication
All 8 OODA test files (10-18) duplicate these helper functions:
- `with_timeout()` — 8 copies (identical)
- `create_test_app()` — 8 copies (identical)
- `extract_json()` — 8 copies (identical)
- `post_json()` — 8 copies (identical)
- Various `*_with_tenant()` helpers — 3+ copies

### Total Duplicated Lines
Approximately 80 lines of helper code duplicated per file × 8 files = ~640 lines of duplication.

### Rust Test Module Convention
Tests in `tests/` directory are compiled independently. The standard approach for shared
code is `tests/common/mod.rs` which test files import via `mod common;`.
