# ADR-006 — Exclude Reasoning-Only Models from Entity Extraction Pipeline

**Status:** Accepted  
**Date:** 2026-04-09  
**Context:** EdgeQuake v0.9.13 — post-mortem of v0.9.12 embedding / vector-search regression

---

## Problem

When a user selects **gpt-5-mini** or **gpt-5-nano** in the quickstart wizard, every entity-
extraction API call fails silently:

```
WARN  edgequake_api: Chunk extraction failed, will retry...
      error="Entity extraction error: Invalid JSON: EOF while parsing a value at line 1 column 0"
```

### Root Cause

`gpt-5-mini` and `gpt-5-nano` are **reasoning-only** (o-series style) models that reserve their
entire `completion_tokens` budget for internal chain-of-thought before emitting any visible text.

| Metric | Value |
|---|---|
| `max_completion_tokens` sent | *(none — default, no limit)* |
| `reasoning_tokens` consumed | 8 192 |
| `completion_tokens` total | 8 192 |
| **Net output tokens** | **0** |

Because `output_tokens = completion_tokens − reasoning_tokens = 0`, the model returns an **empty
string** as its response body, which the JSON parser correctly rejects with an EOF error.

### Consequence Chain

```
reasoning model → all tokens used for CoT → empty response content
  → JSON parse error → chunk extraction fails → chunk NOT stored
  → no embedding generated → vector table stays empty
  → vector similarity search finds 0 results → "0 Sources" in UI
```

Knowledge-graph entities (topics) can still appear because a few chunks may succeed in
`partial_failure` documents, but **no document chunks are ever embedded**, so source retrieval
is completely broken.

---

## Decision

### 1 — Remove pure-reasoning models from the quickstart wizard

Replace:

| Old (reasoning-only) | New (supports reasoning_effort=none) |
|---|---|
| `gpt-5-mini`  | `gpt-5.4-mini`  |
| `gpt-5-nano`  | `gpt-5.4-nano`  |

`gpt-5.4-*` models belong to OpenAI's **adjustable-reasoning** family ("Reasoning: none low
medium high xhigh") — passing `reasoning_effort=none` completely disables CoT and delivers direct
JSON output.

Add inline comment in `quickstart.sh`:

```sh
# NOTE: gpt-5-* and o-series are reasoning-only models. They consume all
# completion tokens for chain-of-thought, leaving zero tokens for JSON output.
# Always prefer gpt-5.4-* or gpt-4.1-* models for structured-extraction tasks.
```

### 2 — Set `reasoning_effort = "none"` in all extraction paths

Modify `CompletionOptions` in both extractors:

**`sota.rs`** (primary SOTA extractor):

```rust
let options = CompletionOptions {
    max_tokens: Some(current_max_tokens),
    temperature: Some(0.0),
    reasoning_effort: Some("none".to_string()), // ← new
    ..Default::default()
};
```

**`llm.rs`** (fallback LLM extractor):

```rust
let options = CompletionOptions {
    max_tokens: Some(16384),
    temperature: Some(0.0),
    reasoning_effort: Some("none".to_string()), // ← new
    ..Default::default()
};
self.llm_provider.complete_with_options(&prompt, &options).await?
```

**Why this is safe for non-reasoning models:** The `edgequake-llm` crate marks
`reasoning_effort` as `#[serde(skip_serializing_if = "Option::is_none")]`; but even when
`reasoning_effort=none` is serialised, OpenAI non-reasoning models silently ignore the field.
Ollama non-thinking models also ignore it. Only models that understand thinking budgets respect
it — and for those, `"none"` means *skip thinking entirely*.

### 3 — Improve the empty-response diagnostic message

Update the `sotа.rs` empty-content guard to include reasoning-model context in the error, so
operators can diagnose the issue faster from logs alone.

---

## Rejected Alternatives

| Alternative | Rejected because |
|---|---|
| Keep gpt-5-mini with high `max_tokens` | Raises token cap for reasoning budget; model still returns empty when reasoning exhausts cap |
| Add a provider-level safeguard that detects reasoning models | Requires model name heuristics that will fail for unknown future models; fragile |
| Remove OpenAI support entirely | Not proportional to the problem |
| Require users to set `reasoning_effort` via env var | Too hidden; most users won't know to do this |

---

## Consequences

### Positive
- Entity extraction succeeds 100 % of the time for `gpt-5.4-mini` / `gpt-5.4-nano`.
- Vector tables are populated; "0 Sources" regression is eliminated.
- Existing deployments using non-reasoning models are unaffected (option is ignored).
- Future additions of reasoning models are automatically handled by the code-level guard.

### Negative
- Users who deliberately chose `gpt-5-mini` for its reasoning capabilities will need to use
  another model or pass `EDGEQUAKE_LLM_MODEL=gpt-5.4-mini` manually.

---

## References

- OpenAI model docs: https://platform.openai.com/docs/models
- `edgequake-llm` v0.5.1 `CompletionOptions.reasoning_effort`
- OODA investigation log: `logs/2026-04-09-*-beastmode-chatmode-log.md`
- Git: `fix/reasoning-model-extraction` → PR squash-merged as v0.9.13
