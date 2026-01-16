# OODA Loop Iteration 228 - OBSERVE

## Objective

Audit all remaining `ProviderFactory::create_*` calls and ensure safety limits are applied.

## Current State

### Provider Creation Locations

| File         | Line | Method                           | Safe?     | Notes               |
| ------------ | ---- | -------------------------------- | --------- | ------------------- |
| resolver.rs  | 299  | `create_safe_embedding_provider` | ✅        | New resolver        |
| resolver.rs  | 362  | `create_safe_llm_provider`       | ✅        | New resolver        |
| state.rs     | 928  | `create_safe_llm_provider`       | ✅        | Pipeline creation   |
| state.rs     | 931  | `create_safe_embedding_provider` | ✅        | Pipeline creation   |
| processor.rs | 228  | `create_safe_llm_provider`       | ✅        | Document processing |
| processor.rs | 231  | `create_safe_embedding_provider` | ✅        | Document processing |
| query.rs     | 522  | `create_embedding_provider`      | ⚠️ **NO** | Query handler       |

### Critical Finding: query.rs Uses Unsafe Provider Creation

```rust
// Line 522 in query.rs - NO SAFETY LIMITS!
let provider = ProviderFactory::create_embedding_provider(
    &workspace.embedding_provider,
    &workspace.embedding_model,
    workspace.embedding_dimension,
)
```

This function creates an embedding provider without:

- Timeout limits
- Rate limiting
- Connection pooling

### Impact

Query operations can:

1. Hang indefinitely if the embedding API is slow
2. Overwhelm the API with concurrent requests
3. Cause resource exhaustion in the backend

## Root Cause

The `get_workspace_embedding_provider` function in `query.rs` was written before the safe provider pattern was established.

## Recommendation

Change `create_embedding_provider` to `create_safe_embedding_provider` in query.rs line 522.
