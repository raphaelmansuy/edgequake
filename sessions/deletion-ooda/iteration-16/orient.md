# OODA-16 Orient: Ollama E2E Testing Analysis

## Available Resources

### Ollama Status: ✅ RUNNING
Endpoint: http://localhost:11434

### Available Models (from local Ollama)
| Model | Type | Size | Notes |
|-------|------|------|-------|
| `gemma3:latest` | LLM | 4.3B Q4_K_M | Recommended for testing |
| `gemma3:12b` | LLM | 12.2B Q4_K_M | Higher quality, slower |
| `nomic-embed-text:latest` | Embedding | 137M F16 | 768 dimensions |
| `embeddinggemma:latest` | Embedding | 307M BF16 | Alternative |

## Architecture Analysis

### Current Provider Factory Flow

```
┌─────────────────────────────────────────────────────────┐
│                    Provider Factory                       │
├─────────────────────────────────────────────────────────┤
│  1. Check PROVIDER env var (explicit override)            │
│  2. Check OLLAMA_HOST → OllamaProvider                    │
│  3. Check LMSTUDIO_HOST → LMStudioProvider                │
│  4. Check OPENAI_API_KEY → OpenAIProvider                 │
│  5. Fallback → MockProvider                               │
└─────────────────────────────────────────────────────────┘
```

### Test State Creation

```rust
// Current: AppState::test_state() uses mock provider
impl AppState {
    pub fn test_state() -> Self {
        // Uses MockProvider internally
    }
}
```

### Required: Ollama Test State

Need a function like:
```rust
pub async fn ollama_test_state() -> Result<Self, Error> {
    // 1. Check Ollama availability
    // 2. Create OllamaProvider for LLM
    // 3. Create OllamaProvider for embeddings
    // 4. Build AppState with real providers
}
```

## Test Strategy

### Option A: Conditional Test Execution (RECOMMENDED)
```rust
#[tokio::test]
#[ignore = "Requires Ollama running locally"]
async fn test_ollama_document_lifecycle() {
    if !is_ollama_available().await {
        eprintln!("⚠️ Skipping: Ollama not available");
        return;
    }
    // ... test logic
}
```

### Option B: Feature Flag
```rust
#[cfg(feature = "ollama-e2e")]
#[tokio::test]
async fn test_ollama_document_lifecycle() {
    // ...
}
```

### Preference: Option A
- More flexible for local development
- Can run with `cargo test -- --ignored`
- No Cargo.toml changes needed

## Test Cases Required

### 1. Document Lifecycle with Ollama

```
Upload → Wait for processing → Query → Delete → Verify cleanup
```

### 2. Query Mode Verification

| Mode | Description |
|------|-------------|
| `llm_only` | Pure LLM response, no embeddings |
| `embedding_only` | Vector similarity, no LLM |
| `hybrid` | Combined vector + graph + LLM |

### 3. Entity Extraction Quality

Mock provider returns simple entities. Ollama should extract:
- Named entities (people, organizations)
- Relationships between entities
- Entity types

## Implementation Plan

1. Create `e2e_ollama_integration.rs` test file
2. Add helper function `is_ollama_available()`
3. Create `create_ollama_app_state()` function
4. Implement 5 core test cases:
   - `test_ollama_document_upload`
   - `test_ollama_entity_extraction`
   - `test_ollama_query_llm_only`
   - `test_ollama_query_embedding_only`
   - `test_ollama_query_hybrid`
   - `test_ollama_document_deletion`

## Risk Analysis

| Risk | Mitigation |
|------|------------|
| Ollama slow response | Set generous timeouts (30s) |
| Model not pulled | Check model availability before tests |
| Port conflict | Use configurable OLLAMA_HOST |
| CI failures | Mark tests with #[ignore] |
