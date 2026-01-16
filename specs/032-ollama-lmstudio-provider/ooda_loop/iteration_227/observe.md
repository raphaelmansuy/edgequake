# OODA Loop Iteration 227 - OBSERVE

## Objective

Refactor `chat.rs` to use `WorkspaceProviderResolver` instead of inline provider creation logic.

## Current State

### Duplication Analysis

Two nearly identical code blocks exist:

1. **Non-streaming handler** (lines 385-455): Creates LLM provider with inline logic
2. **Streaming handler** (lines 855-935): Creates LLM provider with same inline logic

### Key Differences Between Handlers

| Aspect | Non-Streaming | Streaming |
|--------|---------------|-----------|
| Error handling | Returns `ApiError::BadRequest` | Sends `ChatStreamEvent::Error` via channel |
| Provider creation | `ProviderFactory::create_llm_provider` | `ProviderFactory::create_llm_provider` |
| Safety limits | **NONE** (no timeout) | **NONE** (no timeout) |
| Variable names | `request.provider`, `request.model` | `request_provider`, `request_model` |

### Critical Bug Discovered

Both handlers use `ProviderFactory::create_llm_provider` (NO SAFETY LIMITS) while `processor.rs` uses `ProviderFactory::create_safe_llm_provider` (WITH TIMEOUTS).

This means:
- Document ingestion has 300s timeout protection
- Chat queries have NO timeout protection - can hang indefinitely

### Code to Refactor

```rust
// Lines 385-455 (non-streaming) - 70 lines
let (llm_override, used_provider, used_model) = if let Some(ref provider_id) = request.provider
{
    // ... 60+ lines of nested if/else logic
}

// Lines 855-935 (streaming) - 80 lines  
let (llm_override, used_provider, used_model) = if let Some(ref provider_id) = request_provider
{
    // ... 70+ lines of nearly identical nested if/else logic
}
```

## Files to Modify

1. `edgequake/crates/edgequake-api/src/handlers/chat.rs`
   - Add import for `WorkspaceProviderResolver` and related types
   - Refactor non-streaming handler to use resolver
   - Refactor streaming handler to use resolver

## Success Criteria

1. Both handlers use `WorkspaceProviderResolver`
2. Both handlers now use **safe provider creation** with timeouts
3. All tests pass
4. No functional behavior changes except timeout protection

## Risks

1. Error handling differs between handlers - must preserve behavior
2. Workspace is already loaded before provider resolution - need to pass it correctly
3. Variable names differ - need careful adaptation
