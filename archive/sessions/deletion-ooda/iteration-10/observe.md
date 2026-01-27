# OODA Iteration 10 – OBSERVE

**Objective:** Verify document deletion works correctly with PostgreSQL provider (in addition to memory provider)

---

## Observation

### Current Test Infrastructure

1. **Memory Tests**: All 21 deletion tests in `e2e_document_deletion.rs` use `AppState::test_state()` which creates in-memory storage.

2. **Existing PostgreSQL Tests**:
   - `e2e_postgres_workspace.rs` - Tests workspace service with PostgreSQL
   - `edgequake-storage/tests/postgres_integration.rs` - Tests storage layer directly
   - Both use `#[cfg(feature = "postgres")]` and `require_postgres!` macro

3. **PostgreSQL Configuration**:
   - `DATABASE_URL` or `POSTGRES_PASSWORD` environment variables
   - Feature flag: `--features postgres`
   - Current container: `edgequake-postgres` running and healthy

### Gap Analysis

**GAP-10: No E2E deletion tests run with PostgreSQL**

Current deletion tests only verify memory storage. PostgreSQL has different:

- Transaction semantics (ACID)
- Cascading delete behavior
- Constraint enforcement
- Connection pooling issues

This gap violates the mission requirement: "Ensure it working with postgres provider and memory provider for all storage layers"

### Options

1. **Duplicate Tests**: Copy all 21 tests to a new file with PostgreSQL state
   - Pro: Clear separation
   - Con: 2000+ lines of duplicate code

2. **Parameterized Tests**: Make tests accept state as parameter
   - Pro: DRY, single source of truth
   - Con: Significant refactor

3. **Feature-Gated PostgreSQL Suite**: Add new test file with key deletion tests for PostgreSQL
   - Pro: Focused verification of PostgreSQL-specific behaviors
   - Con: Some test duplication

4. **Test State Factory**: Add `test_state_postgres()` async function
   - Pro: Easy to use, minimal code changes
   - Con: Requires database to be running

### Recommendation

Option 4: Create `test_state_postgres()` and add a PostgreSQL-specific test file `e2e_document_deletion_postgres.rs` with the most critical deletion tests (5-7 tests instead of all 21).

Critical tests to port:

1. `test_single_document_deletion` - Basic cascade delete
2. `test_delete_preserves_shared_entities` - source_ids tracking
3. `test_source_ids_accumulates_across_documents` - Multi-document entities
4. `test_delete_with_accumulated_source_ids` - Partial cleanup
5. `test_query_after_deletion_does_not_error` - Query safety

---

## Files to Examine

| File                        | Purpose                                           |
| --------------------------- | ------------------------------------------------- |
| `state.rs:625-720`          | `new_postgres()` - real PostgreSQL state creation |
| `e2e_postgres_workspace.rs` | Pattern for PostgreSQL tests                      |
| `postgres_integration.rs`   | Storage-level PostgreSQL tests                    |

---

## Next Step

Create orient.md with solution design for PostgreSQL test state factory.
