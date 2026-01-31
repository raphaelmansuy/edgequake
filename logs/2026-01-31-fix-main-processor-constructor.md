# Fix: Missing llm_provider Argument in DocumentTaskProcessor Constructors

**Date:** 2026-01-31  
**Type:** Bug Fix  
**Files Modified:** 1  
**Commit:** 3f80c6a8

## Problem

When running `make dev`, the backend failed to compile with 12 errors:

```
error[E0308]: mismatched types - Expected LLMProvider, found KVStorage
error[E0308]: mismatched types - Expected KVStorage, found VectorStorage
error[E0308]: mismatched types - Expected VectorStorage, found WorkspaceVectorRegistry
error[E0308]: mismatched types - Expected WorkspaceVectorRegistry, found GraphStorage
error[E0308]: mismatched types - Expected WorkspaceService, found ModelsConfig
error[E0061]: this function takes 9 arguments but 8 arguments were supplied
```

## Root Cause

The `DocumentTaskProcessor` constructors in `edgequake/src/main.rs` were missing the `llm_provider` argument as the 2nd parameter after `pipeline`. This caused all subsequent arguments to be off-by-one, leading to type mismatches.

**Expected signature** (from processor.rs lines 127, 159):

```rust
pub fn with_workspace_support(
    pipeline: Arc<Pipeline>,
    llm_provider: Arc<dyn LLMProvider>,  // <-- MISSING
    kv_storage: Arc<dyn KVStorage>,
    vector_storage: Arc<dyn VectorStorage>,
    vector_registry: Arc<dyn WorkspaceVectorRegistry>,
    graph_storage: Arc<dyn GraphStorage>,
    pipeline_state: PipelineState,
    workspace_service: SharedWorkspaceService,
    models_config: Arc<ModelsConfig>,
) -> Self
```

**Actual calls** (main.rs lines 83-104):

```rust
// Missing llm_provider argument!
Arc::new(DocumentTaskProcessor::with_workspace_support_strict(
    Arc::clone(&state.pipeline),
    Arc::clone(&state.kv_storage),      // Position 2 (expected: llm_provider)
    Arc::clone(&state.vector_storage),  // Position 3 (expected: kv_storage)
    // ... rest misaligned
))
```

## Solution

Added `Arc::clone(&state.llm_provider)` as the 2nd argument in both constructor calls.

**File:** `edgequake/src/main.rs`

Applied to both:

- `with_workspace_support_strict()` (line 84)
- `with_workspace_support()` (line 96)

## Verification

1. **Compilation:** ✅ `cargo check --bin edgequake` passes
2. **Backend Start:** ✅ Started successfully on http://localhost:8080
3. **Health Check:** ✅ Status: healthy, storage: postgresql, provider: ollama

## Impact

- ✅ Backend compilation fixed (12 errors → 0 errors)
- ✅ Backend starts successfully with correct providers
- ✅ All 9 constructor arguments now aligned correctly
- ✅ Workspace isolation working (STRICT mode for PostgreSQL)

## Testing

- `make dev` starts backend successfully
- Health endpoint responds correctly
- WebSocket connections working
- Document and task endpoints responding

---

**Actions:** Fixed missing argument, verified compilation, tested runtime  
**Decisions:** Applied fix to both constructors for consistency  
**Lessons:** Constructor argument order must match trait definition exactly
