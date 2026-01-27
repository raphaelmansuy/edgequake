# OODA-223: Workspace Isolation Safety Implementation

## Executive Summary

This document describes the safety measures implemented to prevent data from being stored in the wrong vector storage table. The issue was discovered when queries returned "0 Sources" despite data existing, because data was ingested into the global table but queries looked in workspace-specific tables.

## Problem Statement

### Root Cause

Before per-workspace vector tables were implemented, data was stored in a global `eq_eq_default_vectors` table. After the per-workspace implementation, queries started looking in `eq_eq_default_ws_{workspace_id}_vectors` tables, which were empty.

### Symptoms

- Query returns "0 Sources · N Topics"
- Workspace stats show: Documents: 0, Entities: 0, Relationships: 0, Chunks: 0
- Data exists in the database but in the wrong table

### Impact

- Complete loss of query functionality for affected workspaces
- Silent data isolation violation
- No error messages to alert operators

## Solution: Strict Workspace Mode

### Design Principles

1. **Fail Loudly in Production**: When workspace storage cannot be obtained, fail with a clear error instead of silently falling back to default storage.

2. **Storage Mode Detection**: Use `storage_mode` to determine strictness:

   - **PostgreSQL** (production): Strict mode enabled - errors on workspace failures
   - **Memory** (tests/development): Non-strict mode - allows fallback with warnings

3. **Backward Compatibility**: Tests using in-memory storage continue to work without modification.

### Implementation Details

#### `handlers/documents.rs`

Two new functions replace the unsafe `get_workspace_vector_storage`:

```rust
/// STRICT: For ingestion - fails if workspace not found
async fn get_workspace_vector_storage_strict(
    state: &AppState,
    workspace_id: &str,
) -> Result<Arc<dyn VectorStorage>, ApiError>

/// LEGACY: For reads - falls back with warning (deprecated for writes)
async fn get_workspace_vector_storage_with_fallback(
    state: &AppState,
    workspace_id: &str,
) -> Arc<dyn VectorStorage>
```

The strict function:

- Checks `state.storage_mode.is_memory()` to allow fallback in tests
- Returns `ApiError` in production on any failure
- Logs CRITICAL errors with full context
- Documents the OODA-223 lesson learned

#### `processor.rs`

Added `strict_workspace_mode: bool` field to `DocumentTaskProcessor`:

```rust
pub struct DocumentTaskProcessor {
    // ... other fields ...
    /// OODA-223: Strict workspace mode - when true, fail if workspace not found.
    strict_workspace_mode: bool,
}
```

Three constructors with different strictness levels:

```rust
// Legacy - non-strict (for backward compatibility)
DocumentTaskProcessor::new(...)

// With workspace support - non-strict (for tests)
DocumentTaskProcessor::with_workspace_support(...)

// Production - strict (OODA-223 recommended)
DocumentTaskProcessor::with_workspace_support_strict(...)
```

#### `main.rs`

Production server uses strict mode based on storage:

```rust
let processor = if state.storage_mode.is_postgresql() {
    info!("🔒 Using STRICT workspace isolation mode (PostgreSQL storage)");
    Arc::new(DocumentTaskProcessor::with_workspace_support_strict(...))
} else {
    info!("⚠️ Using non-strict workspace mode (in-memory storage)");
    Arc::new(DocumentTaskProcessor::with_workspace_support(...))
};
```

## Error Messages

When strict mode is enabled and workspace storage fails:

### Invalid Workspace ID

```
CRITICAL: Invalid workspace ID during ingestion - refusing to use default storage
Error: BadRequest("Invalid workspace ID 'xxx': invalid UUID format...")
```

### Workspace Not Found

```
CRITICAL: Workspace not found during ingestion - refusing to use default storage
Error: NotFound("Workspace 'xxx' not found. Cannot ingest documents without a valid workspace.")
```

### Failed to Create Storage

```
CRITICAL: Failed to create workspace vector storage - refusing to use default
Error: Internal("Failed to create vector storage for workspace 'xxx' (dimension 1536): ...")
```

## Logging

### Production (Strict Mode)

- `error!` level for failures
- Includes `workspace_id`, `error`, `dimension` in structured logs
- Clear "CRITICAL INGESTION ERROR" prefix

### Development (Non-Strict Mode)

- `warn!` level for fallbacks
- Includes `storage_mode = ?` to indicate test mode
- Message indicates "(non-strict mode)" or "(allowed in memory/test mode)"

## Migration Notes

### For Existing Data

If you have data in the global table that should be in workspace tables:

```sql
-- Check for orphaned data in global table
SELECT workspace_id, COUNT(*)
FROM eq_eq_default_vectors
GROUP BY workspace_id;

-- Migrate to workspace-specific table
INSERT INTO eq_eq_default_ws_{workspace_uuid}_vectors
SELECT * FROM eq_eq_default_vectors
WHERE metadata->>'workspace_id' = '{workspace_uuid}';
```

### For Tests

Tests using `AppState::test_state()` will continue to work because:

1. Memory storage mode is detected
2. Non-strict mode allows fallback
3. Warnings are logged but don't fail

### For Production

New deployments automatically use strict mode when PostgreSQL is detected. Existing deployments should:

1. Ensure all workspaces exist before ingesting documents
2. Monitor logs for "CRITICAL INGESTION ERROR" messages
3. Handle ingestion failures gracefully in UI

## Files Modified

| File                    | Changes                                                           |
| ----------------------- | ----------------------------------------------------------------- |
| `handlers/documents.rs` | Added `get_workspace_vector_storage_strict()`, updated call sites |
| `processor.rs`          | Added `strict_workspace_mode` field, updated constructors         |
| `main.rs`               | Production uses strict mode for PostgreSQL storage                |

## Testing

All existing tests pass (46 API tests). The safety implementation:

- Doesn't require test modifications
- Maintains backward compatibility
- Logs warnings in test mode instead of failing

## Verification

To verify strict mode is working in production:

1. Check server startup logs for:

   ```
   🔒 Using STRICT workspace isolation mode (PostgreSQL storage)
   ```

2. Try ingesting without a valid workspace:

   - Should fail with clear error message
   - Should NOT silently succeed

3. Monitor logs for:
   - `CRITICAL INGESTION ERROR` (indicates failed ingestion - expected)
   - `using default storage` without `(non-strict mode)` (indicates bug - report it)

## Related Documents

- [OODA-223 Investigation Log](../logs/2025-01-14-OODA-223-0-sources-investigation.md)
- [Workspace Vector Registry](./workspace-vector-registry.md)
