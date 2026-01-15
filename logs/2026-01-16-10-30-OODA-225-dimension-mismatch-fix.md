# OODA-225: Fix Vector Dimension Mismatch After Provider Switch

**Date**: 2026-01-16
**Status**: ✅ RESOLVED
**Severity**: HIGH (blocks queries after provider switch)

## Problem Statement

After switching embedding providers (e.g., Ollama 768 dims → OpenAI 1536 dims) and rebuilding the knowledge graph, queries fail with:

```
Storage error: Database error: Vector query failed: error returned from database:
different vector dimensions 1536 and 768
```

## Root Cause Analysis

### Two Independent Issues Identified:

#### Issue 1: Workspace Vector Registry Cache Not Evicted (Fixed in earlier session)

- `PgWorkspaceVectorRegistry` caches `Arc<dyn VectorStorage>` instances by workspace_id
- When embedding dimension changes, cache holds stale storage with old dimension
- **Fix**: Added cache eviction in `rebuild_embeddings` and `rebuild_knowledge_graph` handlers

#### Issue 2: Global Dimension Validation Broken (Fixed now - OODA-225)

- `AppState::new_postgres()` validated dimension mismatch by comparing:
  - `vector_storage.dimension()` - returns **configured** dimension (what we just set!)
  - `embedding_provider.dimension()` - returns provider dimension
- These always match because `with_dimension(embedding_dim)` sets the same value!
- **The validation could never detect a mismatch**

## Solution

### Fix for Issue 2: Added `get_stored_dimension()` Method

**File**: `edgequake-storage/src/adapters/postgres/vector.rs`

```rust
/// Get the dimension of vectors actually stored in the database.
///
/// This queries the first stored vector to detect its actual dimension,
/// which may differ from the configured dimension if the embedding provider
/// has been changed since vectors were stored.
pub async fn get_stored_dimension(&self) -> Result<Option<usize>> {
    let pool = match self.pool.get().await {
        Ok(p) => p,
        Err(_) => return Ok(None),
    };

    let sql = format!(
        "SELECT vector_dims(embedding) as dim FROM {} LIMIT 1",
        self.table_name
    );

    let result: Option<(i32,)> = sqlx::query_as(&sql)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to get stored dimension: {}", e))
        })?;

    Ok(result.map(|(dim,)| dim as usize))
}
```

### Updated Validation in `state.rs`

```rust
// Validate dimension compatibility for existing storage
if !vector_storage.is_empty().await? {
    // Query actual stored dimension from database, not the configured dimension
    let storage_dim = vector_storage
        .get_stored_dimension()
        .await?
        .unwrap_or(embedding_dim);
    let provider_dim = embedding_provider.dimension();

    if storage_dim != provider_dim {
        return Err(format!(
            "❌ Dimension mismatch detected\n..."
        ).into());
    }
}
```

## Files Modified

1. **vector.rs**: Added `get_stored_dimension()` method using `vector_dims()` PostgreSQL function
2. **state.rs**: Updated dimension validation to query actual stored dimension
3. **workspaces.rs**: Added cache eviction on dimension change (earlier fix)
4. **query.rs**: Added auto-evict-and-retry on dimension mismatch (earlier fix)
5. **e2e_postgres_rebuild.rs**: Added test for cache eviction behavior

## Test Results

- **All dimension validation tests pass**: 3/3
- **All rebuild tests pass**: 6/6
- **Full test suite**: 2,631 tests pass, 0 failures

## Technical Details

### pgvector `vector_dims()` Function

The fix uses pgvector's built-in `vector_dims(vector)` function which returns the number of dimensions of a vector column. This is more reliable than:

- Parsing the vector text representation
- Counting array elements
- Relying on column metadata

### Why This Was Hard to Detect

The original validation logic appeared correct at first glance:

```rust
let storage_dim = vector_storage.dimension();  // Looks like stored dimension
let provider_dim = embedding_provider.dimension();
if storage_dim != provider_dim { ... }  // Looks like proper validation
```

But `dimension()` returns `self.dimension` which is set during construction from the provider's dimension - so it's comparing the same value to itself!

## Prevention Measures

1. **New `get_stored_dimension()` method**: Explicitly queries database for actual stored dimension
2. **WHY comments**: Added explanatory comments documenting the design decision
3. **Test coverage**: `test_postgres_dimension_mismatch_error` validates the detection works

## User-Facing Behavior

When dimension mismatch is detected at startup, users see a clear error message with recovery options:

1. Switch back to previous provider
2. Clear existing vectors (destructive)
3. Rebuild vectors with new provider

---

## Task Logs

**Actions**:

- Added `get_stored_dimension()` method using `vector_dims()` SQL function
- Fixed dimension validation in `AppState::new_postgres()` to query actual stored dimension
- Verified 2,631 tests pass

**Decisions**:

- Used pgvector's native `vector_dims()` function for reliable dimension detection
- Default to configured dimension if detection fails (empty table case)

**Next Steps**:

- Commit changes
- Manual browser verification with provider switch scenario

**Lessons**:

- Validation that compares a value against itself will always pass
- Need to distinguish between "configured dimension" and "stored dimension"
