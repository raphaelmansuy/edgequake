---
title: Anthropic
description: Use Claude models through Anthropic's API or an Anthropic-shaped local server, and pair them with an embedding provider.
---

Anthropic provides chat models only. You must choose a different provider for embeddings, such as OpenAI or Ollama.

## Cloud

```bash
export EDGEQUAKE_LLM_PROVIDER=anthropic
export ANTHROPIC_API_KEY=sk-ant-...
export EDGEQUAKE_EMBEDDING_PROVIDER=openai   # Anthropic has no embeddings
export OPENAI_API_KEY=sk-...
```

The key is sent as the `x-api-key` header together with `anthropic-version`. The client also accepts `ANTHROPIC_AUTH_TOKEN` when `ANTHROPIC_API_KEY` is empty. The catalog lists models such as `claude-sonnet-4-6`; set one with `EDGEQUAKE_LLM_MODEL`.

## Anthropic-shaped local server

Any server that implements `POST /v1/messages` works:

```bash
export EDGEQUAKE_LLM_PROVIDER=anthropic
export ANTHROPIC_BASE_URL=http://127.0.0.1:8080
export ANTHROPIC_API_KEY=local
```

Use the server root without a trailing path. In Docker, use `http://host.docker.internal:PORT`.

## How the test probe differs

`POST /api/v1/providers/test` with `"shape":"anthropic_messages"` lists models at `/v1/models` and sends a one-token message to `/v1/messages`. It sends `x-api-key` by default. If you set `"auth_scheme":"bearer"` it sends `Authorization: Bearer` instead. The probe skips the embedding check for this shape.

## Notes

- A saved Connection for an Anthropic-shaped server uses `api_shape=anthropic_messages` (see [Roles](roles.md)).
- For a no-key local setup, see [Ollama](ollama.md).
