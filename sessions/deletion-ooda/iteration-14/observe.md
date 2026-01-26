# OODA Iteration 14 - Observe

**Mission Re-read**: specs/033-study-delete-document/003-study-document.md

## Focus Area: Schema Version Verification

The mission requires:

> "Ensure to have function that verify the integrity of schema against the version of edgequake running."

## Current State

### 1. SQLx Migrations Table

SQLx automatically manages `_sqlx_migrations` table with:

- `version` - Migration version number
- `description` - Migration name
- `installed_on` - Timestamp
- `success` - Boolean
- `checksum` - File hash for integrity

### 2. Existing Health Check

**File**: `edgequake/crates/edgequake-api/src/handlers/health.rs`

Current health response:

```rust
HealthResponse {
    status: "healthy",
    version: env!("CARGO_PKG_VERSION"),
    storage_mode: "postgres" | "memory",
    workspace_id: ...,
    components: { kv: bool, vector: bool, graph: bool, llm: bool },
    llm_provider_name: ...,
}
```

**MISSING**: No schema version in health response.

### 3. No Schema Validation

Current implementation does not verify:

- Expected migrations are applied
- Schema is compatible with current version
- Core tables exist

## Gap Analysis

| Requirement                | Current State         | Gap       |
| -------------------------- | --------------------- | --------- |
| Schema version tracking    | SQLx migrations exist | ✅        |
| Version in health response | Not included          | ❌ GAP-14 |
| Schema validation function | Not implemented       | ❌ GAP-14 |
| Integrity check            | Not implemented       | ❌ GAP-14 |

## Architecture Consideration

### Where to Add Schema Check?

1. **Health endpoint** - Add `schema_version` field to HealthResponse
2. **Startup validation** - Check schema on server start
3. **Dedicated endpoint** - New `/api/v1/schema` endpoint

### PostgreSQL Query for Migration Count

```sql
SELECT
    COUNT(*) as applied_count,
    MAX(version) as latest_version,
    MAX(installed_on) as last_applied_at
FROM _sqlx_migrations
WHERE success = true;
```

### Expected Migrations (as of current codebase)

Need to count migrations in `edgequake/migrations/` directory.
