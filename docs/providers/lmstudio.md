---
title: LM Studio
description: Connect EdgeQuake to the LM Studio local server using its OpenAI-shaped API.
---

LM Studio is a desktop app that serves local models. Start its local server, load a model, then point EdgeQuake at it.

The variable is `LMSTUDIO_HOST`. `LM_STUDIO_BASE_URL` is not read.

```bash
export EDGEQUAKE_LLM_PROVIDER=lmstudio
export LMSTUDIO_HOST=http://127.0.0.1:1234
export LMSTUDIO_MODEL=your-loaded-model
```

| Variable | Purpose |
|----------|---------|
| `LMSTUDIO_HOST` | Server URL. Defaults to `http://localhost:1234`. |
| `LMSTUDIO_MODEL` | Name of the model you loaded. LM Studio only serves what is loaded. |

## Check it

```bash
curl http://127.0.0.1:1234/v1/models
curl http://127.0.0.1:8080/health
```

`/health` reports `degraded` when LM Studio does not answer on `/v1/models`. In Docker, use `http://host.docker.internal:1234`.

## Notes

- `quickstart.sh --provider lmstudio --base-url URL` sets `LMSTUDIO_HOST` for you.
- Embeddings need an embedding model loaded in LM Studio and `EDGEQUAKE_EMBEDDING_PROVIDER=lmstudio`.
