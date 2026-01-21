# OODA Loop Iteration 226 - Decide

## Date: 2026-01-16

## Decision: Create WorkspaceProviderResolver Module

### Approach

Create a new module `edgequake-api/src/providers/` with a unified provider resolution system.

### Implementation Plan

#### Phase 1: Create Resolver Module (This Iteration)

1. **File**: `edgequake-api/src/providers/mod.rs`

   - Export `WorkspaceProviderResolver`
   - Export `ProviderResolutionError`

2. **File**: `edgequake-api/src/providers/resolver.rs`

   - Implement `WorkspaceProviderResolver` struct
   - Method: `resolve_llm_provider()`
   - Method: `resolve_embedding_provider()`
   - Always use `create_safe_*` methods

3. **File**: `edgequake-api/src/providers/error.rs`
   - Define `ProviderResolutionError` enum
   - Include `is_api_key_error` flag

#### Phase 2: Refactor Handlers (OODA 227-228)

1. Update `chat.rs` to use resolver
2. Update `processor.rs` to use resolver
3. Update `query.rs` to use resolver
4. Update `state.rs` to use resolver

#### Phase 3: Add Tests (OODA 229-230)

1. Unit tests for resolver
2. Integration tests for all refactored handlers
3. Property-based tests for invariants

### Code Structure

```
edgequake/crates/edgequake-api/src/
├── providers/
│   ├── mod.rs          # Module exports
│   ├── resolver.rs     # WorkspaceProviderResolver
│   └── error.rs        # ProviderResolutionError
├── handlers/
│   ├── chat.rs         # Uses resolver
│   ├── query.rs        # Uses resolver
│   └── ...
├── processor.rs        # Uses resolver
└── state.rs            # Uses resolver
```

### Key Design Decisions

1. **Always Safe**: Use `create_safe_*` methods with timeouts
2. **API Key Detection**: Universal detection for all providers
3. **Result-Based**: Return errors, let callers handle
4. **Logging**: Structured logging in resolver, not callers
5. **Testability**: Resolver is mockable for testing

### Risk Mitigation

| Risk                       | Mitigation                         |
| -------------------------- | ---------------------------------- |
| Breaking existing behavior | Keep old code until all tests pass |
| Missing edge cases         | Comprehensive test suite           |
| Performance regression     | Profile before/after               |
| Incomplete refactor        | Feature flag to toggle old/new     |

### Success Criteria

1. All 18 provider creation sites use resolver
2. Zero code duplication for provider resolution
3. All existing tests pass
4. New tests for resolver achieve >95% coverage
5. Safety limits applied universally

## Action Items for This Iteration

1. Create `providers/` directory structure
2. Implement `ProviderResolutionError`
3. Implement `WorkspaceProviderResolver::resolve_llm_provider()`
4. Add basic unit tests
5. Document the new module
