# OODA Iteration 122: Act

## Date: 2026-01-14

## Changes Made

### 1. Extended QueryStats with Token & Model Info

**File**: [query_types.rs](../../../../edgequake/crates/edgequake-api/src/handlers/query_types.rs#L163-L205)

Added 4 new optional fields to QueryStats:

```rust
/// Number of tokens generated in the response.
#[serde(skip_serializing_if = "Option::is_none")]
pub tokens_used: Option<usize>,

/// Tokens per second generation speed (calculated as tokens_used / generation_time_ms * 1000).
#[serde(skip_serializing_if = "Option::is_none")]
pub tokens_per_second: Option<f32>,

/// LLM provider used for generation (e.g., "ollama", "openai", "lmstudio").
#[serde(skip_serializing_if = "Option::is_none")]
pub llm_provider: Option<String>,

/// LLM model name used for generation (e.g., "gemma3:12b", "gpt-4o-mini").
#[serde(skip_serializing_if = "Option::is_none")]
pub llm_model: Option<String>,
```

### 2. Added get_workspace_llm_info Helper

**File**: [query.rs](../../../../edgequake/crates/edgequake-api/src/handlers/query.rs#L519-L578)

New function to retrieve workspace LLM configuration for lineage tracking:

```rust
async fn get_workspace_llm_info(
    state: &AppState,
    workspace_id: Option<&str>,
) -> (Option<String>, Option<String>)
```

### 3. Populated New Fields in Query Handler

**File**: [query.rs](../../../../edgequake/crates/edgequake-api/src/handlers/query.rs#L323-L365)

Query response now includes:
- tokens_per_second calculated as `generated_tokens / generation_time_ms * 1000`
- llm_provider from workspace config or defaults
- llm_model from workspace config or defaults

### 4. Updated Chat Handler for Consistency

**File**: [chat.rs](../../../../edgequake/crates/edgequake-api/src/handlers/chat.rs#L510-L550)

Chat completion response now also includes all token metrics and model lineage in stats.

### 5. Updated Tests

**File**: [query_types.rs](../../../../edgequake/crates/edgequake-api/src/handlers/query_types.rs#L306-L348)

Updated test cases to include new fields and verify serialization.

## Verification

### Build Check

```bash
cargo check --package edgequake-api
# ✅ Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.24s
```

### Tests

```bash
cargo test --package edgequake-api
# ✅ 30 passed; 0 failed; 0 ignored
```

## API Response Example

After this change, query responses will include:

```json
{
  "answer": "Machine learning is...",
  "mode": "hybrid",
  "stats": {
    "embedding_time_ms": 45,
    "retrieval_time_ms": 120,
    "generation_time_ms": 2500,
    "total_time_ms": 2700,
    "sources_retrieved": 5,
    "tokens_used": 124,
    "tokens_per_second": 49.6,
    "llm_provider": "ollama",
    "llm_model": "gemma3:12b"
  },
  "sources": [...]
}
```

## SPEC-032 Requirements Addressed

| Item | Requirement | Status |
|------|-------------|--------|
| 18 | Display tokens per second | ✅ Implemented |
| 22 | Display model used after tokens/second | ✅ Implemented |

## Next Steps

- Update WebUI to display "58.5/s • ollama/gemma3:12b" format
- Commit changes
- Continue with Item 24 (rebuild embeddings verification)
