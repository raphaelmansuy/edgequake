# SPEC-163 hermetic proof

Measured 2026-10-09 via `make spec163-proof`.

| Gate | Result |
|------|--------|
| `edgequake-secrets` round-trip / redaction | PASS |
| Fake LLM OpenAI + Anthropic + Ollama + faults | PASS |
| locality / SSRF / doctor | PASS |
| `spec163_probe` (401 + metadata SSRF) | PASS |
| Schema train HEAD = 169 | PASS |
| Env registry | PASS (`make spec163-env-docs`) |
| Hermetic elapsed | 108 s (budget 120 s) |

Honest bounds: CI does not run real oMLX/MLX/LM Studio (`make spec163-local-matrix`). Full `make spec150-matrix` through 169 is operator-run. Product crate pin remains **v0.32.2** until the tagged 0.33.0 cut; schema **169** is on HEAD.
