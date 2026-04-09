# ADR-003 — In-Wizard Model Catalogue and Selection

**Status:** Accepted  
**Date:** 2026-04-09  
**Version:** EdgeQuake v0.9.12

---

## Context

v0.9.11 hard-coded model defaults. Users wanting to change models had to:
1. Know the env var names (`EDGEQUAKE_LLM_MODEL`, `EDGEQUAKE_EMBEDDING_MODEL`)
2. Export them before running the script
3. Re-run

This is poor UX. Models differ significantly in cost, size, and capability — the choice
should be surfaced in the wizard, not buried in documentation.

## Decision

After provider selection, present a two-step model wizard:

1. **LLM model** — primary inference model
2. **Embedding model** — vector embedding model (defaults to same provider)

### OpenAI Model Catalogue (as of v0.9.12)

| #   | Model          | Position                        | Cost (in/out per MTok) |
| --- | -------------- | ------------------------------- | ---------------------- |
| 1   | `gpt-5-mini` ★ | Recommended — fast & affordable | $0.40 / $1.60          |
| 2   | `gpt-5-nano`   | Ultra-cheap, great for testing  | $0.20 / $0.80          |
| 3   | `gpt-5.4`      | Premium quality, large context  | $2.50 / $15.00         |
| 4   | `gpt-5.4-mini` | Fast with larger context        | $0.75 / $4.50          |

### OpenAI Embedding Catalogue

| #   | Model                      | Notes                         |
| --- | -------------------------- | ----------------------------- |
| 1   | `text-embedding-3-small` ★ | Recommended — 1536 dims, fast |
| 2   | `text-embedding-3-large`   | Higher quality, 3072 dims     |

### Ollama LLM Catalogue (as of v0.9.12)

| #   | Model             | Notes                                     | Disk   |
| --- | ----------------- | ----------------------------------------- | ------ |
| 1   | `gemma4:e4b` ★    | Recommended — balanced quality/size       | 9.6 GB |
| 2   | `gemma4:e2b`      | Lighter, faster startup                   | 7.2 GB |
| 3   | `gemma4:26b`      | Large MoE, best quality (needs ≥16GB RAM) | ~17 GB |
| 4   | `qwen2.5:latest`  | Strong at structured tasks                | ~5 GB  |
| 5   | `llama3.2:latest` | Meta's general-purpose model              | ~2 GB  |

### Ollama Embedding Catalogue

| #   | Model                     | Notes                            |
| --- | ------------------------- | -------------------------------- |
| 1   | `embeddinggemma:latest` ★ | Recommended — fast, high quality |
| 2   | `nomic-embed-text:latest` | Alternative — well-tested        |

## Embedding Provider

The embedding provider always matches the LLM provider selected in ADR-001.

> Rationale: Mixed providers (e.g. OpenAI LLM + Ollama embeddings) are a power-user
> feature that increases failure modes (two providers to validate, possible dimension
> mismatch). For v0.9.12 the wizard exposes same-provider embeddings only.
> Power users can still override via `EDGEQUAKE_EMBEDDING_PROVIDER` env var before running.

## Consequences

### Positive
- Users see the performance/cost trade-off and choose consciously.
- Catalogue keeps reasonable breadth without overwhelming new users.
- Recommended options are marked with ★ (shown inline in the menu label).

### Negative
- Catalogue goes stale as new models are released.
  **Mitigation:** Model names are chosen with date-stable aliases (e.g. `gemma4:e4b` not
  `gemma4:e4b-20250415`). The catalogue is updated with each EdgeQuake release.

## Maintenance Protocol

When adding/removing a model from the catalogue:
1. Update the model menu strings in `quickstart.sh` `choose_model()`.
2. Update this ADR's tables.
3. Bump the patch version (catalogue changes are non-breaking).

## Edge Cases

| Edge case                                          | Handling                                                                   |
| -------------------------------------------------- | -------------------------------------------------------------------------- |
| Ollama model not pulled                            | Warn with `ollama pull <model>` command; non-blocking                      |
| OpenAI model deprecated                            | API returns error at inference time; user sees actionable message from API |
| User selects large Ollama model on low-RAM machine | Warning on `gemma4:26b` option label                                       |
| Embedding model unavailable in Ollama              | Same warn-and-continue as LLM model                                        |
