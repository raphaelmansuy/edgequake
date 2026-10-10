---
title: "Integration: Open WebUI"
description: Connect Open WebUI to EdgeQuake's Ollama-compatible API for a chat UI backed by Graph-RAG. Pins product image 0.32.2 and model edgequake:latest.
---

# Integration: Open WebUI

Connect [Open WebUI](https://github.com/open-webui/open-webui) to EdgeQuake's Ollama-compatible API. You get a ChatGPT-style chat UI that answers from your Graph-RAG workspace. Upload documents through EdgeQuake (its WebUI or the REST API), not through Open WebUI's file picker.

| Service | URL |
|---------|-----|
| EdgeQuake WebUI | `http://localhost:3000` |
| EdgeQuake API | `http://localhost:8080` |
| Open WebUI (this guide) | `http://localhost:8081` (avoids clashing with `:3000`) |

## How the pieces connect

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    OW["Open WebUI :8081"] -->|"POST /api/chat"| EQ["EdgeQuake :8080"]
    EQ --> PG["PostgreSQL"]
    EQ --> LLM["Configured LLM"]
%% eq-classes
classDef eqActor fill:#FCE7F3,stroke:#EC4899,color:#500724
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
class OW eqActor
class PG eqStore
class LLM eqLlm
```

Read it left to right. Open WebUI speaks the Ollama protocol. EdgeQuake runs Graph-RAG and calls your configured LLM. In Open WebUI, the model is always **`edgequake:latest`**.

## 1. Start EdgeQuake

```bash
EDGEQUAKE_VERSION=0.32.2 docker compose -f docker-compose.quickstart.yml up -d
# or: make dev
curl -s http://localhost:8080/health
curl -s http://localhost:8080/api/tags   # should list edgequake:latest
```

Images: `ghcr.io/raphaelmansuy/edgequake:0.32.2`, `edgequake-frontend:0.32.2` and `edgequake-postgres`. The tag follows `EDGEQUAKE_VERSION` and `EDGEQUAKE_POSTGRES_TAG`.

Make sure a real LLM is reachable. For example, run Ollama on the host at port `11434`.

Keep Ollama emulation enabled (the default). With `EDGEQUAKE_OLLAMA_COMPAT_ENABLED=false`, every `/api/*` Ollama route returns 503. See [Extended API: Ollama emulation](../api-reference/extended-api.md#ollama-emulation).

## 2. Upload documents

```bash
curl -s -X POST http://localhost:8080/api/v1/documents/pdf \
  -H "X-Workspace-ID: $WORKSPACE_ID" \
  -F "file=@document.pdf" \
  -F "title=My Document"
```

Wait until the document `display_status` is `completed`, or the task is `indexed`. See [PDF ingestion](../tutorials/pdf-ingestion.md).

## 3. Start Open WebUI

macOS or Docker Desktop:

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

Open `http://localhost:8081`, create an admin account, and select the model **`edgequake:latest`**.

## Compose example

This file runs PostgreSQL, the API, the frontend and Open WebUI together. It pins 0.32.2.

```yaml
# docker-compose.open-webui.yml, pinned to 0.32.2
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
      # Required here: without a strong JWT_SECRET, startup is fatal when auth is off.
      # For production, set JWT_SECRET (32+ bytes) and remove this line.
      EDGEQUAKE_DEV_MODE: "true"
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

- Open WebUI calls `POST /api/chat` (streaming is on by default) and `POST /api/generate`. It also reads `GET /api/tags`, `GET /api/version` and `GET /api/ps`.
- EdgeQuake answers with its own model name, `edgequake:latest`, whatever model Open WebUI requests.
- The search mode comes from a prefix in your message. The default is `hybrid`.

| Prefix | Mode |
|--------|------|
| `/local ` | Local |
| `/global ` | Global |
| `/naive ` | Naive |
| `/hybrid ` | Hybrid (default) |
| `/mix ` | Mix |
| `/bypass ` | No retrieval; the LLM answers directly |
| `/context ` | Mix, returns the context only |

- There is no `/v1/chat/completions` or `/v1/embeddings` on the Ollama surface. For sources and modes from code, use [`POST /api/v1/chat/completions`](../api-reference/rest-api.md#chat).
- Requests use the workspace named in `X-Workspace-ID`. Without that header, they use the default workspace. To target another workspace, put a proxy in front that adds the header.

## Troubleshoot

| Problem | Check |
|---------|-------|
| Connection error in Open WebUI | `curl http://localhost:8080/health` and the `OLLAMA_BASE_URL` value |
| Empty answers | Documents are not indexed yet, or the LLM is down |
| Model missing | `curl http://localhost:8080/api/tags` should list `edgequake:latest` |
| `/api/*` returns 503 | `EDGEQUAKE_OLLAMA_COMPAT_ENABLED` is set to false |
| API container exits at startup | Set `EDGEQUAKE_DEV_MODE` for local use, or set a strong `JWT_SECRET` |

Related: [Extended API](../api-reference/extended-api.md#ollama-emulation), [Quick start](../getting-started/quick-start.md), [Provider security](../providers/security.md).
