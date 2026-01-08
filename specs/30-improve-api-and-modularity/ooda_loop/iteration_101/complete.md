# OODA Loop Iteration 101: Move PostgreSQL Services to edgequake-core

**Date**: 2026-01-08
**Status**: ✅ Complete
**Focus**: Architectural refactoring - correct service layer placement

## Observe

Identified two files in `edgequake-api` that were misplaced:
- `postgres_conversation_service.rs` (430 lines)
- `postgres_workspace_service.rs` (817 lines)

These files implement `ConversationService` and `WorkspaceService` traits, which are:
1. Defined in `edgequake-core`
2. Have in-memory implementations already in `edgequake-core`
3. Are service-layer abstractions, not HTTP-layer code

**Problem**: API crate should only contain HTTP handling logic, not database service implementations.

## Orient

Analyzed the dependency graph:
```
edgequake-api → edgequake-core → edgequake-storage
                     ↑
               PostgreSQL services belong here
               (same level as InMemoryXxxService)
```

The PostgreSQL services:
- Implement traits from `edgequake-core`
- Use storage layer from `edgequake-storage`
- Are logically siblings to `InMemoryWorkspaceService` and `InMemoryConversationService`

## Decide

**Decision**: Move PostgreSQL service implementations to `edgequake-core`

Changes required:
1. Add `postgres` feature to `edgequake-core/Cargo.toml`
2. Create new files in `edgequake-core/src/`:
   - `postgres_workspace_service.rs`
   - `postgres_conversation_service.rs`
3. Update `edgequake-core/src/lib.rs` to export conditionally
4. Update `edgequake-api/Cargo.toml` to enable `edgequake-core/postgres` feature
5. Update `edgequake-api` imports to use `edgequake_core::PostgresXxxService`
6. Remove old files from `edgequake-api`

## Act

### 1. Updated edgequake-core/Cargo.toml
Added features section:
```toml
[features]
default = []
postgres = ["edgequake-storage/postgres", "dep:sqlx"]
```
Added sqlx as optional dependency.

### 2. Created postgres_workspace_service.rs in edgequake-core
- 820 lines with WHY documentation
- Feature-gated with `#[cfg(feature = "postgres")]`
- Uses `crate::` imports instead of `edgequake_core::`

### 3. Created postgres_conversation_service.rs in edgequake-core
- 380 lines with WHY documentation
- Feature-gated with `#[cfg(feature = "postgres")]`

### 4. Updated edgequake-core/src/lib.rs
```rust
#[cfg(feature = "postgres")]
mod postgres_conversation_service;
#[cfg(feature = "postgres")]
mod postgres_workspace_service;

#[cfg(feature = "postgres")]
pub use postgres_conversation_service::PostgresConversationService;
#[cfg(feature = "postgres")]
pub use postgres_workspace_service::PostgresWorkspaceService;
```

### 5. Updated edgequake-api/Cargo.toml
```toml
postgres = ["edgequake-storage/postgres", "edgequake-core/postgres", "dep:sqlx"]
```

### 6. Updated edgequake-api/src/lib.rs
Changed from local module to re-export:
```rust
#[cfg(feature = "postgres")]
pub use edgequake_core::PostgresConversationService;

#[cfg(feature = "postgres")]
pub use edgequake_core::PostgresWorkspaceService;
```

### 7. Updated edgequake-api/src/state.rs
```rust
#[cfg(feature = "postgres")]
use edgequake_core::PostgresConversationService;
#[cfg(feature = "postgres")]
use edgequake_core::PostgresWorkspaceService;
```

### 8. Removed old files from edgequake-api

## Verification

- ✅ `cargo build --package edgequake-core --features postgres` - Success
- ✅ `cargo build --package edgequake-api --features postgres` - Success
- ✅ `cargo test --workspace --lib` - 1,351 tests passed
- ✅ `cargo clippy --package edgequake-core --features postgres` - 0 warnings

## Architecture After

```
edgequake-core/
├── src/
│   ├── workspace_service.rs        # Trait + InMemoryWorkspaceService
│   ├── postgres_workspace_service.rs   # NEW: PostgresWorkspaceService
│   ├── conversation_service.rs     # Trait + InMemoryConversationService
│   └── postgres_conversation_service.rs # NEW: PostgresConversationService

edgequake-api/
├── src/
│   ├── state.rs      # Uses edgequake_core::PostgresXxxService
│   └── lib.rs        # Re-exports from edgequake-core
    # REMOVED: postgres_conversation_service.rs
    # REMOVED: postgres_workspace_service.rs
```

## Commits

- `refactor(core): Move PostgreSQL services from API to Core layer`
