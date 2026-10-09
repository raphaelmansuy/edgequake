---
title: OpenAI
description: Use OpenAI for chat and embeddings in EdgeQuake, with the environment variables, default models and a quick check.
---

OpenAI is the catalog default for both chat and embeddings. You need an API key and no other setup.

## Set it up

```bash
export EDGEQUAKE_LLM_PROVIDER=openai
export OPENAI_API_KEY=sk-...
export EDGEQUAKE_LLM_MODEL=gpt-5.4-mini
export EDGEQUAKE_EMBEDDING_PROVIDER=openai
export EDGEQUAKE_EMBEDDING_MODEL=text-embedding-3-small
```

| Variable | Purpose |
|----------|---------|
| `OPENAI_API_KEY` | Required. Sent as `Authorization: Bearer`. |
| `EDGEQUAKE_LLM_MODEL` | Chat model. The quickstart picks `gpt-5.4-mini`. |
| `EDGEQUAKE_EMBEDDING_MODEL` | Embedding model. `text-embedding-3-small` returns 1536-dimension vectors. |
| `OPENAI_BASE_URL` | Optional. Send traffic through an OpenAI-compatible proxy or gateway. |

The quickstart script also offers `gpt-5.4-nano` and `gpt-5.4` for chat and `text-embedding-3-large` for embeddings. All model names come from `edgequake/models.toml`.

## Check it

```bash
curl -sS http://127.0.0.1:8080/health
```

Look for `"llm_provider": true` under `components` and your model under `providers.llm`. For OpenAI, `/health` only checks that a key is set; it does not call OpenAI. Use `POST /api/v1/providers/test` for a live call (see [Providers](index.md#test-a-server)).

## Notes

- Changing the embedding model changes the vector size. Rebuild stored embeddings after you change it.
- To store the key in the database instead of the environment, use a Connection with `api_shape=openai_chat` (see [Provider security](security.md)).
- Related: [Roles](roles.md), [Anthropic](anthropic.md).
