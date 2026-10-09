# 09 — Test proof protocol

| Layer | What |
|-------|------|
| Unit | SSRF encodings, AES-GCM round-trip, SecretString Debug, locality, taxonomy |
| Fake LLM | OpenAI chat/embed/models, Anthropic messages (x-api-key + Bearer), Ollama tags/chat, 401/500/wrong-dim |
| API contract | POST /providers/test, connections CRUD, non-admin 403 |
| Doctor | exit codes, JSON shape |
| Compose | `scripts/spec163/onboarding_e2e.sh` against fake server |
| SPEC-150 | schema 169 expand-only; `make schema-train-docs` |

LAW-163-1: a health proof is valid only if the historically lying shape (hard-coded true) is gone.
