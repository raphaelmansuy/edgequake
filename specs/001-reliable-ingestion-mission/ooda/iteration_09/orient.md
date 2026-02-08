# OODA Iteration 09 - Orient

## Date: 2026-02-08

## First Principles Analysis

### Problem Statement

The EdgeQuake ingestion pipeline is failing because:

1. **Embedding context overflow** - Ollama's nomic-embed-text (2048 tokens) rejects inputs > context window
2. **Provider mismatch** - OPENAI_API_KEY set but system uses Ollama defaults
3. **No runtime provider selection** - Cannot easily switch providers for ingestion

### Root Cause Tree

```
PDF Upload Fails to Create KG
│
├── 1. Embedding API Error (400 Bad Request)
│   └── "input length exceeds context length"
│       ├── nomic-embed-text: 2048 token max
│       ├── Content being embedded > 2048 tokens
│       └── No truncation before embedding
│
├── 2. Provider Selection Issue
│   ├── DEFAULT_LLM_PROVIDER hardcoded to "ollama"
│   ├── Makefile detects OPENAI_API_KEY but backend ignores it
│   └── No env-based default override
│
└── 3. Pipeline Configuration
    ├── chunk_size: 1200 tokens (should be fine)
    └── But descriptions/summaries may exceed limit
```

### Solution Analysis

#### Option A: Fix Ollama Embedding Input Handling

**Approach:** Truncate text to model's context length before embedding

**Pros:**

- Works with any embedding model
- Defensive approach
- Doesn't require OpenAI

**Cons:**

- May lose information from truncation
- Need to implement per-model context limits

**Implementation:**

```rust
// In embedding provider
fn embed(&self, text: &str) -> Embedding {
    let truncated = truncate_to_tokens(text, self.context_length - 100);
    self.model.embed(&truncated)
}
```

#### Option B: Use OpenAI Embeddings

**Approach:** Configure workspace to use `text-embedding-3-small` (8191 tokens)

**Pros:**

- 4x larger context window
- Higher quality embeddings
- Matches production use case

**Cons:**

- Requires API key
- Incurs costs (minimal: $0.00002/1K tokens)
- Different embedding dimension (1536 vs 768)

**Implementation:**

1. Update workspace settings via API
2. Or change DEFAULT_EMBEDDING_PROVIDER to read from env

#### Option C: Make Defaults Environment-Configurable

**Approach:** Read provider defaults from environment variables

**Pros:**

- Most flexible
- No code changes for switching
- Works in CI/production

**Cons:**

- Requires restart to change
- More env vars to manage

**Implementation:**

```rust
pub fn default_llm_provider() -> &'static str {
    static PROVIDER: OnceCell<String> = OnceCell::new();
    PROVIDER.get_or_init(|| {
        std::env::var("EDGEQUAKE_DEFAULT_LLM_PROVIDER")
            .unwrap_or_else(|_| "ollama".to_string())
    })
}
```

### Recommended Approach

**Strategy: Option A + C**

1. **Immediate fix (Option A):** Add truncation to embedding providers
   - Prevents 400 errors regardless of input size
   - Safe fallback for any model

2. **Config fix (Option C):** Environment-based provider selection
   - Allows battle testing with OpenAI
   - Supports both providers in same deployment

### Risk Assessment

| Risk                         | Probability | Impact | Mitigation                              |
| ---------------------------- | ----------- | ------ | --------------------------------------- |
| Truncation loses information | Medium      | Low    | Log warnings, use larger context models |
| OpenAI API costs             | Low         | Low    | gpt-5-nano is very cheap                |
| Embedding dimension mismatch | High        | High   | Recreate vector table if switching      |
| Breaking existing tests      | Medium      | Medium | Run full test suite                     |

### Verification Strategy

1. After truncation fix: Upload a large PDF with Ollama
2. After provider fix: Upload same PDF with OpenAI
3. Compare entity counts and quality
4. Run E2E tests with both providers

## Decision Framework

**Must Fix:**

- [ ] Embedding context overflow (blocking all ingestion)
- [ ] Provider environment configuration (for battle testing)

**Should Fix:**

- [ ] Update workspace settings UI for provider selection
- [ ] Add health check for embedding model availability

**Nice to Have:**

- [ ] Per-document provider override
- [ ] Automatic model fallback chain
