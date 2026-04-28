# Ingestion Reliability — Mitigation Strategies and Resilience

> Companion to: `root_cause_analysis.md`, `fix_specification.md`  
> Version: v0.11.2

---

## Overview

This document catalogs all mitigation strategies considered for the two production failure
modes found in v0.11.1, records decisions, and outlines a defence-in-depth posture for future
reliability work.

---

## Failure Mode 1 — JSON-EOF on Dense Extraction

### Root Cause (summary)
`SafetyLimitedProviderWrapper` silently clamped `max_tokens` to 8 192 on every extraction
attempt, including the retry loop. Dense seismology chunks needed > 8 192 output tokens to
enumerate all entities; all three retries hit the same wall and the final JSON was always
truncated at or near token 8 192.

### Applied Mitigations

| Layer           | Strategy                                          | Status    |
| --------------- | ------------------------------------------------- | --------- |
| **Fix A**       | Raise `DEFAULT_MAX_TOKENS` 8 192 → 16 384         | ✅ v0.11.2 |
| **Fix B**       | Partial JSON recovery via suffix-append heuristic | ✅ v0.11.2 |
| **WHY comment** | Document the safety-limit defaults in source      | ✅ v0.11.2 |

### Additional Strategies Considered

#### 1. Adaptive Token Budget Escalation on Retry (CONSIDERED — DEFERRED)

On each retry, double the `max_tokens` up to `ABSOLUTE_MAX_TOKENS`.

```
retry 0 → max_tokens = 16 384
retry 1 → max_tokens = 32 768
retry 2 → max_tokens = 65 536
```

**Why deferred**: Adds stateful retry coordination across the safety wrapper and the extractor.
The current fixes (Fix A + Fix B) cover all known production cases. Escalation adds complexity
with no measured benefit at this time.

#### 2. Chunk Pre-Splitting (CONSIDERED — DEFERRED)

Before sending to the extractor, split chunks larger than `MAX_CHUNK_TOKENS` into
sub-chunks and merge entity lists. Eliminates the token-cap problem entirely.

**Why deferred**: Requires coordination with the chunking pipeline, which is a separate
concern. The current fix does not change ingestion throughput or latency. Chunk pre-splitting
is on the roadmap for v0.12 as part of a broader context-window optimization pass.

#### 3. Streaming JSON Parser (CONSIDERED — REJECTED)

Use a streaming JSON library to emit partial entity objects as they are generated, so a
mid-stream EOF yields partial results with no recovery step.

**Why rejected**: Requires a different LLM call style (streaming tokens) and fundamental
changes to `LLMExtractor::complete`. Current architecture uses a single request/response round
trip. The `try_recover_truncated_json` heuristic achieves the same result without changing
the call pattern.

---

## Failure Mode 2 — Embedding 400 "Too Many Tokens Overall"

### Root Cause (summary)
`embed_batched` splits by item count (default 2 048) with no awareness of total token budget.
142 entity descriptions from a dense seismology paper exceeded Mistral's 8 192-token per-call
budget in a single sub-batch.

### Applied Mitigations

| Layer             | Strategy                                              | Status    |
| ----------------- | ----------------------------------------------------- | --------- |
| **Fix C**         | Token-aware sub-batching in `embed_with_token_budget` | ✅ v0.11.2 |
| **Safety factor** | 15% headroom via `EMBED_SAFETY_FACTOR = 0.85`         | ✅ v0.11.2 |

### Additional Strategies Considered

#### 4. Per-Provider Token-Count Configuration (DEFERRED)

Allow operators to set `EDGEQUAKE_EMBED_MAX_TOKENS` per provider to override the value
returned by `provider.max_tokens()`.

**Why deferred**: `EmbeddingProvider::max_tokens()` already returns the correct limit for
Mistral (8 192) and OpenAI (8 191). Env-var override adds surface area with no immediate need.

#### 5. Fallback to 512-Token Sub-batches on 400 Error (DEFERRED)

Detect HTTP 400 responses during embedding and retry the same batch with halved sub-batch
size.

**Why deferred**: Retry-on-error is a useful backstop but adds latency on the hot path. With
Fix C, the 400 should never occur for normal production inputs. Retry-on-error is worth
adding if edge cases emerge in production after v0.11.2.

#### 6. Token-Level Truncation Guard (ALREADY IMPLEMENTED)

`guard_for_embedding(text, max_chars)` truncates individual texts before embedding so no
single item can exceed the provider limit. This was already in `helpers.rs` before this fix.

**Relationship to Fix C**: The guard handles _per-text_ over-limit; Fix C handles _total
batch_ over-limit. Both are needed; neither alone is sufficient.

---

## Defence-in-Depth Model (v0.11.2)

```
Document ingested
│
├── Chunking
│   └── Chunk size limited by CHUNKER_MAX_TOKENS (upstream)
│
├── Extraction (per chunk)
│   ├── Safety wrapper: max_tokens ≤ ABSOLUTE_MAX_TOKENS (65 536)
│   ├── Extractor retry loop (3 attempts)
│   └── parse_response: try_recover_truncated_json on primary parse failure
│
└── Embedding (batch)
    ├── guard_for_embedding: per-text char truncation
    └── embed_with_token_budget: token-aware sub-batching
```

---

## Monitoring Recommendations

| Signal                                                    | Alert threshold  | Notes                                                        |
| --------------------------------------------------------- | ---------------- | ------------------------------------------------------------ |
| `WARN: JSON parse failed for chunk … attempting recovery` | > 5 / hour       | Indicates chunks regularly exceeding output budget           |
| `WARN: recovered truncated JSON`                          | > 5 / hour       | As above                                                     |
| HTTP 400 on embed endpoint                                | Any              | Should not occur after Fix C                                 |
| Extraction retry count per chunk                          | > 1 consistently | May indicate extractor prompt is generating oversized output |

---

## Open Questions

1. **Adaptive chunk pre-splitting**: Should dense PDFs be split into smaller chunks before
   extraction to keep entity lists below ~50 per chunk? Currently out of scope for v0.11.2.

2. **Token-counting accuracy**: `EMBED_CHARS_PER_TOKEN = 2.5` is a conservative heuristic.
   For production, integration with a real tokenizer (e.g., tiktoken-rs) would eliminate the
   safety factor entirely. Worth a spike in v0.12.

3. **Upstream `edgequake-llm` fix**: `embed_batched` in the external crate should ideally be
   token-aware by default. A PR to `edgequake-llm` to add optional token-budget splitting is
   on the backlog.
