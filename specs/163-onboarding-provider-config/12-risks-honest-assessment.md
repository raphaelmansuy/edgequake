# 12 — Risks (honest)

- CI does not boot real oMLX/MLX. Local matrix is opt-in.
- Anthropic Bearer is on the test probe; runtime LLM still uses `x-api-key` until an `edgequake-llm` release that keeps Authorization.
- Connections require PostgreSQL. Memory mode has env providers only.
- Envelope key is env-held. Compromise of the host process still yields plaintext in memory.
- First-run wizard in dev mode is dismissible-by-completion, not a security boundary.
