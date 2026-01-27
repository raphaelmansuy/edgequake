# OODA Loop Iteration 227 - DECIDE

## Decision: Refactor chat.rs to use WorkspaceProviderResolver

### Approach

Replace inline provider resolution in both handlers with resolver calls.

### Changes Made

1. **Add import** for `LlmResolutionRequest` and `WorkspaceProviderResolver`

2. **Replace non-streaming handler** (lines 378-462):

   - Old: 85 lines of nested if/else with `ProviderFactory::create_llm_provider`
   - New: 15 lines using `resolver.resolve_llm_provider_with_workspace`

3. **Replace streaming handler** (lines 810-900):
   - Old: 90 lines of nearly identical nested if/else
   - New: 15 lines using same resolver

### Error Handling Preservation

| Scenario                 | Non-Streaming          | Streaming                            |
| ------------------------ | ---------------------- | ------------------------------------ |
| Explicit provider fails  | `ApiError::BadRequest` | `ChatStreamEvent::Error` via channel |
| Workspace provider fails | Falls back to default  | Falls back to default                |
| No provider specified    | Uses server default    | Uses server default                  |

### Code Reduction

- **Before**: 175 lines of duplicated logic
- **After**: 30 lines of unified calls + 200 lines in resolver (shared)
- **Net reduction**: ~145 lines of duplicated code

## Rationale

1. **DRY Principle**: Don't Repeat Yourself - single implementation reduces bugs
2. **Safety by Default**: Resolver uses safe provider creation with timeouts
3. **Testability**: Resolver can be unit tested in isolation
4. **Maintainability**: Future changes only need to be made in one place
