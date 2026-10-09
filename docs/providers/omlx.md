---
title: oMLX
description: Run EdgeQuake against oMLX, an OpenAI-shaped local server for Apple Silicon, including settings-file discovery and embedding size.
---

oMLX serves MLX models on Apple Silicon through an OpenAI-shaped API. Its default address is `http://127.0.0.1:9050`.

## Set it up

```bash
export EDGEQUAKE_LLM_PROVIDER=omlx
export OMLX_HOST=http://127.0.0.1:9050
export OMLX_MODEL=default
export EDGEQUAKE_EMBEDDING_PROVIDER=omlx
export EDGEQUAKE_EMBEDDING_DIMENSION=768
```

| Variable | Purpose |
|----------|---------|
| `OMLX_HOST` | Server URL (alias `OMLX_BASE_URL`). A trailing `/v1` is removed. |
| `OMLX_MODEL` | Chat model name. `default` means the model oMLX has loaded. |
| `OMLX_API_KEY` | Optional key, sent as a Bearer token. |
| `OMLX_EMBEDDING_MODEL` | Embedding model name. |
| `EDGEQUAKE_EMBEDDING_DIMENSION` | Vector size. Set it when the catalog has no entry for your embedding model; it must match the model's real output size. |

## Where the URL and key come from

The client reads the environment first. If `OMLX_HOST` and `OMLX_API_KEY` are unset, it reads `~/.omlx/settings.json` (`server.host`, `server.port`, `auth.api_key`, `model.default_model`). If that file is missing too, it uses `http://127.0.0.1:9050`.

## Check it

```bash
curl http://127.0.0.1:9050/v1/models
curl http://127.0.0.1:8080/health
```

In Docker, use `http://host.docker.internal:9050`. The shortcut is `quickstart.sh --yes --provider omlx --base-url http://127.0.0.1:9050`.

## Notes

- `/health` reports `degraded` when oMLX does not answer on `/v1/models`. Its check uses `OMLX_HOST` or `OMLX_BASE_URL`, not the settings file.
- The sibling server MTPLX (`mtplx`, `MTPLX_HOST`) follows the same pattern but its client default port is 8000 while the health check assumes 9060. Set `MTPLX_HOST` explicitly.
