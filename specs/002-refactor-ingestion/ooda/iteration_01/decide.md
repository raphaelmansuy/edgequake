# OODA Iteration 01 - Decide

**Mission Re-read**: ✅ `specs/002-refactor-ingestion.md`
**Focus**: Critical Issue #1 - Race Condition in Re-ingestion
**Date**: 2026-02-08

---

## Decision

### Primary Change: Add Atomic Status Transition to KV Storage

**Priority**: 🔴 CRITICAL - Must fix before other changes

---

## Action Plan

### Step 1: Add `transition_if_status` Method to KVStorage Trait

**File**: `edgequake/crates/edgequake-storage/src/traits/kv.rs`
**Estimated Lines**: +30

```rust
/// Atomically transition document status if current status matches expected.
///
/// @implements FIX-RACE-01: Prevent TOCTOU race conditions
///
/// # Arguments
/// - `key`: Document metadata key
/// - `expected_status`: Status value that must match for transition
/// - `new_status`: Status value to set if match
///
/// # Returns
/// - `Ok(true)`: Transition succeeded (status matched and was updated)
/// - `Ok(false)`: Transition failed (status did not match expected)
/// - `Err(...)`: Database error
async fn transition_if_status(
    &self,
    key: &str,
    expected_status: &str,
    new_status: &str,
) -> Result<bool>;
```

### Step 2: Implement for PostgreSQL KV Storage

**File**: `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs`
**Estimated Lines**: +20

```sql
-- Atomic update with JSON path check
UPDATE eq_kv
SET value = jsonb_set(value, '{status}', $new_status::jsonb),
    updated_at = NOW()
WHERE key = $key
  AND value->>'status' = $expected_status;
```

### Step 3: Implement for Memory KV Storage

**File**: `edgequake/crates/edgequake-storage/src/adapters/memory/kv.rs`
**Estimated Lines**: +20

```rust
// Use RwLock to ensure atomic check-and-set
async fn transition_if_status(&self, key: &str, expected: &str, new: &str) -> Result<bool> {
    let mut guard = self.data.write().await;
    if let Some(value) = guard.get_mut(key) {
        if value.get("status").and_then(|v| v.as_str()) == Some(expected) {
            value["status"] = serde_json::json!(new);
            return Ok(true);
        }
    }
    Ok(false)
}
```

### Step 4: Refactor `delete_document_for_reingestion`

**File**: `edgequake/crates/edgequake-api/src/handlers/documents.rs`
**Lines**: 489-555

**Before** (race condition):

```rust
let status = get_status().await;
if status == "pending" || status == "processing" {
    return Ok(false);
}
// ⚠️ RACE WINDOW
delete_data().await;
```

**After** (atomic):

```rust
// Try to claim the document for deletion
let transitioned = state.kv_storage
    .transition_if_status(&metadata_key, "failed", "deleting")
    .await?;

if !transitioned {
    // Also try completed status (user wants fresh extraction)
    let transitioned = state.kv_storage
        .transition_if_status(&metadata_key, "completed", "deleting")
        .await?;

    if !transitioned {
        return Err(ApiError::Conflict(
            "Document is currently processing or status changed. Try again later.".to_string()
        ));
    }
}

// Safe to delete - we hold exclusive "deleting" status
delete_data().await;
```

### Step 5: Add ApiError::Conflict Variant

**File**: `edgequake/crates/edgequake-api/src/error.rs`
**Estimated Lines**: +15

```rust
#[derive(Debug, Serialize)]
pub enum ApiError {
    // ... existing variants

    /// HTTP 409 Conflict - Resource state changed
    #[error("Conflict: {0}")]
    Conflict(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Conflict(msg) => {
                (StatusCode::CONFLICT, Json(json!({"error": msg}))).into_response()
            }
            // ... other matches
        }
    }
}
```

### Step 6: Add Unit Tests

**File**: `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs` (test module)
**Estimated Lines**: +50

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_transition_if_status_success() {
        // Setup: document with status "failed"
        // Action: transition_if_status("failed", "deleting")
        // Assert: returns true, status is now "deleting"
    }

    #[tokio::test]
    async fn test_transition_if_status_wrong_status() {
        // Setup: document with status "processing"
        // Action: transition_if_status("failed", "deleting")
        // Assert: returns false, status still "processing"
    }

    #[tokio::test]
    async fn test_concurrent_transitions_only_one_succeeds() {
        // Setup: document with status "failed"
        // Action: parallel calls to transition_if_status
        // Assert: exactly one succeeds
    }
}
```

---

## Change Summary

| File                      | Change Type    | Lines Added | Lines Removed |
| ------------------------- | -------------- | ----------- | ------------- |
| `traits/kv.rs`            | New method     | +30         | 0             |
| `adapters/postgres/kv.rs` | Implementation | +20         | 0             |
| `adapters/memory/kv.rs`   | Implementation | +20         | 0             |
| `handlers/documents.rs`   | Refactor       | +25         | -15           |
| `error.rs`                | New variant    | +15         | 0             |
| Tests                     | New tests      | +50         | 0             |
| **Total**                 |                | **+160**    | **-15**       |

---

## Rollback Plan

If issues discovered:

1. Remove `transition_if_status` method calls
2. Revert to non-atomic check (accept temporary race risk)
3. Investigate issue, fix in next iteration

---

## Success Criteria

- [ ] `cargo test -p edgequake-storage` passes
- [ ] `cargo test -p edgequake-api` passes
- [ ] Concurrent re-ingestion test passes
- [ ] No breaking changes to existing API contracts

---

## Commit Message Template

```
OODA-01: Fix TOCTOU race condition in document re-ingestion

- Add transition_if_status() atomic method to KVStorage trait
- Implement for PostgreSQL and Memory adapters
- Refactor delete_document_for_reingestion to use atomic transition
- Add ApiError::Conflict (HTTP 409) for concurrent access
- Add concurrency tests for transition operations

Fixes #RACE-01: Document state can corrupt during concurrent operations

@implements FIX-RACE-01
```

---

**Ready for Act phase**: YES
