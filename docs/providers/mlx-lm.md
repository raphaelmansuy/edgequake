---
title: MLX-LM
description: Connect EdgeQuake to Apple's mlx_lm.server through its OpenAI-shaped API, with the real default port and a health note.
---

`mlx_lm.server` is Apple's official MLX server. EdgeQuake treats it as an OpenAI-shaped local server.

```bash
export EDGEQUAKE_LLM_PROVIDER=mlx-lm
export MLX_LM_HOST=http://127.0.0.1:8083
export MLX_LM_MODEL=default
```

| Variable | Purpose |
|----------|---------|
| `MLX_LM_HOST` | Server URL. Aliases: `MLX_LM_BASE_URL`, `MLXLM_HOST`. |
| `MLX_LM_MODEL` | Model name. `default` means the loaded model. |
| `MLX_LM_API_KEY` | Optional key (alias `MLXLM_API_KEY`). |
| `MLX_LM_EMBEDDING_MODEL` | Embedding model, if your server offers one. |

## Pick a port and set it explicitly

The client's built-in default is `http://127.0.0.1:8080`. The `/health` check and the test probe assume `http://127.0.0.1:8083`. Because the two disagree, always set `MLX_LM_HOST` and start the server on the same port:

```bash
mlx_lm.server --model <your-model> --port 8083
curl http://127.0.0.1:8083/v1/models
```

`/health` reports `degraded` when the server does not answer on `/v1/models`.
