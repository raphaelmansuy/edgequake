# P1 — QueryError semantic HTTP mapping (API-DRY-006)

**Status:** ✅ Proven  
**Date:** 2026-06-04

## Claim

`QueryError` maps to semantic HTTP status via `From<QueryError> for ApiError`; partial LLM override returns 400 not 500.

## Evidence

```bash
cargo test -p edgequake-api --lib test_query_error_status_code
cargo test -p edgequake-api --test e2e_query test_query_partial_llm_override_returns_bad_request
cargo test -p edgequake-api --lib validate_llm_override_pair_rejects_partial
```

| Mapping | HTTP |
|---------|------|
| `InvalidQuery` | 400 |
| `NoResults` | 404 |
| `ConfigError` | 422 |
| `LlmError` | 502 |
| Partial `llm_provider` without `llm_model` | 400 (`validate_llm_override_pair`) |

Fix applied 2026-06-04: `query_execute.rs` and `query_stream.rs` call `validate_llm_override_pair` before resolver.
