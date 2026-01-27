# OODA Iteration 14 - Orient

## Solution Analysis

### Option A: Extend HealthResponse with Schema Info

Add schema version info to existing `/health` endpoint.

**Pros**:

- Single endpoint for all health info
- No new routes needed
- Immediate visibility

**Cons**:

- Mixes concerns (runtime health vs schema state)
- PostgreSQL-only (memory mode has no schema)

### Option B: New `/api/v1/schema` Endpoint

Dedicated endpoint for schema verification.

**Pros**:

- Clean separation of concerns
- Can include detailed migration list
- Better for CI/CD integration

**Cons**:

- New route to maintain
- Separate call needed

### Option C: Startup Validation (Panic on Mismatch)

Check schema on server startup, panic if incompatible.

**Pros**:

- Fail-fast behavior
- Clear error messages
- Prevents running with wrong schema

**Cons**:

- No runtime visibility
- Harder to debug

### First Principles Decision

1. **Defense in Depth**: Both startup check AND runtime visibility
2. **Simplicity**: Start with health endpoint extension
3. **Testability**: Add to existing health tests

## Recommended Approach: Hybrid

1. **Phase 1** (this iteration): Add schema info to HealthResponse
   - Add `schema_version` field (latest migration number)
   - Add `migrations_applied` field (count of successful migrations)
   - PostgreSQL-only (memory mode returns None)

2. **Phase 2** (future): Add startup validation
   - Check expected vs applied migrations
   - Warning log if mismatch

## Schema Info Structure

```rust
#[derive(Serialize, ToSchema)]
pub struct SchemaHealth {
    /// Latest migration version applied (e.g., 15)
    pub latest_version: Option<i64>,
    /// Number of successful migrations
    pub migrations_applied: usize,
    /// When last migration was applied
    pub last_applied_at: Option<DateTime<Utc>>,
}
```
