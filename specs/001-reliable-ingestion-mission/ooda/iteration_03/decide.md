# OODA Iteration 03 - Decide

## Mission Re-Read Checkpoint
✅ Mission file re-read: `./specs/001-reliable-ingestion-mission.md`

## Decision: Require DATABASE_URL for Server Startup

Based on the mission directive to "eliminate all in-memory storage providers," this iteration implements Phase 1 of the conditional compilation approach:

**Make DATABASE_URL required for server startup.**

## Specific Changes

### Change 1: Modify main.rs to Require DATABASE_URL

**File:** `edgequake/src/main.rs`
**Lines:** ~248-265

**Current Code:**
```rust
let state = if let Ok(database_url) = std::env::var("DATABASE_URL") {
    info!("🐘 DATABASE_URL detected - using PostgreSQL storage");
    AppState::new_postgres(&database_url, &api_key)
        .await
        .expect("Failed to initialize PostgreSQL storage")
} else {
    warn!("⚠️ WARNING: No DATABASE_URL set - using IN-MEMORY storage.");
    warn!("   Data WILL NOT PERSIST across restarts. NOT FOR PRODUCTION USE.");
    warn!("   Set DATABASE_URL to use PostgreSQL for production.");
    AppState::new_memory(if api_key.is_empty() {
        None
    } else {
        Some(api_key)
    })
};
```

**New Code:**
```rust
// OODA-03: DATABASE_URL is now REQUIRED - in-memory storage removed for production consistency
// WHY: Mission directive requires eliminating in-memory providers to ensure:
// 1. Consistent behavior between dev and production
// 2. No accidental data loss from memory mode
// 3. Proper testing against real storage
let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
    error!("═══════════════════════════════════════════════════════════════════════");
    error!(" FATAL: DATABASE_URL environment variable is REQUIRED");
    error!("═══════════════════════════════════════════════════════════════════════");
    error!(" In-memory storage has been removed for production consistency.");
    error!(" Please set DATABASE_URL to a PostgreSQL connection string:");
    error!("");
    error!("   export DATABASE_URL=\"postgresql://user:pass@localhost:5432/edgequake\"");
    error!("");
    error!(" Or use the Makefile:");
    error!("   make dev          # Starts with PostgreSQL (recommended)");
    error!("   make backend-dev  # Backend only with PostgreSQL");
    error!("═══════════════════════════════════════════════════════════════════════");
    std::process::exit(1);
});

info!("🐘 PostgreSQL storage mode (DATABASE_URL detected)");
let state = AppState::new_postgres(&database_url, &api_key)
    .await
    .expect("Failed to initialize PostgreSQL storage");
```

### Change 2: Remove Unused Memory Mode Logic

**Files to Update:**
- Remove the `else` branch that calls `new_memory()`
- Remove non-strict workspace mode logic (lines ~298-301)

### Change 3: Update Makefile backend-memory Target

**File:** `Makefile`

**Option A (Remove):**
```makefile
# REMOVED: backend-memory target - in-memory storage no longer supported
# Use: make backend-dev (requires PostgreSQL)
```

**Option B (Make it fail with helpful message):**
```makefile
backend-memory: ## DEPRECATED - use backend-dev with PostgreSQL
	@echo "$(RED)ERROR: In-memory storage has been removed$(RESET)"
	@echo "$(RED)Please use 'make backend-dev' with PostgreSQL instead$(RESET)"
	@exit 1
```

**Decision:** Option B (keep target but make it fail with guidance)

### NOT In Scope (Deferred to Iteration 04+)

- Conditional compilation of memory adapters (`#[cfg(test)]`)
- Removal of memory adapter source files
- Test updates for PostgreSQL-only
- AGENTS.md documentation update

## Implementation Checklist

1. [ ] Update main.rs to require DATABASE_URL
2. [ ] Remove memory fallback code
3. [ ] Remove non-strict workspace mode fallback
4. [ ] Update Makefile backend-memory to fail with guidance
5. [ ] Build and verify compilation
6. [ ] Test that server fails without DATABASE_URL
7. [ ] Test that server works with DATABASE_URL
8. [ ] Commit with `OODA-03: Require DATABASE_URL, remove memory fallback`

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Tests may fail | Medium | Tests should use DATABASE_URL too |
| Developer friction | Low | Clear error message with fix |
| CI breaks | Medium | CI should have DATABASE_URL |

## Success Criteria

- [ ] `cargo run` without DATABASE_URL exits with error code 1
- [ ] Error message clearly explains what to do
- [ ] `DATABASE_URL=... cargo run` works correctly
- [ ] Build compiles without warnings
- [ ] Makefile `backend-memory` fails with guidance
