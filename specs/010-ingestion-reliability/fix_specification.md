# Ingestion Reliability — Fix Specification

> **Fixes**: Failure Mode 1 (JSON EOF) + Failure Mode 2 (Embedding 400)  
> **Version**: v0.11.2  
> **Branch**: `fix/010-ingestion-reliability`

---

## Fix A — Raise Safety-Limit Token Cap

**File**: `edgequake/crates/edgequake-api/src/safety_limits.rs`

| Constant              | Before | After  | Rationale                                                |
| --------------------- | ------ | ------ | -------------------------------------------------------- |
| `DEFAULT_MAX_TOKENS`  | 8 192  | 16 384 | Matches the `max_tokens` LLM extractor already requests  |
| `ABSOLUTE_MAX_TOKENS` | 32 768 | 65 536 | Allows operator override via env var for very dense PDFs |

The `SafetyLimitedProviderWrapper` now passes `max_tokens = 16 384` to the underlying
provider, eliminating the silent clamping that truncated entity-list JSON.

---

## Fix B — Partial JSON Recovery in LLM Extractor

**File**: `edgequake/crates/edgequake-pipeline/src/extractor/llm.rs`

Added `LLMExtractor::try_recover_truncated_json(s)` — a deterministic, allocation-minimal
recovery function that appends a series of closing suffixes to a truncated JSON string and
returns the first suffix that produces a valid `serde_json::Value`.

### Suffix table (applied in order)

| Suffix  | Recovers truncation at                        |
| ------- | --------------------------------------------- |
| `""`    | Already-complete JSON (fast path)             |
| `}`     | Inside the root object only                   |
| `]}`    | Inside the `relationships` array              |
| `}]}`   | Inside a relationship object                  |
| `]}]}`  | Inside `entities` array after complete object |
| `}]}]}` | Inside an entity object                       |
| `"]}]}` | Inside a string field of an entity            |
| `"]}`   | Inside a string in relationships              |

`parse_response` now calls this function on primary parse failure and logs a `WARN` with
`chunk_id`. The returned value is used as if it were the primary parse result, so all
complete entity/relationship objects emitted before the cutoff are preserved.

### Invariants

- Returns `None` for truly unrecoverable input (garbage / non-JSON)
- Never panics
- Preserves index ordering of entities and relationships
- Idempotent — calling it on a complete JSON returns the parsed value unchanged

---

## Fix C — Token-Aware Embedding Sub-batching

**File**: `edgequake/crates/edgequake-pipeline/src/pipeline/helpers.rs`

Added `embed_with_token_budget(provider, texts)` — a pure free function that replaces all
three `provider.embed_batched(...)` calls in `generate_all_embeddings`.

### Algorithm

```
token_budget = provider.max_tokens() × EMBED_SAFETY_FACTOR   (0.85)
batch_start  = 0
batch_tokens = 0

for each text[i]:
    text_tokens = ceil(len(text) / EMBED_CHARS_PER_TOKEN)   (2.5 chars/token)

    if batch_tokens + text_tokens > token_budget  AND  i > batch_start:
        flush sub-batch [batch_start..i]
        batch_start = i
        batch_tokens = 0

    batch_tokens += text_tokens

flush final sub-batch [batch_start..]
```

### Constants (shared with `guard_for_embedding`)

| Constant                | Value | Rationale                                                  |
| ----------------------- | ----- | ---------------------------------------------------------- |
| `EMBED_CHARS_PER_TOKEN` | 2.5   | Conservative for dense technical content (scientific PDFs) |
| `EMBED_SAFETY_FACTOR`   | 0.85  | 15% headroom for tokenizer variance                        |

### Fallback

When `provider.max_tokens() == 0` (limit unknown), the function delegates directly to
`embed_batched` to preserve existing behavior.

### Before vs After

```
Before:  provider.embed_batched(all_142_entity_texts)
         → single HTTP call → 400 "Too many tokens overall"

After:   embed_with_token_budget(provider, all_142_entity_texts)
         → splits into sub-batches of ~68 tokens each
         → N HTTP calls, each within the 8192-token budget
         → all 142 embeddings returned in original order
```

---

## Test Coverage Added

| File                  | Test                                                | Covers                         |
| --------------------- | --------------------------------------------------- | ------------------------------ |
| `extractor/llm.rs`    | `test_recover_already_complete_json`                | Fast-path empty suffix         |
| `extractor/llm.rs`    | `test_recover_truncated_after_complete_entity`      | Mid-array truncation           |
| `extractor/llm.rs`    | `test_recover_truncated_entities_array_open`        | Open relationships array       |
| `extractor/llm.rs`    | `test_recover_returns_none_for_unrecoverable_input` | Garbage input                  |
| `extractor/llm.rs`    | `test_parse_response_recovers_partial_json`         | Integration: salvage entities  |
| `extractor/llm.rs`    | `test_parse_response_fails_on_unrecoverable_json`   | Error propagation              |
| `pipeline/helpers.rs` | `test_embed_max_chars_*`                            | Token budget calculation       |
| `pipeline/helpers.rs` | `test_truncate_*`                                   | UTF-8 boundary safety          |
| `pipeline/helpers.rs` | `test_guard_*`                                      | Input guard behavior           |
| `pipeline/helpers.rs` | `test_embed_budget_single_batch_*`                  | No split when within budget    |
| `pipeline/helpers.rs` | `test_embed_budget_splits_batches_correctly`        | Split + reassembly correctness |
| `pipeline/helpers.rs` | `test_embed_budget_empty_input`                     | Empty input edge case          |
| `pipeline/helpers.rs` | `test_embed_budget_zero_max_tokens_fallback`        | Unknown-limit fallback         |
