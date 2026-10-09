---
title: vLLM-MLX
description: Connect EdgeQuake to a vLLM-MLX server through its OpenAI-shaped API, with the port caveat.
---

vLLM-MLX is a vLLM-style server for Apple Silicon. EdgeQuake treats it as an OpenAI-shaped local server.

```bash
export EDGEQUAKE_LLM_PROVIDER=vllm-mlx
export VLLM_MLX_HOST=http://127.0.0.1:8082
export VLLM_MLX_MODEL=default
```

| Variable | Purpose |
|----------|---------|
| `VLLM_MLX_HOST` | Server URL (alias `VLLM_MLX_BASE_URL`). |
| `VLLM_MLX_MODEL` | Model name. `default` means the loaded model. |
| `VLLM_MLX_API_KEY` | Optional key. |
| `VLLM_MLX_EMBEDDING_MODEL` | Embedding model, if offered. |

## Set the port explicitly

The client's built-in default is `http://127.0.0.1:8000`. The `/health` check and the test probe assume `http://127.0.0.1:8082`. Start your server on the port you put in `VLLM_MLX_HOST` so all three agree.

```bash
curl "$VLLM_MLX_HOST/v1/models"
```

`/health` reports `degraded` when the server does not answer on `/v1/models`.
