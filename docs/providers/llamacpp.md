---
title: llama.cpp
description: Connect EdgeQuake to llama-server from llama.cpp through its OpenAI-shaped API, with the port caveat.
---

`llama-server` from llama.cpp serves GGUF models on almost any hardware. EdgeQuake treats it as an OpenAI-shaped local server.

```bash
export EDGEQUAKE_LLM_PROVIDER=llamacpp
export LLAMACPP_HOST=http://127.0.0.1:8081
export LLAMACPP_MODEL=default
```

| Variable | Purpose |
|----------|---------|
| `LLAMACPP_HOST` | Server URL. Aliases: `LLAMA_SERVER_HOST`, `LLAMACPP_BASE_URL`, `LLAMA_SERVER_BASE_URL`. |
| `LLAMACPP_MODEL` | Model name. `default` means the loaded model. |
| `LLAMACPP_API_KEY` | Optional key (alias `LLAMA_SERVER_API_KEY`). |
| `LLAMACPP_EMBEDDING_MODEL` | Embedding model, if you run the server with embeddings enabled. |

## Set the port explicitly

The client's built-in default is `http://127.0.0.1:8080`. The `/health` check and the test probe assume `http://127.0.0.1:8081`. Start the server on the port in `LLAMACPP_HOST`:

```bash
llama-server -m model.gguf --port 8081
curl http://127.0.0.1:8081/v1/models
```

`/health` reports `degraded` when the server does not answer on `/v1/models`.
