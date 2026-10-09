---
title: Generic OpenAI-shaped server
description: Connect any server that speaks the OpenAI Chat Completions API, using environment variables or a saved Connection.
---

Many servers copy the OpenAI API: vLLM, LiteLLM, text-generation-inference, corporate gateways and others. Use this page when none of the named providers fits.

## With environment variables

```bash
export EDGEQUAKE_LLM_PROVIDER=openai-compatible
export OPENAI_COMPATIBLE_BASE_URL=http://127.0.0.1:8000/v1
export OPENAI_COMPATIBLE_API_KEY=optional
export OPENAI_COMPATIBLE_MODEL=my-model
```

| Variable | Purpose |
|----------|---------|
| `OPENAI_COMPATIBLE_BASE_URL` | Required. Include the `/v1` part. EdgeQuake appends `/chat/completions` and `/embeddings` to it. |
| `OPENAI_COMPATIBLE_API_KEY` | Optional. Sent as a Bearer token when set. |
| `OPENAI_COMPATIBLE_MODEL` | Chat model. Defaults to `default`. |
| `OPENAI_COMPATIBLE_EMBEDDING_MODEL` | Optional embedding model. |

The environment holds one such server. For two servers, use two Connections.

## With a saved Connection

Open Settings, then **LLM connections**. Enter a slug, the shape `openai_chat`, the base URL and an optional key, then press **Test connection** and **Save**. Saving needs PostgreSQL, an admin credential and, if you enter a key, `EDGEQUAKE_SECRETS_KEY` (see [Provider security](security.md)). The same fields work through `POST /api/v1/connections`.

Accepted shapes: `openai_chat` (also `openai`, `openai-compatible`, `omlx`, `mlx-lm`, `llamacpp`, `vllm-mlx`, `lmstudio`) and `anthropic_messages`. Any other value is passed to the provider factory by name.

## Known limitation: the `/v1` suffix

The test probe adds `/v1` itself (it calls `{base_url}/v1/models`), so it expects the server root, such as `http://127.0.0.1:9050`. The chat client built from a saved Connection uses the stored URL as given and appends only `/chat/completions`. From reading the code, a Connection saved without `/v1` can pass the test and then fail at chat time. This is a suspected code issue, not yet confirmed against a live server. If chat fails with a 404 after a green test, edit the Connection and add `/v1`, or use the environment variables above.

## Related

- [Roles](roles.md)
- [Providers overview](index.md)
