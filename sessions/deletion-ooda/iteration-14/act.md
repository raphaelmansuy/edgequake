# OODA Iteration 14 - Act

## Changes Implemented

### 1. Added SchemaHealth Struct

**File**: `edgequake/crates/edgequake-api/src/handlers/health_types.rs`
**Lines**: 47-58

```rust
pub struct SchemaHealth {
    pub latest_version: Option<i64>,
    pub migrations_applied: usize,
    pub last_applied_at: Option<String>,
}
```

### 2. Extended HealthResponse with Schema Field

**File**: `edgequake/crates/edgequake-api/src/handlers/health_types.rs`
**Lines**: 34-39

Added optional `schema` field to HealthResponse:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub schema: Option<SchemaHealth>,
```

### 3. Implemented Schema Query

**File**: `edgequake/crates/edgequake-api/src/handlers/health.rs`
**Lines**: 98-135

Added `get_schema_health()` function that queries `_sqlx_migrations` table:

```rust
SELECT
    COUNT(*) FILTER (WHERE success = true) as applied_count,
    MAX(version) FILTER (WHERE success = true) as latest_version,
    MAX(installed_on) FILTER (WHERE success = true) as last_applied_at
FROM _sqlx_migrations
```

- PostgreSQL mode: Returns actual migration stats
- Memory mode: Returns None (graceful degradation)

### 4. Added Unit Tests

- `test_health_response_with_schema` - Tests schema serialization
- `test_schema_health_serialization` - Tests optional field skipping

## Test Results

```
cargo test --package edgequake-api --lib
   test handlers::health_types::tests::test_schema_health_serialization ... ok
   test handlers::health_types::tests::test_health_response_with_schema ... ok
   test result: ok. 423 passed; 0 failed
```

## API Response Example (PostgreSQL)

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "storage_mode": "postgresql",
  "components": {...},
  "schema": {
    "latest_version": 15,
    "migrations_applied": 15,
    "last_applied_at": "2025-01-26T10:00:00Z"
  }
}
```

## API Response Example (Memory)

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "storage_mode": "memory",
  "components": {...}
  // no schema field (skipped when None)
}
```

## Commit

```bash
git commit -m "feat(health): add schema version to health endpoint (OODA-14)

- Add SchemaHealth struct with version, count, timestamp
- Query _sqlx_migrations for PostgreSQL mode
- Graceful degradation for memory mode (returns None)
- Add unit tests for serialization

Mission: verify schema integrity against running version"
```
