# OODA Iteration 10 – DECIDE

**Objective:** Implementation plan for PostgreSQL deletion tests

---

## Decision

Use the **simpler approach**: Create test-local PostgreSQL state factory in the test file itself. This avoids modifying production code while still testing the same deletion logic with PostgreSQL storage.

---

## Implementation Plan

### Step 1: Create `e2e_document_deletion_postgres.rs`

```
edgequake/crates/edgequake-api/tests/e2e_document_deletion_postgres.rs
```

Structure:

1. Feature guard: `#![cfg(feature = "postgres")]`
2. Environment helpers: `get_database_url()`, `create_postgres_state()`
3. `require_postgres!` macro
4. 5-6 critical deletion tests

### Step 2: Implement `create_postgres_state()`

Factory function that:

1. Creates PostgreSQL config with unique namespace
2. Initializes KV, Vector, Graph storages
3. Creates mock LLM provider
4. Assembles full `AppState`
5. Returns state for test use

### Step 3: Implement Tests

| #   | Test Name                                    | Original Line  |
| --- | -------------------------------------------- | -------------- |
| 1   | `test_single_document_deletion_pg`           | From line 152  |
| 2   | `test_delete_preserves_shared_entities_pg`   | From line 437  |
| 3   | `test_source_ids_accumulates_pg`             | From line 761  |
| 4   | `test_delete_with_accumulated_source_ids_pg` | From line 890  |
| 5   | `test_query_after_deletion_pg`               | From line 1610 |

### Step 4: Add Cleanup Helper

```rust
async fn cleanup_test_namespace(pool: &PgPool, namespace: &str) {
    // Delete from kv_store WHERE namespace = ?
    // Delete from chunks WHERE namespace = ?
    // Delete graph data
}
```

### Step 5: Verify

Run: `cargo test --package edgequake-api --test e2e_document_deletion_postgres --features postgres`

---

## Files to Create/Modify

| File                                      | Action |
| ----------------------------------------- | ------ |
| `tests/e2e_document_deletion_postgres.rs` | CREATE |

---

## Success Criteria

1. All 5 PostgreSQL tests pass
2. Tests clean up after themselves
3. Tests skip gracefully if DATABASE_URL not set
4. No changes to production code

---

## Estimated Complexity

- Lines of code: ~500 (mostly copied from memory tests)
- Time: 30-45 minutes
- Risk: LOW (isolated test file)
