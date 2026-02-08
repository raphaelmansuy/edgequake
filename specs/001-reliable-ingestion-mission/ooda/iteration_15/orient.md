# OODA-15: Orient - Price/Performance Configuration Analysis

**Date**: 2026-02-08
**Mission**: Reliable Document Ingestion Pipeline
**Focus**: Analyze optimal model configuration for EdgeQuake

---

## 1. Gap Analysis

### Current State vs Optimal State

| Aspect           | Current                | Optimal                | Gap          |
| ---------------- | ---------------------- | ---------------------- | ------------ |
| LLM Model        | gpt-5-nano             | gpt-5-nano             | ✅ None      |
| Embedding Model  | text-embedding-3-small | text-embedding-3-small | ✅ None      |
| Embedding Dims   | 1536                   | 1536                   | ✅ None      |
| JSON Truncation  | Possible issue         | None                   | ⚠️ Risk      |
| Default Provider | ollama (code)          | configurable           | ⚠️ Confusing |

---

## 2. Model Selection Decision Tree

```
┌─────────────────────────────────────────────────────────────────┐
│                    MODEL SELECTION CRITERIA                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Q1: Is cost the primary concern?                               │
│      │                                                          │
│      ├─ YES ─→ gpt-5-nano ($0.05/$0.40)                        │
│      │         ├─ Pros: 25x cheaper than gpt-5                  │
│      │         └─ Cons: May truncate long JSON                  │
│      │                                                          │
│      └─ NO ─→ Q2: Is JSON reliability critical?                │
│               │                                                 │
│               ├─ YES ─→ gpt-4.1-nano ($0.10/$0.40)             │
│               │         ├─ No reasoning tokens                  │
│               │         └─ Full JSON output control             │
│               │                                                 │
│               └─ NO ─→ gpt-5-mini ($0.25/$2.00)                │
│                        └─ Better reasoning, more expensive      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Risk Analysis

### gpt-5-nano Reasoning Token Issue

**Problem**: gpt-5-nano uses internal reasoning tokens (8192 budget)

- Reasoning tokens count toward output limit
- Complex entity extraction may exhaust budget
- JSON response gets truncated

**Impact**:

- Medium-High for documents with many entities (>30)
- Low for simple documents (<10 entities)

**Mitigation Options**:

| Option                                 | Effort | Impact | Recommendation |
| -------------------------------------- | ------ | ------ | -------------- |
| A. Switch to gpt-4.1-nano              | Low    | High   | ⭐ Best option |
| B. Reduce entity extraction batch size | Medium | Medium | Workaround     |
| C. Add JSON validation & retry         | Medium | High   | Complementary  |
| D. Increase max_tokens                 | Low    | Low    | May not help   |

---

## 4. gpt-4.1-nano vs gpt-5-nano Comparison

| Aspect           | gpt-5-nano     | gpt-4.1-nano |
| ---------------- | -------------- | ------------ |
| Input Price      | $0.05/1M       | $0.10/1M     |
| Output Price     | $0.40/1M       | $0.40/1M     |
| Reasoning Tokens | Yes (internal) | No           |
| JSON Reliability | Medium         | High         |
| Speed            | Fast           | Fast         |
| Quality          | Good           | Good         |

**Conclusion**: gpt-4.1-nano is **2x more expensive for input** but **more reliable for JSON**.

For a 100-page PDF:

- gpt-5-nano: ~$0.024
- gpt-4.1-nano: ~$0.029

Delta: **$0.005 per document** (20% more) for better reliability.

---

## 5. Embedding Model Analysis

### text-embedding-3-small (Current)

| Metric         | Value           |
| -------------- | --------------- |
| Price          | $0.02/1M tokens |
| Dimensions     | 1536            |
| Quality        | Good for RAG    |
| Context Length | 8191 tokens     |

### text-embedding-3-large (Alternative)

| Metric         | Value                       |
| -------------- | --------------------------- |
| Price          | $0.13/1M tokens (6.5x more) |
| Dimensions     | 3072                        |
| Quality        | Better similarity matching  |
| Context Length | 8191 tokens                 |

**Decision**: Keep `text-embedding-3-small` - quality is sufficient for RAG.

---

## 6. Operational Considerations

### A. Hybrid Mode (Recommended for Development)

```
┌─────────────────────────────────────────────────────────────────┐
│                     HYBRID MODE SETUP                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  LLM Provider:       OpenAI (gpt-5-nano or gpt-4.1-nano)        │
│  Embedding Provider: Ollama (nomic-embed-text) FREE             │
│                                                                  │
│  Benefits:                                                       │
│  - Quality entity extraction from OpenAI                        │
│  - Zero cost embeddings from Ollama                             │
│  - Reduces OpenAI cost by 40%+                                  │
│                                                                  │
│  Configuration:                                                  │
│  EDGEQUAKE_LLM_PROVIDER=openai                                  │
│  EDGEQUAKE_EMBEDDING_PROVIDER=ollama                            │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### B. Full OpenAI Mode (Production)

Best for consistency and quality:

- LLM: gpt-4.1-nano (reliable JSON)
- Embedding: text-embedding-3-small (1536 dims)
- Total cost: ~$0.03 per 100-page PDF

---

## 7. Documentation Gaps

| Missing Doc              | Priority | Impact            |
| ------------------------ | -------- | ----------------- |
| Model selection guide    | High     | User confusion    |
| Price comparison table   | High     | Cost optimization |
| Hybrid mode setup        | Medium   | Dev efficiency    |
| JSON truncation handling | High     | Reliability       |

---

## 8. First Principles Analysis

**Question**: What is the minimal configuration for reliable RAG ingestion?

**Answer**:

1. **LLM for entity extraction**: Must produce valid JSON reliably
   - gpt-4.1-nano preferred over gpt-5-nano for JSON reliability
2. **Embedding for similarity search**: Must have consistent dimensions
   - text-embedding-3-small (1536 dims) is cost-optimal
3. **Provider consistency**: Match workspace settings with query-time providers
   - Dimension mismatch = query failure

---

## 9. Recommended Configuration

### Development (Minimal Cost)

```toml
[llm]
provider = "ollama"
model = "gemma3:latest"

[embedding]
provider = "ollama"
model = "nomic-embed-text"
dimension = 768
```

### Production (Optimal Quality/Cost)

```toml
[llm]
provider = "openai"
model = "gpt-4.1-nano"  # Changed from gpt-5-nano for JSON reliability

[embedding]
provider = "openai"
model = "text-embedding-3-small"
dimension = 1536
```

---

## 10. Decision Points for OODA-15

| Priority | Decision                                   | Rationale        |
| -------- | ------------------------------------------ | ---------------- |
| P0       | Test gpt-4.1-nano for entity extraction    | JSON reliability |
| P1       | Update mission doc with pricing table      | User guidance    |
| P2       | Add JSON validation to extraction pipeline | Reliability      |
| P3       | Document hybrid mode in AGENTS.md          | Dev efficiency   |

---

## Next Steps (Decide)

1. **Test gpt-4.1-nano** vs gpt-5-nano for entity extraction quality
2. **Update workspace** to use gpt-4.1-nano if tests pass
3. **E2E test** with Playwright to verify full pipeline
4. **Update documentation** with model selection guide
