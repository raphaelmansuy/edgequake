# OODA Loop Iteration 226 - Observe

## Date: 2026-01-16

## Mission Amendment: Code Reliability Audit

User requested deep audit for duplicate code in ingestion and integration, using first-principles thinking and reliability theory.

## Observations

### Provider Creation Duplication Analysis

I performed a comprehensive grep search across the codebase for provider creation patterns. Here are the findings:

#### Location 1: chat.rs (Non-Streaming Handler)
**File**: [chat.rs](../../../edgequake/crates/edgequake-api/src/handlers/chat.rs#L370-L460)

```rust
// Lines 385-460: Provider resolution with 3-level priority
let (llm_override, used_provider, used_model) = if let Some(ref provider_id) = request.provider {
    // ... 70+ lines of provider resolution logic
    match ProviderFactory::create_llm_provider(&provider_name, &model_name) {
        Ok(llm) => (Some(llm), Some(provider_name), Some(model_name)),
        Err(e) => return Err(ApiError::BadRequest(...))
    }
}
```

#### Location 2: chat.rs (Streaming Handler)  
**File**: [chat.rs](../../../edgequake/crates/edgequake-api/src/handlers/chat.rs#L850-L940)

```rust
// Lines 850-940: IDENTICAL logic with different error handling
let (llm_override, used_provider, used_model) = if let Some(ref provider_id) = request_provider {
    // ... 70+ lines of SAME provider resolution logic
    match ProviderFactory::create_llm_provider(&provider_name, &model_name) {
        Ok(llm) => (Some(llm), Some(provider_name), Some(model_name)),
        Err(e) => {
            tx.send(ChatStreamEvent::Error { message: error_msg, ... }).await;
            return; // Different error path!
        }
    }
}
```

#### Location 3: processor.rs (Document Ingestion)
**File**: [processor.rs](../../../edgequake/crates/edgequake-api/src/processor.rs#L220-L280)

```rust
// Uses DIFFERENT method with safety limits!
let llm_provider_result = ProviderFactory::create_safe_llm_provider(&ws.llm_provider, &ws.llm_model);
let embedding_provider_result = ProviderFactory::create_safe_embedding_provider(...);
```

#### Location 4: query.rs (Embedding Provider)
**File**: [query.rs](../../../edgequake/crates/edgequake-api/src/handlers/query.rs#L480-L550)

```rust
// Creates embedding provider with different error wrapping
let provider = ProviderFactory::create_embedding_provider(...)
    .map_err(|e| {
        if error_str.contains("OPENAI_API_KEY") {
            ApiError::ConfigError(...)  // Special case for API key errors
        } else {
            ApiError::Internal(...)
        }
    })?;
```

#### Location 5: state.rs (State Initialization)
**File**: [state.rs](../../../edgequake/crates/edgequake-api/src/state.rs#L920-L940)

```rust
// Uses safe provider creation
ProviderFactory::create_safe_llm_provider(&ws.llm_provider, &ws.llm_model);
```

### Critical Inconsistencies Found

| Aspect | chat.rs | processor.rs | query.rs |
|--------|---------|--------------|----------|
| Method Used | `create_llm_provider` | `create_safe_llm_provider` | `create_embedding_provider` |
| Safety Limits | ❌ No | ✅ Yes | ❌ No |
| Error Handling | Return/Send | Log + Fallback | Map to ApiError |
| API Key Detection | ❌ No | ❌ No | ✅ Yes |

### Reliability Risk Assessment

1. **Single Point Failure**: Changes to one location don't propagate to others
2. **Safety Limit Gap**: Chat queries can hang indefinitely (no timeout)
3. **Error Message Inconsistency**: Users get different error messages for same underlying issue
4. **Testing Difficulty**: Must test 5 separate code paths for same logic

### Lines of Duplicated Code

- chat.rs non-streaming: ~80 lines
- chat.rs streaming: ~80 lines (copy)
- **Total waste**: ~80 lines of duplicated logic

### Tenant ID Handling Observations

From grep search for `workspace.tenant_id`:

```
edgequake-api/src/handlers/chat.rs:371: workspace.tenant_id.to_string()
edgequake-core/src/workspace_service_impl.rs:389: workspace.tenant_id
edgequake-core/src/workspace_service.rs:350: workspace.tenant_id
```

The OODA-231 fix we just applied (commit 8ab2aad) fixed one instance, but there may be more.

## Metrics

- **Provider creation call sites**: 18 (from grep)
- **Distinct patterns identified**: 5
- **Duplicated logic blocks**: 2 major (chat.rs)
- **Safety limit inconsistencies**: 2 (chat vs processor)

## Next Step

Orient: Analyze the root cause of this duplication and design a unified solution.
