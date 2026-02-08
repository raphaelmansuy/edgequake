# OODA Iteration 03 - Orient

## Mission Re-Read Checkpoint
✅ Mission file re-read: `./specs/001-reliable-ingestion-mission.md`

## First Principles Analysis

**Mission Directive:** "Remove In-Memory Providers: Eliminate all in-memory storage providers to ensure consistency - only PostgreSQL should be used"

**First Principle:** Consistency and reliability are non-negotiable for production systems.

### Why In-Memory Providers Are Problematic

1. **Data Loss Risk**: In-memory storage loses all data on restart
2. **Inconsistent Behavior**: Different code paths for memory vs PostgreSQL create testing gaps
3. **Hidden Bugs**: Memory providers may mask bugs that only appear with real database
4. **Production Accidents**: Developers may accidentally deploy with memory mode
5. **Maintenance Burden**: Maintaining two implementations doubles complexity

### Why Tests Need Storage Abstractions

1. **Fast Tests**: Unit tests shouldn't need database connections
2. **CI Simplicity**: Some CI environments struggle with PostgreSQL
3. **Isolation**: Tests should be isolated from external state

## Option Analysis

### Option A: Complete Removal (MISSION MAXIMUM)

**Approach:**
- Delete `edgequake-storage/src/adapters/memory/` directory
- Remove `AppState::new_memory()` method
- Make DATABASE_URL required for server startup
- Update all tests to either:
  - Use real PostgreSQL (test containers)
  - Use mock traits with `mockall` or similar

**Pros:**
- ✅ Fully aligns with mission
- ✅ Eliminates code duplication
- ✅ Forces production-like testing
- ✅ No risk of accidental memory mode

**Cons:**
- ❌ Major refactor (~1400 lines to remove)
- ❌ All tests need updating
- ❌ CI needs PostgreSQL setup
- ❌ Developer setup more complex

**Effort:** HIGH (3-5 OODA iterations)

### Option B: Conditional Compilation (COMPROMISE)

**Approach:**
```rust
#[cfg(test)]
mod memory;  // Only compile for tests

// In main.rs
#[cfg(not(test))]
fn require_database_url() -> String {
    std::env::var("DATABASE_URL")
        .expect("DATABASE_URL is required for production. Use PostgreSQL.")
}
```

**Pros:**
- ✅ Production code requires DATABASE_URL
- ✅ Tests can still use memory for speed
- ✅ Moderate effort
- ✅ Same test infrastructure

**Cons:**
- ⚠️ Memory code still exists (in test builds)
- ⚠️ Tests may miss database-specific bugs
- ⚠️ Partial mission compliance

**Effort:** MEDIUM (1-2 OODA iterations)

### Option C: Runtime Guard Only (MINIMUM VIABLE)

**Approach:**
- Keep memory providers but require `--allow-memory` flag
- Default to DATABASE_URL required
- Explicit opt-in for testing

**Pros:**
- ✅ Minimal code changes
- ✅ Production safe by default
- ✅ Tests work unchanged

**Cons:**
- ❌ Does NOT align with mission (providers still "remain")
- ❌ Memory code still maintained
- ❌ Technical debt persists

**Effort:** LOW (1 iteration)

## Recommendation

**Recommended: Option B (Conditional Compilation)**

**Rationale:**
1. Balances mission alignment with practical constraints
2. Production-safe: DATABASE_URL required for release builds
3. Test-friendly: Memory providers available for unit tests
4. Achievable within 1-2 iterations
5. Can evolve to Option A later if needed

## Implementation Plan for Option B

### Phase 1: Make DATABASE_URL Required in Production

```rust
// main.rs changes
fn main() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| {
            eprintln!("ERROR: DATABASE_URL environment variable is required.");
            eprintln!("In-memory storage has been removed for production consistency.");
            eprintln!("Please set DATABASE_URL to a PostgreSQL connection string.");
            std::process::exit(1);
        })?;
    
    // Continue with PostgreSQL-only initialization
}
```

### Phase 2: Conditional Compilation for Memory Adapters

```rust
// edgequake-storage/src/adapters/mod.rs
#[cfg(any(test, feature = "memory-storage"))]
pub mod memory;

pub mod postgres;
```

### Phase 3: Update Makefile

- Remove `backend-memory` target OR
- Require `--features memory-storage` flag

### Phase 4: Update Tests

- Tests that need memory storage add: `#[cfg(feature = "memory-storage")]`
- Tests that want PostgreSQL use test containers or real DB

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Breaking tests | Run full test suite before merging |
| CI failures | Update CI to use test database |
| Developer friction | Document setup clearly |
| Emergency rollback | Feature flag allows re-enabling |

## Files to Modify

| File | Change | Priority |
|------|--------|----------|
| `edgequake/src/main.rs` | Remove memory fallback, require DATABASE_URL | P1 |
| `edgequake-storage/src/adapters/mod.rs` | Add `#[cfg(any(test, feature = "memory-storage"))]` | P1 |
| `edgequake-api/src/state.rs` | Conditionally compile `new_memory()` | P1 |
| `Makefile` | Remove/modify `backend-memory` | P2 |
| `AGENTS.md` | Update documentation | P3 |

## Decision Input for decide.md

Proceed with **Option B: Conditional Compilation**

1. Modify main.rs to require DATABASE_URL
2. Add conditional compilation to memory adapters
3. Update Makefile to remove backend-memory target
4. Run all tests to verify
5. Document changes in AGENTS.md
