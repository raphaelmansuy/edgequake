# 02 — Incident catalogue (pre-163)

| ID | Defect | User-visible |
|----|--------|----------------|
| I1 | `/health` hard-codes `llm_provider: true` | Dashboard "All systems operational" with Ollama down |
| I2 | Local omlx/mlx-lm/llamacpp/vllm-mlx reported healthy at 0 ms | False confidence |
| I3 | No test-connection | First failure is a failed document |
| I4 | Workspace stores names only | Cannot run two OpenAI-shaped servers |
| I5 | `EDGEQUAKE_EMBEDDING_MODEL` compose default is `text-embedding-3-small` | Ollama asked for an OpenAI model |
| I6 | First-run wizard requires `auth_enabled` | Quickstart never sees it |
| I7 | AUTH_ENABLED overridden by DEV_MODE | Stuck on open API |
| I8 | `LM_STUDIO_BASE_URL` in docs vs `LMSTUDIO_HOST` in code | Config does nothing |
| I9 | Seven install paths, conflicting ports and models | Newcomer paralysis |
| I10 | `ProviderFactory::from_env().expect` | Panic instead of actionable error |
