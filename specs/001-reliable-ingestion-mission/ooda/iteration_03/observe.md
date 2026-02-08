# OODA Iteration 03 - Observe

## Mission Re-Read Checkpoint

✅ Mission file re-read: `./specs/001-reliable-ingestion-mission.md`

## Critical Mission Alignment Issue

**Iteration 01 Decision:** Keep in-memory providers for testing
**Mission Directive:** "Remove In-Memory Providers: Eliminate all in-memory storage providers to ensure consistency - only PostgreSQL should be used"

⚠️ **These are in conflict!** The mission is clear: eliminate in-memory providers.

## Observation Summary

### 1. Service Status

- **Backend**: ✅ Healthy (http://localhost:8080/health)
  - Storage mode: postgresql
  - LLM provider: ollama
- **Frontend**: ✅ Running (http://localhost:3000)
- **Database**: ✅ PostgreSQL running (0 documents currently)

### 2. In-Memory Provider Inventory

**Location:** `edgequake/crates/edgequake-storage/src/adapters/memory/`

| File                  | Purpose                  | Lines | Impact |
| --------------------- | ------------------------ | ----- | ------ |
| `mod.rs`              | Module exports           | ~20   | Low    |
| `graph.rs`            | In-memory graph storage  | ~300  | High   |
| `kv.rs`               | In-memory KV storage     | ~200  | High   |
| `vector.rs`           | In-memory vector storage | ~400  | High   |
| `workspace_vector.rs` | Workspace vector storage | ~500  | High   |

**Total: ~1400 lines of in-memory provider code**

### 3. In-Memory Provider Usage in main.rs

```rust
// Line 254-262 (main.rs)
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

### 4. AppState Memory Constructor

Need to locate `AppState::new_memory()` to understand full impact.

```
edgequake-api/src/state.rs  → likely location
```

### 5. Test Dependencies on Memory Providers

Need to audit which tests depend on in-memory storage:

- Unit tests that mock storage
- Integration tests without database
- CI/CD pipelines without PostgreSQL

### 6. Risk Assessment for Removal

| Risk                   | Impact | Mitigation                      |
| ---------------------- | ------ | ------------------------------- |
| Break unit tests       | High   | Update tests to use mock traits |
| Break CI/CD            | Medium | Require PostgreSQL in CI        |
| Remove dev convenience | Low    | Dev env setup documentation     |
| Compilation errors     | High   | Systematic removal              |

### 7. Architecture: Storage Selection Flow

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                        Current Storage Selection Flow                         │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   Server Start                                                               │
│       │                                                                      │
│       ▼                                                                      │
│   ┌─────────────────────┐                                                   │
│   │ Check DATABASE_URL  │                                                   │
│   └─────────────────────┘                                                   │
│       │                                                                      │
│       ├──── Set ────────────────────────┐                                   │
│       │                                  ▼                                   │
│       │                          ┌──────────────────┐                       │
│       │                          │   PostgreSQL     │                       │
│       │                          │   (Production)   │                       │
│       │                          └──────────────────┘                       │
│       │                                                                      │
│       └──── Not Set ────────────┐                                           │
│                                  ▼                                           │
│                          ┌──────────────────┐                               │
│                          │   In-Memory      │  ← TO BE REMOVED              │
│                          │   (Testing)      │                               │
│                          └──────────────────┘                               │
│                                                                              │
│   NEW: Require DATABASE_URL or exit with error                              │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 8. Proposed New Flow

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                        Proposed Storage Selection Flow                        │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   Server Start                                                               │
│       │                                                                      │
│       ▼                                                                      │
│   ┌─────────────────────┐                                                   │
│   │ Check DATABASE_URL  │                                                   │
│   └─────────────────────┘                                                   │
│       │                                                                      │
│       ├──── Set ────────────────────────┐                                   │
│       │                                  ▼                                   │
│       │                          ┌──────────────────┐                       │
│       │                          │   PostgreSQL     │                       │
│       │                          │   (Always)       │                       │
│       │                          └──────────────────┘                       │
│       │                                                                      │
│       └──── Not Set ────────────┐                                           │
│                                  ▼                                           │
│                          ┌──────────────────┐                               │
│                          │   EXIT WITH      │                               │
│                          │   ERROR MESSAGE  │                               │
│                          └──────────────────┘                               │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

## Files Requiring Modification

### Must Change

1. `edgequake/src/main.rs` - Remove memory fallback, require DATABASE_URL
2. `edgequake-api/src/state.rs` - Remove `new_memory()` or mark as test-only
3. `Makefile` - Remove `backend-memory` target or make it fail

### May Need Conditional Compilation

1. `edgequake-storage/src/adapters/memory/*` - Keep for tests only
2. Tests - Update to use PostgreSQL or mock traits

## Decision Required

**Option A:** Complete removal of in-memory providers (MISSION ALIGNED)

- Remove memory adapter code
- Require DATABASE_URL for server startup
- Update all tests to use PostgreSQL or mocks

**Option B:** Conditional compilation with `#[cfg(test)]` (COMPROMISE)

- Keep memory adapters for tests only
- Require DATABASE_URL for `cargo run`
- Tests can still use memory adapters

**Option C:** Keep current with stronger warnings (CURRENT STATE)

- ❌ NOT MISSION ALIGNED

## Next Steps

1. Locate `AppState::new_memory()` implementation
2. Audit test dependencies on memory providers
3. Decide between Option A and Option B
4. Implement removal/restriction
