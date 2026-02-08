# OODA Iteration 10: Observe

## Date: 2026-02-08

## Focus: OpenAI API Quota Exceeded - Model Configuration Audit

### Issue Report

User reported: "OpenAI API quota is exceeded" - need to ensure cheapest possible models are used for ingestion.

### OpenAI Pricing Research (2026-02)

#### LLM Models (Cheapest Options)

| Model          | Input (per 1M tokens) | Output (per 1M tokens) | Batch Price |
| -------------- | --------------------- | ---------------------- | ----------- |
| **gpt-5-nano** | $0.05                 | $0.40                  | $0.005      |
| gpt-4.1-nano   | $0.10                 | $0.40                  | $0.025      |
| gpt-4o-mini    | $0.15                 | $0.60                  | $0.075      |
| gpt-5-mini     | $0.25                 | $2.00                  | $0.025      |

**Winner: `gpt-5-nano`** - 3x cheaper than gpt-4o-mini ✅

#### Embedding Models (Cheapest Options)

| Model                      | Price (per 1M tokens) | Batch Price | Dimensions |
| -------------------------- | --------------------- | ----------- | ---------- |
| **text-embedding-3-small** | $0.02                 | $0.01       | 1536       |
| text-embedding-ada-002     | $0.10                 | $0.05       | 1536       |
| text-embedding-3-large     | $0.13                 | $0.065      | 3072       |

**Winner: `text-embedding-3-small`** - 5x cheaper than ada-002 ✅

### Current Configuration Audit

#### 1. edgequake-llm/src/model_config.rs

```rust
fn default_llm_model() -> String {
    // WHY: gpt-5-nano is the recommended default (2025-02).
    // gpt-4o-mini has quota issues and is being phased out.
    "gpt-5-nano".to_string()  // ✅ CORRECT
}

fn default_embedding_model() -> String {
    "text-embedding-3-small".to_string()  // ✅ CORRECT
}
```

#### 2. edgequake-llm/src/providers/openai.rs (lines 43-45)

```rust
Self {
    model: "gpt-5-nano".to_string(),  // ✅ CORRECT
    embedding_model: "text-embedding-3-small".to_string(),  // ✅ CORRECT
    ...
}
```

#### 3. edgequake-core/src/types/multitenancy.rs (lines 331-346)

```rust
// Defaults for Ollama (dev without API keys)
pub const DEFAULT_LLM_MODEL: &str = "gemma3:12b";
pub const DEFAULT_LLM_PROVIDER: &str = "ollama";
pub const DEFAULT_EMBEDDING_MODEL: &str = "embeddinggemma";
pub const DEFAULT_EMBEDDING_PROVIDER: &str = "ollama";
```

**Note**: These are Ollama defaults. OpenAI defaults are in model_config.rs.

#### 4. models.toml comment example shows outdated model (line 31):

```toml
//! llm_model = "gpt-4o-mini"  // ⚠️ OUTDATED COMMENT - should be gpt-5-nano
```

### Environment Variable Configuration

The system respects these env vars (highest priority):

- `EDGEQUAKE_DEFAULT_LLM_PROVIDER`
- `EDGEQUAKE_DEFAULT_LLM_MODEL`
- `EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER`
- `EDGEQUAKE_DEFAULT_EMBEDDING_MODEL`
- `EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION`

### Ollama Default Models

For local development without API keys:

- LLM: `gemma3:12b` (128K context, vision support)
- Embedding: `embeddinggemma` (768 dimensions)

### Key Findings

1. **OpenAI defaults are already optimal** ✅
   - LLM: `gpt-5-nano` (cheapest available)
   - Embedding: `text-embedding-3-small` (cheapest available)

2. **Documentation comment outdated** ⚠️
   - model_config.rs line 31 shows `gpt-4o-mini` in example

3. **Need to verify end-to-end ingestion** ⚠️
   - With OpenAI provider
   - With Ollama provider
   - Both must work seamlessly

4. **Quota exceeded could be user's API key limit** ⚠️
   - Not a code configuration issue
   - User may need to increase API quota in OpenAI dashboard

### Services Status Check

Need to verify:

- [ ] PostgreSQL database running
- [ ] Backend API healthy
- [ ] OpenAI API key valid
- [ ] Ollama server running with required models
