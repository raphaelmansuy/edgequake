# EdgeQuake Infrastructure Improvement Plan V2

## Status: 🚀 IN PROGRESS

## Date: 2025-01-27

---

## Executive Summary

This plan addresses 7 key improvements to make EdgeQuake more robust for production use and agent-friendly:

1. **Auto-create database schema** - Ensure schema exists before migrations run
2. **Display storage mode on startup** - Clear banner showing memory vs database mode
3. **Database as default mode** - Refactor Makefile so PostgreSQL is default, memory for testing
4. **Background/agentic mode support** - Add `make dev-bg` and similar commands
5. **Health API reports storage mode** - Add `storage_mode` field to `/health` endpoint
6. **Deep testing from scratch** - Test full lifecycle with fresh database
7. **Schema auto-update** - Ensure all tables/columns are created via migrations

---

## Current State Analysis

### Key Files

| File                                                    | Purpose                    | Lines    |
| ------------------------------------------------------- | -------------------------- | -------- |
| `Makefile`                                              | Build/run orchestration    | 363      |
| `edgequake/src/main.rs`                                 | Backend entry point        | 102      |
| `edgequake/crates/edgequake-api/src/state.rs`           | AppState with storage init | 641      |
| `edgequake/crates/edgequake-api/src/handlers/health.rs` | Health endpoint            | 117      |
| `edgequake/docker/init.sql`                             | Production DB init         | 1056     |
| `edgequake/migrations/`                                 | SQLx migrations            | 11 files |

### Current Behavior

1. **Storage Mode Decision** (main.rs:31-38):

   ```rust
   if let Ok(database_url) = std::env::var("DATABASE_URL") {
       AppState::new_postgres(&database_url, &api_key)
   } else {
       AppState::new_memory(&api_key)
   }
   ```

2. **Makefile targets**:

   - `make dev` → Sets `DATABASE_URL` → PostgreSQL mode
   - `make backend-dev` → No `DATABASE_URL` → In-memory mode
   - No background mode support

3. **Health Response** - Missing `storage_mode` field

4. **Schema Creation**:
   - `docker/init.sql` creates tables in Docker initialization
   - `migrations/` runs via SQLx but schema itself may not exist

---

## Implementation Plan

### Phase 1: Schema Auto-Creation ✓

**Goal**: Ensure `edgequake` schema and all tables are auto-created

**Changes**:

1. Add schema creation to `state.rs` before running migrations
2. Add first migration that creates schema if not exists
3. Ensure `CREATE SCHEMA IF NOT EXISTS` runs before any table creation

**Files to modify**:

- `edgequake/crates/edgequake-api/src/state.rs`
- `edgequake/migrations/001_create_tasks_table.sql` (add schema creation)

### Phase 2: Storage Mode Display ✓

**Goal**: Clear, prominent banner on startup showing storage mode

**Changes**:

1. Add `storage_mode` field to AppState
2. Display ASCII banner on startup with storage mode
3. Log detailed storage information

**Files to modify**:

- `edgequake/src/main.rs`
- `edgequake/crates/edgequake-api/src/state.rs`

### Phase 3: Health API Enhancement ✓

**Goal**: Health endpoint reports storage mode

**Changes**:

1. Add `storage_mode: String` to `HealthResponse`
2. Add `database_url_set: bool` for debugging
3. Report "memory" or "postgresql" storage type

**Files to modify**:

- `edgequake/crates/edgequake-api/src/handlers/health.rs`
- `edgequake/crates/edgequake-api/src/state.rs` (expose storage_mode)

### Phase 4: Makefile Refactoring ✓

**Goal**: Database as default, agentic mode support

**New targets**:

| Target                | Purpose                          | Storage    |
| --------------------- | -------------------------------- | ---------- |
| `make dev`            | Full stack (default=database)    | PostgreSQL |
| `make dev-bg`         | Full stack in background         | PostgreSQL |
| `make dev-memory`     | Development with in-memory       | Memory     |
| `make start-bg`       | Start all services in background | PostgreSQL |
| `make backend-db`     | Backend with database (explicit) | PostgreSQL |
| `make backend-memory` | Backend with memory (testing)    | Memory     |

**Behavior changes**:

- `make backend-dev` → renamed to `make backend-memory`
- `make dev` → unchanged (already uses DATABASE_URL)
- New: `make dev-bg` for agentic mode compatibility

### Phase 5: Deep Testing ✓

**Goal**: Test complete lifecycle from scratch

**Test scenarios**:

1. Fresh database - no schema, no tables
2. Existing database - with old schema
3. Database with new columns needed
4. Memory mode fallback
5. Health API verification

---

## Todo Checklist

```markdown
- [ ] 1. Add storage_mode field to AppState struct
- [ ] 2. Create schema auto-creation in state.rs before migrations
- [ ] 3. Update migrations to use IF NOT EXISTS everywhere
- [ ] 4. Add startup banner showing storage mode
- [ ] 5. Add storage_mode to HealthResponse
- [ ] 6. Refactor Makefile - database as default
- [ ] 7. Add make dev-bg for background mode
- [ ] 8. Add make backend-memory for testing
- [ ] 9. Test fresh database scenario
- [ ] 10. Test existing database scenario
- [ ] 11. Test health API returns storage_mode
- [ ] 12. Document changes
```

---

## Detailed Implementation

### 1. AppState Storage Mode Field

```rust
// In state.rs
pub struct AppState {
    // ... existing fields
    pub storage_mode: StorageMode,
}

#[derive(Clone, Debug)]
pub enum StorageMode {
    Memory,
    PostgreSQL { url_masked: String },
}

impl StorageMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            StorageMode::Memory => "memory",
            StorageMode::PostgreSQL { .. } => "postgresql",
        }
    }
}
```

### 2. Schema Auto-Creation

```rust
// In new_postgres() before migrations
sqlx::query("CREATE SCHEMA IF NOT EXISTS edgequake")
    .execute(&pool)
    .await?;
```

### 3. Startup Banner

```rust
// In main.rs
fn print_banner(storage_mode: &str, version: &str) {
    println!("\n{}", "═".repeat(60));
    println!("  🚀 EdgeQuake v{}", version);
    println!("  📦 Storage: {}", storage_mode.to_uppercase());
    println!("{}\n", "═".repeat(60));
}
```

### 4. Health Response Enhancement

```rust
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub storage_mode: String,  // NEW: "memory" or "postgresql"
    // ... existing fields
}
```

### 5. Makefile Changes

```makefile
# Default development with database
dev: db-wait
	@echo "Starting full stack with PostgreSQL storage..."
	$(MAKE) -j2 backend-db frontend-dev

# Background mode for agents
dev-bg: db-wait
	@echo "Starting full stack in background..."
	$(MAKE) backend-bg &
	$(MAKE) frontend-bg &
	@echo "Services starting in background. Use 'make status' to check."

# Memory mode for testing
dev-memory:
	@echo "Starting with in-memory storage (for testing)..."
	$(MAKE) -j2 backend-memory frontend-dev

# Backend with database (explicit)
backend-db: db-wait
	@export DATABASE_URL=$(DATABASE_URL) && \
	cd edgequake && cargo run

# Backend with memory
backend-memory:
	@cd edgequake && cargo run
```

---

## Testing Plan

### Test 1: Fresh Database

```bash
# Stop and remove existing database
make db-stop
docker volume rm edgequake-postgres-data

# Start fresh
make dev

# Verify schema and tables created
docker exec edgequake-postgres psql -U edgequake -d edgequake -c "\dt"
```

### Test 2: Health API

```bash
# Check health endpoint returns storage_mode
curl http://localhost:8080/health | jq '.storage_mode'
# Expected: "postgresql"

# Check memory mode
make stop
make dev-memory &
curl http://localhost:8080/health | jq '.storage_mode'
# Expected: "memory"
```

### Test 3: Background Mode

```bash
# Start in background
make dev-bg

# Verify services are running
make status

# Stop all
make stop
```

---

## Success Criteria

1. ✅ Fresh database auto-creates schema and all tables
2. ✅ Console clearly shows "POSTGRESQL" or "MEMORY" storage mode
3. ✅ `make dev` uses database by default
4. ✅ `make dev-bg` starts everything in background without blocking terminal
5. ✅ `/health` endpoint returns `storage_mode` field
6. ✅ All existing tests pass
7. ✅ No manual intervention needed for database setup

---

## Risk Mitigation

1. **Backwards Compatibility**: Existing `make dev` behavior unchanged
2. **Migration Safety**: All migrations use `IF NOT EXISTS`
3. **Testing**: Both memory and PostgreSQL modes tested
4. **Documentation**: Clear comments in Makefile and code

---

## Next Steps After Completion

1. Update AGENTS.md with new Makefile commands
2. Add section on agentic mode compatibility
3. Consider adding `make agent-start` alias for clarity
