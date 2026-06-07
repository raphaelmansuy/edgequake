# Embedding Batch Size Limits

> **TL;DR**: Every embedding provider has a hard limit on the number of input strings
> per API request. These limits **must** be declared in `max_batch_size()` so that
> calling code can split batches correctly without hitting HTTP 400 errors.

---

## Why batch size limits exist

LLM embedding APIs process inputs in parallel on GPU/TPU clusters. Accepting an
unbounded batch would:

1. Exhaust memory on the serving node (OOM kill).
2. Delay the response beyond reasonable client timeouts.
3. Allow one tenant to monopolise shared inference capacity.

Providers therefore enforce a **maximum input count** per request, completely independent
of the per-input token count (which is a separate limit).

---

## Per-provider limits (as of 2026-05)

| Provider | Max inputs/request | Error code | HTTP status | Source |
|---|---|---|---|---|
| **Mistral** (`mistral-embed`) | **512** | 3210 | 400 | Empirical (error code documented in source) |
| **OpenAI** (`text-embedding-*`) | 2048 (v2), 2048 (v3) | — | 400 | OpenAI docs |
| **Anthropic** | N/A (no native embedding API) | — | — | — |
| **Ollama** | Unlimited (single input at a time) | — | — | Ollama docs |
| **LM Studio** | OpenAI-compatible; inherits OpenAI limits | — | — | — |
| **Gemini** | 100 per request (`batchEmbedContents`) | — | 400 | Google docs |
| **NVIDIA NIM** | 50 per request | — | 400 | NVIDIA docs |

> ⚠️ These limits can change without notice. If a provider starts returning
> "too many inputs" errors, consult their current docs and update the constant.

---

## How batch splitting works in edgequake-pipeline

`embed_with_token_budget()` in `edgequake-pipeline/src/pipeline/helpers.rs` splits
inputs using **two independent flush triggers**:

```
flush IF:
  count_overflow:  current_batch_count >= provider.max_batch_size()
  OR
  token_overflow:  estimated_tokens + next_input_tokens > token_budget_per_batch
```

This dual-trigger approach handles:
- Providers with small input limits (count trigger fires first).
- Providers with large input limits but small token budgets (token trigger fires first).
- Providers like Ollama that process one item at a time (count trigger fires at 1 if
  `max_batch_size()` returns 1, but Ollama's limit is effectively infinite so the token
  trigger governs).

### Token budget calculation

```rust
// Per-batch token budget = total_max_tokens * 0.85  (15% safety headroom)
let token_budget = provider.max_tokens() * 85 / 100;
```

The 15% headroom accounts for:
- Tokenisation variance (our token estimator uses word-based heuristics, not the
  provider's actual BPE tokeniser).
- Metadata overhead added by some providers (model name, encoding format).

---

## Edge cases

### EC-BATCH-01: Single input exceeds token budget

A text longer than `max_tokens` cannot be embedded by the provider.

**Behaviour**: The item is placed in its own sub-batch and sent to the API.
The API will reject it with a token-too-long error.

**Why not truncate?** Truncation silently changes the semantic meaning. The correct
fix is to chunk text before embedding, not to silently truncate.

**Status**: Propagated as `EmbeddingError` to the pipeline, which marks the document
chunk as failed.

### EC-BATCH-02: Empty input slice

`embed(&[])` returns `Ok(vec![])` immediately without making an API call.

All providers must uphold this contract.

### EC-BATCH-03: `max_tokens() == 0`

If a provider returns 0 from `max_tokens()`, `embed_with_token_budget` skips the
token-budget flush path and falls back to the count-only path. This prevents a
division-by-zero and handles providers that do not declare a token limit.

### EC-BATCH-04: `max_batch_size()` default (2048) is wrong for Mistral

**Fixed in 0.6.15**: `MistralProvider` now overrides `max_batch_size()` to return
512 (`MISTRAL_EMBED_MAX_BATCH_SIZE`).

Regression test:
```rust
assert_eq!(EmbeddingProvider::max_batch_size(&p), 512,
  "max_batch_size must report Mistral's hard input limit");
```

### EC-BATCH-05: Operator override via env var

`EDGEQUAKE_EMBEDDING_BATCH_SIZE` allows reducing the batch size below the provider's
hard limit (e.g. to reduce latency or per-request cost). The provider implementation
**always clamps** to the hard limit:

```rust
fn max_batch_size(&self) -> usize {
    let env_val = std::env::var("EDGEQUAKE_EMBEDDING_BATCH_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(MISTRAL_EMBED_MAX_BATCH_SIZE);
    env_val.min(MISTRAL_EMBED_MAX_BATCH_SIZE)  // hard ceiling
}
```

This means the operator can reduce but never exceed the provider's hard limit.

### EC-BATCH-06: Safety wrapper vs. provider limit

`SafetyLimitedEmbeddingProviderWrapper` in `edgequake-api` applies a default cap of
512 as an operator safety rail. Now that `MistralProvider::max_batch_size()` correctly
returns 512, the effective limit is:

```
effective = min(provider.max_batch_size(), safety_wrapper.config.max_embed_batch_size)
          = min(512, 512)
          = 512   ✓
```

The safety wrapper remains valuable for providers whose hard limits are unknown or
undeclared, and for future providers that might not override `max_batch_size()`.

### EC-BATCH-07: Retry with smaller batch on 400

Currently, embedding errors are propagated immediately without retry. A future
improvement could catch error code 3210 specifically and retry with a halved batch
size. This is deferred because:
- With correct `max_batch_size()` declarations, 3210 should never occur.
- Automatic retry with smaller batches can mask misconfigured providers.
- The pipeline already has document-level retry logic.
