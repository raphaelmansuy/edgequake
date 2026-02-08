# OODA-15: Observe - OpenAI Model Price/Performance Analysis

**Date**: 2026-02-08
**Mission**: Reliable Document Ingestion Pipeline
**Focus**: Find optimal OpenAI models for RAG ingestion (best price/performance)

---

## 1. OpenAI Pricing Research (2026-02)

### LLM Models - Prices per 1M tokens

| Model          | Input | Cached | Output | Notes                               |
| -------------- | ----- | ------ | ------ | ----------------------------------- |
| **gpt-5-nano** | $0.05 | $0.005 | $0.40  | ⭐ CHEAPEST - fastest inference     |
| gpt-4.1-nano   | $0.10 | $0.025 | $0.40  | Alternative, no reasoning tokens    |
| gpt-4o-mini    | $0.15 | $0.075 | $0.60  | Legacy, quota exceeded              |
| gpt-5-mini     | $0.25 | $0.025 | $2.00  | Better reasoning, 5x more expensive |
| gpt-4.1-mini   | $0.40 | $0.10  | $1.60  | Good quality/cost balance           |
| gpt-4.1        | $2.00 | $0.50  | $8.00  | High quality, expensive             |
| gpt-5          | $1.25 | $0.125 | $10.00 | Full reasoning model                |
| gpt-5.2        | $1.75 | $0.175 | $14.00 | Best overall quality                |

### Embedding Models - Prices per 1M tokens

| Model                      | Standard | Batch  | Dimensions    |
| -------------------------- | -------- | ------ | ------------- | ------------- |
| **text-embedding-3-small** | $0.02    | $0.01  | 1536          | ⭐ BEST VALUE |
| text-embedding-3-large     | $0.13    | $0.065 | 3072          |
| text-embedding-ada-002     | $0.10    | $0.05  | 1536 (legacy) |

---

## 2. Cost Analysis for Document Ingestion

### Scenario: 100-page PDF (~50,000 tokens)

| Configuration                  | LLM Cost                     | Embedding Cost | Total       |
| ------------------------------ | ---------------------------- | -------------- | ----------- |
| **gpt-5-nano + embed-3-small** | $0.0025 input + $0.02 output | $0.001         | **~$0.024** |
| gpt-5-mini + embed-3-small     | $0.0125 + $0.10              | $0.001         | ~$0.114     |
| gpt-4.1-mini + embed-3-small   | $0.02 + $0.08                | $0.001         | ~$0.101     |
| gpt-5 + embed-3-large          | $0.0625 + $0.50              | $0.0065        | ~$0.569     |

**Winner**: `gpt-5-nano` + `text-embedding-3-small` = **~$0.024 per 100-page PDF**

---

## 3. Current EdgeQuake Configuration

Checked workspace settings via API:

```json
{
  "llm_provider": "openai",
  "llm_model": "gpt-5-nano",
  "embedding_provider": "openai",
  "embedding_model": "text-embedding-3-small",
  "embedding_dimension": 1536
}
```

✅ Already using optimal models!

---

## 4. gpt-5-nano Characteristics

### Pros:

- 25x cheaper than gpt-5 for input tokens
- 25x cheaper than gpt-5 for output tokens
- Fast inference (~50ms per request)
- Good at structured output (JSON)
- Works with Chat Completions API

### Cons:

- Uses reasoning tokens internally (8192 token budget)
- May truncate long JSON responses
- Less capable for complex reasoning tasks

### Observed Issue (OODA-14):

- gpt-5-nano's reasoning mode can exhaust output token budget
- Long entity extraction responses may get truncated
- Need to monitor JSON completeness

---

## 5. Backend Configuration Code

Checked `multitenancy.rs`:

```rust
// Line 334
pub const DEFAULT_LLM_PROVIDER: &str = "ollama";
```

Workspace-specific settings override this default via database.

### Provider Selection Flow:

```
┌─────────────────────────────────────────────────────┐
│              Provider Selection                     │
├─────────────────────────────────────────────────────┤
│ 1. Check workspace DB settings                      │
│    └─> If set, use workspace llm_provider           │
│                                                     │
│ 2. Check environment variable                       │
│    └─> EDGEQUAKE_DEFAULT_LLM_PROVIDER              │
│                                                     │
│ 3. Use hardcoded default                            │
│    └─> "ollama" (from multitenancy.rs:334)         │
└─────────────────────────────────────────────────────┘
```

---

## 6. Embedding Dimension Consistency

**Critical**: Embedding dimensions must match across:

1. Workspace configuration (1536 for OpenAI)
2. Vector storage schema
3. Query-time embedding generation

Mismatch causes "different number of dimensions" errors.

---

## 7. Service Status Check

| Service    | Status     | Notes                        |
| ---------- | ---------- | ---------------------------- |
| Backend    | ✅ Running | localhost:8080               |
| Frontend   | ✅ Running | localhost:3000               |
| PostgreSQL | ✅ Running | edgequake-postgres container |
| Ollama     | ✅ Running | localhost:11434              |

---

## 8. Files to Investigate

| File                  | Purpose                      | Lines |
| --------------------- | ---------------------------- | ----- |
| `multitenancy.rs`     | Default provider constants   | 334   |
| `provider_factory.rs` | Provider instantiation       | TBD   |
| `openai.rs`           | OpenAI client implementation | TBD   |
| `workspace.rs`        | Workspace settings API       | TBD   |

---

## 9. Key Observations Summary

1. **gpt-5-nano is optimal** for RAG ingestion ($0.05/1M input, $0.40/1M output)
2. **text-embedding-3-small is optimal** for embeddings ($0.02/1M tokens, 1536 dims)
3. **Current workspace** already configured with optimal models
4. **Reasoning token issue** may truncate long JSON responses
5. **Dimension consistency** (1536) must be maintained

---

## Next Steps (Orient)

1. Analyze gpt-5-nano JSON truncation issue
2. Consider fallback to gpt-4.1-nano (no reasoning tokens)
3. Test parallel document ingestion
4. Verify E2E flow with Playwright
