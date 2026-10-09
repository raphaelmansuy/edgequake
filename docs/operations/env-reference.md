---
title: Environment variable reference
description: Generated from edgequake/env_registry.toml (SPEC-163).
---

> **Generated.** Do not edit by hand. `python3 scripts/generate_env_reference.py`

| Variable | Purpose | Documented in |
|----------|---------|---------------|
| `EDGEQUAKE_LLM_PROVIDER` | Process default LLM provider id | [`docs/providers/index.md`](../../docs/providers/index.md), [`docs/operations/configuration.md`](../../docs/operations/configuration.md) |
| `EDGEQUAKE_DEFAULT_LLM_PROVIDER` | Alias for EDGEQUAKE_LLM_PROVIDER | [`docs/operations/configuration.md`](../../docs/operations/configuration.md) |
| `EDGEQUAKE_LLM_MODEL` | Process default chat model | [`docs/providers/index.md`](../../docs/providers/index.md) |
| `EDGEQUAKE_EMBEDDING_PROVIDER` | Embedding provider (may differ from LLM) | [`docs/providers/roles.md`](../../docs/providers/roles.md) |
| `EDGEQUAKE_EMBEDDING_MODEL` | Embedding model id | [`docs/providers/roles.md`](../../docs/providers/roles.md) |
| `EDGEQUAKE_EMBEDDING_DIMENSION` | Vector size when the catalog has no card | [`docs/providers/omlx.md`](../../docs/providers/omlx.md) |
| `OLLAMA_HOST` | Ollama base URL | [`docs/providers/ollama.md`](../../docs/providers/ollama.md) |
| `LMSTUDIO_HOST` | LM Studio base URL (not LM_STUDIO_BASE_URL) | [`docs/providers/lmstudio.md`](../../docs/providers/lmstudio.md) |
| `OMLX_HOST` | oMLX base URL | [`docs/providers/omlx.md`](../../docs/providers/omlx.md) |
| `OPENAI_API_KEY` | OpenAI API key | [`docs/providers/openai.md`](../../docs/providers/openai.md) |
| `ANTHROPIC_API_KEY` | Anthropic API key | [`docs/providers/anthropic.md`](../../docs/providers/anthropic.md) |
| `ANTHROPIC_BASE_URL` | Anthropic or Anthropic-shaped base URL | [`docs/providers/anthropic.md`](../../docs/providers/anthropic.md) |
| `OPENAI_COMPATIBLE_BASE_URL` | Generic OpenAI-shaped server | [`docs/providers/openai-compatible.md`](../../docs/providers/openai-compatible.md) |
| `EDGEQUAKE_SECRETS_KEY` | 32-byte envelope key for stored connection secrets | [`docs/providers/security.md`](../../docs/providers/security.md) |
| `EDGEQUAKE_SETUP_TOKEN` | Required header for POST /setup/initialize when set | [`docs/providers/security.md`](../../docs/providers/security.md) |
| `EDGEQUAKE_AUTH_ENABLED` | Explicit auth on/off; wins over DEV_MODE | [`docs/providers/security.md`](../../docs/providers/security.md) |
| `JWT_SECRET` | JWT HMAC secret (≥32 bytes) | [`docs/providers/security.md`](../../docs/providers/security.md) |
