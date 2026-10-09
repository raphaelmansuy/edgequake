---
title: Ollama
description: Run EdgeQuake against a local Ollama server, including the Docker host address, default models and health check.
---

Ollama runs open models on your machine. It needs no API key. EdgeQuake talks to its native API.

## Set it up

```bash
ollama serve
ollama pull gemma4:latest
ollama pull embeddinggemma
export EDGEQUAKE_LLM_PROVIDER=ollama
export EDGEQUAKE_LLM_MODEL=gemma4:latest
export EDGEQUAKE_EMBEDDING_PROVIDER=ollama
export OLLAMA_EMBEDDING_MODEL=embeddinggemma
export OLLAMA_HOST=http://127.0.0.1:11434
```

| Variable | Purpose |
|----------|---------|
| `OLLAMA_HOST` | Server URL. Defaults to `http://localhost:11434`. |
| `EDGEQUAKE_LLM_MODEL` | Chat model. Must be pulled first. |
| `OLLAMA_EMBEDDING_MODEL` | Embedding model. `embeddinggemma` is the quickstart default. |

## Docker

A container cannot reach `127.0.0.1` on your host. Use `http://host.docker.internal:11434`. The quickstart script rewrites loopback addresses for you and tells you what it chose.

## Check it

```bash
curl http://127.0.0.1:11434/api/tags
curl http://127.0.0.1:8080/health
```

`/health` reports `degraded` when Ollama does not answer (it calls `/api/version`). The test probe uses the `ollama` shape: it checks `/api/version` and a chat ping on `/api/chat`, and skips the embedding check.

## Notes

- Local models are slow. EdgeQuake applies longer timeouts and lower concurrency to Ollama than to cloud providers.
- If extraction fails with a network error, Ollama is not running. See [Troubleshooting](../troubleshooting/index.md).
- A saved Connection with `api_shape=ollama` is tested against its own URL, but the runtime client reads `OLLAMA_HOST`. Set `OLLAMA_HOST` too.
