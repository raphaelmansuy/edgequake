# ADR-001 — Explicit Provider Selection via Numbered Menu

**Status:** Accepted  
**Date:** 2026-04-09  
**Version:** EdgeQuake v0.9.12

---

## Context

The v0.9.11 quickstart used this heuristic to pick the LLM provider:

```sh
if [ -n "$OPENAI_API_KEY" ]; then
  EDGEQUAKE_LLM_PROVIDER="${EDGEQUAKE_LLM_PROVIDER:-openai}"
else
  EDGEQUAKE_LLM_PROVIDER="${EDGEQUAKE_LLM_PROVIDER:-ollama}"
fi
```

This is a **flaky heuristic** with multiple failure modes:

| Scenario                                                         | Heuristic result       | User expectation            |
| ---------------------------------------------------------------- | ---------------------- | --------------------------- |
| Key exported in shell profile but user wants Ollama              | OpenAI                 | Ollama                      |
| Using a custom OpenAI-compatible endpoint via Ollama compat mode | Ollama                 | OpenAI with OPENAI_BASE_URL |
| CI/CD with key in secrets but Ollama on sidecar                  | OpenAI                 | Could be either             |
| Testing different providers in the same session                  | Last exported key wins | Explicit choice each time   |

The heuristic also bypasses the user entirely — they get no confirmation that the correct
provider was selected, and must understand the invisible env-var logic to override it.

## Decision

**The wizard always asks the user which provider to use**, via a numbered menu:

```
  Which LLM provider do you want to use?

    [1]  OpenAI   — cloud API (GPT-5 family), requires OPENAI_API_KEY
    [2]  Ollama   — fully local, free to run, requires Ollama daemon on port 11434

  Enter choice (1–2):
```

Rules:
1. **No auto-detection.** The env var `OPENAI_API_KEY` is never used to pick the provider.
2. **Informational hint only.** If `OPENAI_API_KEY` is set, show a one-line hint
   `"→  OPENAI_API_KEY is set in your environment."` before the menu. This is purely
   informational — it does not pre-select or highlight an option.
3. **Input always from `/dev/tty`** so the menu works when stdin is piped (see ADR-004).
4. **Loop until valid.** Invalid input (non-number, out-of-range) re-prompts with a warning.

## Consequences

### Positive
- **Zero ambiguity.** The user's choice is always explicit and logged back to them.
- **Two-pass validation.** Provider is chosen first; API key / Ollama reachability checked
  afterward — giving clear, targeted error messages.
- **Testable.** Wizard choice is deterministic given a specific key press; no env-var
  side-effects.

### Negative
- **One extra interaction step** for returning users who don't want to reconfigure.
  Mitigated by: the "Update & Reconfigure" flow for running containers, which is the common
  re-run path.

## Alternatives Rejected

| Alternative                       | Reason rejected                                                   |
| --------------------------------- | ----------------------------------------------------------------- |
| Auto-detect from `OPENAI_API_KEY` | Flaky; fails in hybrid / edge scenarios                           |
| Pre-fill menu default from env    | Still requires verification; adds hidden state                    |
| `--provider` CLI flag             | Breaks `curl \| sh` UX; env vars are the right override mechanism |

## Implementation Notes

- `LLM_PROVIDER` is a script-global variable set by the wizard; never read from env.
- EMBED_PROVIDER defaults to the same as LLM_PROVIDER (always consistent).
- After provider selection, model selection wizard runs (ADR-003).
- After model selection, provider validation runs (key check / Ollama ping).
