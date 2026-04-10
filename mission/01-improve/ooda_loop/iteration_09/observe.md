# Observe — Iteration 09
Date: 2026-04-10. Mission re-read: `mission/01-improve.md`

## Findings
57 instances of `map_err(|e| ApiError::Internal(format!("Failed to {}: {}", context, e)))` across the API crate. This is a DRY violation — the same error wrapping pattern repeated everywhere.

Additionally, 12 instances of `uuid::Uuid::parse_str().map_err(|_| ApiError::ValidationError("Invalid ... ID"))`.

## Impact
- Every new handler copy-pastes the same pattern
- Error message format inconsistency (some use `{}`, some use `{:?}`)
- No helper trait or extension method exists
