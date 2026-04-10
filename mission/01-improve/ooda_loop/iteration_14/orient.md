# OODA-14: Orient

## DRY Analysis

The `ResultExt::internal_err()` helper (OODA-09) and `parse_uuid()` (OODA-09) eliminate the repeated pattern:
```rust
.map_err(|e| ApiError::Internal(format!("Failed to X: {}", e)))
```
→ becomes:
```rust
.internal_err("X")
```

**Risk**: Changing error message format from "Foo error: ..." to "Failed to foo: ..." could affect log grep patterns. Acceptable since these are internal errors not exposed to API consumers.

**Benefit**: 49 sites × ~50 chars saved = ~2450 chars of boilerplate removed. More importantly, consistent error formatting across all handlers.

## Decision Criteria
- All sites use the same `internal_err` pattern → mechanical, safe
- UUID parse sites use `parse_uuid` → same validation + error type
- Unused imports from prior iterations need cleanup
