---
title: "Integration: Open WebUI"
description: Connect Open WebUI to EdgeQuake's Ollama-compatible API for a chat UI backed by Graph-RAG. Pins product image 0.32.2 and model edgequake:latest.
---

# Integration: Open WebUI

Connect [Open WebUI](https://github.com/open-webui/open-webui) to EdgeQuake's Ollama-compatible API so you get a ChatGPT-style UI on top of Graph-RAG. Upload documents through EdgeQuake (WebUI or REST), not through Open WebUI's file picker.

| Service | URL |
|---------|-----|
| EdgeQuake WebUI | `http://localhost:3000` |
| EdgeQuake API | `http://localhost:8080` |
| Open WebUI (this guide) | `http://localhost:8081` (avoids clashing with `:3000`) |

```mermaid
flowchart LR
    OW["Open WebUI :8081"] -->|"POST /api/chat"| EQ["EdgeQuake :8080"]
    EQ --> PG["PostgreSQL"]
    EQ --> LLM["Configured LLM"]
```

Read it left to right: Open WebUI speaks the Ollama protocol; EdgeQuake runs Graph-RAG and calls your real LLM. The model name shown in Open WebUI is always **`edgequake:latest`**.

## 1. Start EdgeQuake

```bash
EDGEQUAKE_VERSION=0.32.2 docker compose -f docker-compose.quickstart.yml up -d
# or: make dev
curl -s http://localhost:8080/health
curl -s http://localhost:8080/api/tags   # should list edgequake:latest
```

Images: `ghcr.io/raphaelmansuy/edgequake:0.32.2`, `edgequake-frontend:0.32.2`, `edgequake-postgres` (tag follows `EDGEQUAKE_VERSION` / `EDGEQUAKE_POSTGRES_TAG`). Ensure a real LLM is reachable (for example Ollama on the host at `:11434`).

Ollama emulation must stay enabled (default). Setting `EDGEQUAKE_OLLAMA_COMPAT_ENABLED=false` returns 503 on `/api/*`. Details: [Extended API: Ollama emulation](../api-reference/extended-api.md#ollama-emulation).

## 2. Upload documents

```bash
curl -s -X POST http://localhost:8080/api/v1/documents/pdf \
  -H "X-Workspace-ID: $WORKSPACE_ID" \
  -F "file=@document.pdf" \
  -F "title=My Document"
```

Wait until the document `display_status` is `completed` (or the task is `indexed`). See [PDF ingestion](../tutorials/pdf-ingestion.md).

## 3. Start Open WebUI

macOS / Docker Desktop:

```bash
docker run -d \
  -p 8081:8080 \
  -e OLLAMA_BASE_URL=http://host.docker.internal:8080 \
  --name open-webui \
  ghcr.io/open-webui/open-webui:main
```

Linux:

```bash
docker run -d \
  -p 8081:8080 \
  -e OLLAMA_BASE_URL=http://172.17.0.1:8080 \
  --add-host=host.docker.internal:host-gateway \
  --name open-webui \
  ghcr.io/open-webui/open-webui:main
```

Open `http://localhost:8081`, create an admin account, select model **`edgequake:latest`**.

## Compose example

```yaml
# docker-compose.open-webui.yml — pin 0.32.2
services:
  postgres:
    image: ghcr.io/raphaelmansuy/edgequake-postgres:0.32.2
    environment:
      POSTGRES_USER: edgequake
      POSTGRES_PASSWORD: edgequake_secret
      POSTGRES_DB: edgequake
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U edgequake -d edgequake"]
      interval: 10s
      retries: 5

  api:
    image: ghcr.io/raphaelmansuy/edgequake:0.32.2
    ports: ["8080:8080"]
    environment:
      DATABASE_URL: postgres://edgequake:edgequake_secret@postgres:5432/edgequake
      EDGEQUAKE_LLM_PROVIDER: ollama
      OLLAMA_HOST: http://host.docker.internal:11434
    extra_hosts: ["host.docker.internal:host-gateway"]
    depends_on:
      postgres:
        condition: service_healthy

  frontend:
    image: ghcr.io/raphaelmansuy/edgequake-frontend:0.32.2
    ports: ["3000:3000"]
    environment:
      EDGEQUAKE_API_URL: http://localhost:8080
    depends_on: [api]

  open-webui:
    image: ghcr.io/open-webui/open-webui:main
    ports: ["8081:8080"]
    environment:
      OLLAMA_BASE_URL: http://api:8080
    depends_on: [api]
```

```bash
docker compose -f docker-compose.open-webui.yml up -d
```

## Behaviour notes

- Open WebUI calls `POST /api/chat` (stream defaults to **true**) and `POST /api/generate`. EdgeQuake ignores the `model` field and always runs a workspace RAG query.
- There is no `/v1/chat/completions` or `/v1/embeddings` on the Ollama surface. For sources and modes use [`POST /api/v1/chat/completions`](../api-reference/rest-api.md#chat).
- Scope the workspace with the same headers your API uses when you upload (`X-Workspace-ID`). The Ollama routes use the server's default workspace unless you terminate TLS or a proxy that injects headers.

| Problem | Check |
|---------|-------|
| Connection error in Open WebUI | `curl http://localhost:8080/health` and `OLLAMA_BASE_URL` |
| Empty answers | Documents not indexed yet; LLM down |
| Model missing | `curl http://localhost:8080/api/tags` should show `edgequake:latest` |
| `/api/*` returns 503 | `EDGEQUAKE_OLLAMA_COMPAT_ENABLED` is false |

Related: [Extended API](../api-reference/extended-api.md#ollama-emulation), [Quick start](../getting-started/quick-start.md).
