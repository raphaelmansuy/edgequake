# OODA Loop Iteration 15 - Act Phase

**Date:** 2026-01-11  
**Focus:** Provider Selection at Query Time  
**Status:** ✅ COMPLETE

## Changes Made

### 1. Extended QueryRequest with LLM Override Fields

**File:** [edgequake-query/src/engine.rs](../../edgequake/crates/edgequake-query/src/engine.rs)

Added new fields to `QueryRequest` struct:

```rust
/// Override: LLM provider to use for answer generation.
/// Format: provider name (e.g., "ollama", "openai", "lmstudio").
/// If not provided, uses the server default.
/// @implements SPEC-032: Provider selection at query time
#[serde(default)]
pub llm_provider: Option<String>,

/// Override: LLM model to use for answer generation.
/// If not provided, uses the provider's default model.
/// @implements SPEC-032: Model selection at query time
#[serde(default)]
pub llm_model: Option<String>,
```

Added builder methods:

```rust
pub fn with_llm_provider(mut self, provider: impl Into<String>) -> Self
pub fn with_llm_model(mut self, model: impl Into<String>) -> Self
pub fn with_llm_full_id(mut self, full_id: impl AsRef<str>) -> Self
```

The `with_llm_full_id()` method parses the "provider/model" format used by the WebUI.

### 2. Updated Chat Completion Handler

**File:** [edgequake-api/src/handlers/chat.rs](../../edgequake/crates/edgequake-api/src/handlers/chat.rs)

Added provider override in non-streaming handler:

```rust
// SPEC-032: Apply provider/model override from request
// Format: "provider/model" (e.g., "ollama/gemma3:12b") or just "provider"
if let Some(ref provider) = request.provider {
    if !provider.is_empty() {
        engine_request = engine_request.with_llm_full_id(provider);
        debug!(provider = %provider, "Using LLM provider override from request");
    }
}
```

### 3. Updated Streaming Chat Handler

**File:** [edgequake-api/src/handlers/chat.rs](../../edgequake/crates/edgequake-api/src/handlers/chat.rs)

Added provider cloning for async task:

```rust
// SPEC-032: Clone provider for async task
let request_provider = request.provider.clone();
```

Added provider override in streaming handler:

```rust
// SPEC-032: Apply provider/model override from request (streaming handler)
if let Some(ref provider) = request_provider {
    if !provider.is_empty() {
        engine_request = engine_request.with_llm_full_id(provider);
        debug!(provider = %provider, "Using LLM provider override in streaming");
    }
}
```

## Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│ WebUI Query Interface                                            │
│                                                                  │
│  ProviderModelSelector: "ollama/gemma3:12b"                     │
│           │                                                      │
│           ▼                                                      │
│  ChatCompletionRequest { provider: "ollama/gemma3:12b" }        │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│ chat.rs Handler                                                  │
│                                                                  │
│  engine_request.with_llm_full_id("ollama/gemma3:12b")           │
│           │                                                      │
│           ▼                                                      │
│  QueryRequest {                                                  │
│    llm_provider: Some("ollama"),                                │
│    llm_model: Some("gemma3:12b"),                               │
│  }                                                               │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│ SOTAQueryEngine                                                  │
│                                                                  │
│  📌 NOTE: Engine uses default LLM for now                       │
│     Next iteration: wire provider to LLM generation step        │
└─────────────────────────────────────────────────────────────────┘
```

## Test Results

```
cargo test --package edgequake-query --package edgequake-api
   Compiling edgequake-query v0.1.0
   Compiling edgequake-api v0.1.0
    Finished dev profile

test result: ok. 34 passed; 0 failed
test result: ok. 4 passed; 0 failed
test result: ok. 7 passed; 0 failed
```

## Files Changed

| File                                 | Lines Added | Change                                  |
| ------------------------------------ | ----------- | --------------------------------------- |
| `edgequake-query/src/engine.rs`      | +35         | QueryRequest fields and builder methods |
| `edgequake-api/src/handlers/chat.rs` | +15         | Provider override in both handlers      |

## Remaining Work

The `QueryRequest` now carries the provider override, but the `SOTAQueryEngine` doesn't yet use these fields. This will be addressed in the next iteration.

**Next iteration (OODA-16):** Wire `llm_provider` and `llm_model` from QueryRequest into the LLM generation step of SOTAQueryEngine.

## Acceptance Criteria

- [x] QueryRequest has `llm_provider` and `llm_model` fields
- [x] Builder methods allow setting provider/model
- [x] `with_llm_full_id()` parses "provider/model" format
- [x] Non-streaming handler passes provider from request
- [x] Streaming handler passes provider from request
- [x] All tests pass
- [x] Build succeeds
