# OODA Iteration 10 – ORIENT

**Objective:** Design PostgreSQL test infrastructure for deletion tests

---

## Solution Architecture

### 1. Test State Factory

Add `test_state_postgres()` to `AppState` that:

- Reads DATABASE_URL from environment
- Creates PostgreSQL-backed storage with unique namespace
- Uses Mock LLM provider (same as memory tests)
- Returns cleanup function for test isolation

### 2. PostgreSQL Test File

Create `e2e_document_deletion_postgres.rs`:

- Feature-gated with `#[cfg(feature = "postgres")]`
- Uses `require_postgres!` macro pattern
- Runs subset of critical deletion tests
- Cleans up after each test

### 3. Test Selection Rationale

| Test                   | Why Critical                                      |
| ---------------------- | ------------------------------------------------- |
| Single deletion        | Verifies cascade delete works with PG constraints |
| Shared entities        | Tests source_ids merge with real transactions     |
| Accumulated source_ids | Tests entity update with real UPSERT              |
| Partial cleanup        | Tests selective deletion with FK constraints      |
| Query after deletion   | Verifies query engine works with PG storage       |

### 4. Implementation Approach

```rust
// In state.rs (new function)
#[cfg(feature = "postgres")]
pub async fn test_state_postgres(database_url: &str) -> Result<Self, Box<dyn Error>> {
    // Parse database URL
    // Create unique namespace for test isolation
    // Create PG storage instances
    // Use MockProvider for LLM
    // Return configured state
}
```

```rust
// In e2e_document_deletion_postgres.rs
#[cfg(feature = "postgres")]
#[tokio::test]
async fn test_single_document_deletion_postgres() {
    let pool = require_postgres!();
    let state = AppState::test_state_postgres(&get_database_url().unwrap()).await.unwrap();
    let server = Server::new(create_test_config(), state);
    // ... same test logic as memory version ...
}
```

### 5. Alternative: Simpler Approach

Instead of adding `test_state_postgres()` to the main crate, create a test-local helper:

```rust
async fn create_postgres_test_state() -> Option<AppState> {
    let database_url = get_database_url()?;
    // Direct construction using PostgreSQL storages
}
```

This approach:

- Avoids touching production code
- Keeps test infrastructure isolated
- Still verifies the same deletion logic

---

## Dependencies

- `postgres` feature flag
- `sqlx` for direct DB access (test cleanup)
- Running PostgreSQL with AGE extension

---

## Risks

1. **Test isolation**: Must create unique namespace per test
2. **Cleanup failures**: May leave orphaned data
3. **Extension availability**: AGE extension required for graph storage

---

## Next Step

Create decide.md with implementation plan.
