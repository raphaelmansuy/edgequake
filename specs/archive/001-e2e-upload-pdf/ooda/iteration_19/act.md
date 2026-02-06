# OODA-19: Act — Shared Module Implementation

## Implementation

### Files Created
- `edgequake/crates/edgequake-api/tests/common/mod.rs` — 237 lines, 10 public functions

### Files Modified
- `edgequake/crates/edgequake-api/tests/e2e_query_engine.rs` — -112 lines helpers, +3 lines imports

### Test Results
```
running 11 tests — all pass in 0.02s
```

### Commit
- SHA: `c3dff93b`
- Message: "OODA-19: Extract shared test helpers into common module, refactor e2e_query_engine"

### Public API of common/mod.rs
| Function | Purpose |
|---|---|
| `create_test_app()` | Build router with test_state() |
| `with_timeout()` | Wrap future with timeout |
| `extract_json()` | Parse response body |
| `post_json()` | POST without auth headers |
| `post_json_with_tenant()` | POST with X-Tenant-ID/X-User-ID/X-Workspace-ID |
| `get_endpoint()` | GET without auth headers |
| `get_with_tenant()` | GET with auth headers |
| `delete_endpoint()` | DELETE without auth headers |
| `upload_document()` | Convenience POST to /api/v1/documents |
| `upload_document_assert()` | Upload + assert 201 |
