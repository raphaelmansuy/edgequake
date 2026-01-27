# OODA Loop Iteration 16 - Act Phase

**Date:** 2026-01-11  
**Focus:** Implement LLM Provider Override in Query Engine  
**Status:** ✅ COMPLETE

## Changes Made

### 1. Re-exported LLMProvider

**File:** [edgequake-query/src/lib.rs](../../edgequake/crates/edgequake-query/src/lib.rs)

```rust
pub use edgequake_llm::traits::LLMProvider;
```

### 2. Added generate_answer_with_provider

**File:** [edgequake-query/src/sota_engine.rs](../../edgequake/crates/edgequake-query/src/sota_engine.rs)

Internal method that accepts an optional LLM provider override:

```rust
async fn generate_answer_with_provider(
    &self,
    query: &str,
    context: &QueryContext,
    llm_override: Option<&Arc<dyn crate::LLMProvider>>,
) -> Result<(String, usize)> {
    // ...
    let response = if let Some(provider) = llm_override {
        provider.complete(&prompt).await?
    } else {
        self.llm_provider.complete(&prompt).await?
    };
    // ...
}
```

### 3. Added query_with_llm_provider

**File:** [edgequake-query/src/sota_engine.rs](../../edgequake/crates/edgequake-query/src/sota_engine.rs)

Public method (~100 lines) that:

- Runs full SOTA pipeline (keywords, mode, embeddings, retrieval, reranking, truncation)
- Uses the provided LLM for answer generation

```rust
pub async fn query_with_llm_provider(
    &self,
    request: crate::engine::QueryRequest,
    llm_provider: std::sync::Arc<dyn crate::LLMProvider>,
) -> Result<crate::engine::QueryResponse>
```

### 4. Updated chat.rs Handler

**File:** [edgequake-api/src/handlers/chat.rs](../../edgequake/crates/edgequake-api/src/handlers/chat.rs)

Added import:

```rust
use edgequake_llm::ProviderFactory;
```

Updated query execution to create and use LLM override:

```rust
// Parse "provider/model" format
let llm_override = if let Some(ref provider_full_id) = request.provider {
    if !provider_full_id.is_empty() {
        let (provider_name, model_name) = if let Some((p, m)) = provider_full_id.split_once('/') {
            (p.to_string(), m.to_string())
        } else {
            (provider_full_id.clone(), "default".to_string())
        };

        match ProviderFactory::create_llm_provider(&provider_name, &model_name) {
            Ok(llm) => {
                debug!(provider = %provider_name, model = %model_name, "Created LLM provider override");
                Some(llm)
            }
            Err(e) => {
                warn!(provider = %provider_name, error = %e, "Failed to create LLM provider");
                None
            }
        }
    } else {
        None
    }
} else {
    None
};

// Execute query with or without LLM override
let result = if let Some(ref llm) = llm_override {
    state.sota_engine.query_with_llm_provider(engine_request, llm.clone()).await?
} else {
    state.sota_engine.query(engine_request).await?
};
```

## Data Flow (Complete)

```
┌─────────────────────────────────────────────────────────────────┐
│ WebUI Query Interface                                            │
│                                                                  │
│  ProviderModelSelector: "ollama/gemma3:12b"                     │
│           │                                                      │
│           ▼                                                      │
│  POST /api/v1/chat/completions                                  │
│  { "message": "...", "provider": "ollama/gemma3:12b" }          │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│ chat.rs Handler                                                  │
│                                                                  │
│  1. Parse "ollama/gemma3:12b" → provider="ollama", model="..."  │
│  2. ProviderFactory::create_llm_provider("ollama", "gemma3:12b")│
│  3. query_with_llm_provider(request, ollama_llm)                │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│ SOTAQueryEngine                                                  │
│                                                                  │
│  query_with_llm_provider()                                      │
│    ├── Keywords extraction (uses default LLM)                  │
│    ├── Mode selection                                           │
│    ├── Embeddings (uses default embedding provider)             │
│    ├── Retrieval (mode-specific)                                │
│    ├── Reranking                                                │
│    ├── Truncation                                               │
│    └── generate_answer_with_provider(query, ctx, Some(ollama))  │
│              │                                                   │
│              ▼                                                   │
│        ollama_llm.complete(&prompt)                             │
│              │                                                   │
│              ▼                                                   │
│        Response generated by Ollama gemma3:12b                  │
└─────────────────────────────────────────────────────────────────┘
```

## Test Results

```
cargo build --package edgequake-query --package edgequake-api
   Compiling edgequake-query v0.1.0
   Compiling edgequake-api v0.1.0
    Finished dev profile

cargo test --package edgequake-query --package edgequake-api
test result: ok. 34 passed; 0 failed
test result: ok. 4 passed; 0 failed
test result: ok. 7 passed; 0 failed
```

## Files Changed

| File                                 | Lines Added | Type   |
| ------------------------------------ | ----------- | ------ |
| `edgequake-query/src/lib.rs`         | +1          | Export |
| `edgequake-query/src/sota_engine.rs` | +130        | Engine |
| `edgequake-api/src/handlers/chat.rs` | +40         | API    |

## Remaining Work

1. **Streaming handler:** The streaming path (`query_stream_with_context`) doesn't yet support LLM override
2. **Integration test:** Add E2E test that verifies different providers generate different responses

## Acceptance Criteria

- [x] `generate_answer_with_provider` accepts optional LLM override
- [x] `query_with_llm_provider` public method added
- [x] `LLMProvider` re-exported from edgequake-query
- [x] chat.rs creates provider from request and uses new method
- [x] Fallback to default on provider creation failure
- [x] All tests pass
- [x] Build succeeds
