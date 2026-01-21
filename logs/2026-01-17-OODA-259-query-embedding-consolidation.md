# OODA-259: Query Embedding Provider Consolidation

**Date**: 2026-01-17
**Status**: ✅ COMPLETE

## Problem Statement

The `get_workspace_embedding_provider` function in `handlers/query.rs` duplicated logic
that already existed in `providers/resolver.rs`. This violated the Single Source of Truth
principle and made maintenance harder.

## Solution

1. Added `resolve_embedding_provider_optional` method to `WorkspaceProviderResolver`:

   - Returns `Ok(None)` for fallback semantics (vs. `resolve_embedding_provider` which errors)
   - Handles workspace lookup, empty provider check, provider creation
   - Logs warnings with actionable messages for API key errors
   - Returns `None` for fallback instead of hard error

2. Refactored `get_workspace_embedding_provider` in `query.rs`:
   - Now delegates to `WorkspaceProviderResolver::resolve_embedding_provider_optional`
   - Reduced from ~80 lines to ~10 lines
   - Eliminated direct `ProviderFactory` usage

## Files Modified

- `edgequake/crates/edgequake-api/src/providers/resolver.rs`:

  - Added `resolve_embedding_provider_optional` method (~80 lines)

- `edgequake/crates/edgequake-api/src/handlers/query.rs`:
  - Added import for `WorkspaceProviderResolver`
  - Refactored `get_workspace_embedding_provider` to delegate to resolver

## Test Results

```
cargo test -p edgequake-api

test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Benefits

1. **Single Source of Truth**: All embedding provider resolution now goes through `WorkspaceProviderResolver`
2. **Consistent Error Handling**: Unified logging and error messages
3. **Reduced Code Duplication**: ~70 lines of duplicated logic removed
4. **Better Testability**: Resolver can be mocked for unit testing

## Related

- OODA-226: Initial WorkspaceProviderResolver implementation
- OODA-257: Duplication audit identifying this issue
- SPEC-032: Workspace-specific embedding in query process

## Task Logs

- **Actions**: Added `resolve_embedding_provider_optional` method, refactored `get_workspace_embedding_provider`
- **Decisions**: Used delegation pattern rather than moving function to avoid breaking existing callers
- **Next steps**: Continue with remaining OODA loops (260+)
- **Lessons**: Fallback semantics (returning None vs. error) are important API design decisions
