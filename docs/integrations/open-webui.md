---
title: 'Integration: Open WebUI'
description: "Connect Open WebUI to EdgeQuake."
---

# Integration: Open WebUI

> **Product: v0.23.0** · Contract: [`openapi.snapshot.json`](../../edgequake_webui/openapi/openapi.snapshot.json) · Spec ops: [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md)

Connect [Open WebUI](https://github.com/open-webui/open-webui) to EdgeQuake's **Ollama-compatible API** for a ChatGPT-style UI backed by Graph-RAG.

**Ports (v0.23.0)**:

| Service | URL |
| ------- | --- |
| EdgeQuake WebUI | `http://localhost:3000` |
| EdgeQuake API | `http://localhost:8080` |
| Open WebUI (this guide) | `http://localhost:8081` — avoids conflict with EdgeQuake WebUI on `:3000` |

---

## Architecture

```
Open WebUI (:8081)
       │  Ollama API (POST /api/chat)
       ▼
EdgeQuake API (:8080)
       │  Graph-RAG + knowledge graph
       ▼
PostgreSQL (pgvector + AGE)
```

Upload documents via EdgeQuake WebUI (`:3000`) or REST — not through Open WebUI's file UI (not integrated).

---

## Quick start

### 1. Start EdgeQuake (GHCR 0.23.0)

```bash
EDGEQUAKE_VERSION=0.23.0 docker compose -f docker-compose.quickstart.yml up -d
```

Images (SSOT from release CI):

```
ghcr.io/raphaelmansuy/edgequake:0.23.0
ghcr.io/raphaelmansuy/edgequake-frontend:0.23.0
ghcr.io/raphaelmansuy/edgequake-postgres:0.23.0
```

Verify:

```bash
curl -s http://localhost:8080/health | jq .status    # "healthy"
curl -s -o /dev/null -w "%{http_code}" http://localhost:3000   # 200
```

Or local dev: `make dev` (same ports).

Ensure an LLM is reachable (default: Ollama on host `:11434`).

### 2. Upload documents

```bash
curl -X POST http://localhost:8080/api/v1/documents/pdf \
  -H "X-Workspace-ID: default" \
  -F "file=@document.pdf" \
  -F "title=My Document"
```

Wait for `display_status: completed` in EdgeQuake WebUI or via `GET /api/v1/documents`. See [PDF Ingestion Tutorial](/docs/tutorials/pdf-ingestion/).

### 3. Start Open WebUI

```bash
docker run -d \
  -p 8081:8080 \
  -e OLLAMA_BASE_URL=http://host.docker.internal:8080 \
  --name open-webui \
  ghcr.io/open-webui/open-webui:main
```

Linux (no `host.docker.internal` by default):

```bash
docker run -d \
  -p 8081:8080 \
  -e OLLAMA_BASE_URL=http://172.17.0.1:8080 \
  --add-host=host.docker.internal:host-gateway \
  --name open-webui \
  ghcr.io/open-webui/open-webui:main
```

Open **`http://localhost:8081`**, create an admin account, select model **`edgequake:latest`**.

---

## Docker Compose (EdgeQuake + Open WebUI)

```yaml
# docker-compose.open-webui.yml — pin 0.23.0
services:
  postgres:
    image: ghcr.io/raphaelmansuy/edgequake-postgres:0.23.0
    environment:
      POSTGRES_USER: edgequake
      POSTGRES_PASSWORD: edgequake_secret
      POSTGRES_DB: edgequake
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U edgequake -d edgequake"]
      interval: 10s
      retries: 5

  api:
    image: ghcr.io/raphaelmansuy/edgequake:0.23.0
    ports:
      - "8080:8080"
    environment:
      DATABASE_URL: postgres://edgequake:edgequake_secret@postgres:5432/edgequake
      EDGEQUAKE_LLM_PROVIDER: ollama
      OLLAMA_HOST: http://host.docker.internal:11434
    extra_hosts:
      - "host.docker.internal:host-gateway"
    depends_on:
      postgres:
        condition: service_healthy

  frontend:
    image: ghcr.io/raphaelmansuy/edgequake-frontend:0.23.0
    ports:
      - "3000:3000"
    environment:
      EDGEQUAKE_API_URL: http://localhost:8080
    depends_on:
      api:
        condition: service_healthy

  open-webui:
    image: ghcr.io/open-webui/open-webui:main
    ports:
      - "8081:8080"
    environment:
      OLLAMA_BASE_URL: http://api:8080
    depends_on:
      - api

volumes:
  postgres_data:
```

```bash
docker compose -f docker-compose.open-webui.yml up -d
```

---

## Open WebUI settings

| Setting | Value |
| ------- | ----- |
| Ollama Base URL | `http://localhost:8080` (host) or `http://api:8080` (compose network) |
| Streaming | Enabled (recommended) |

---

## Query mode prefixes

Prefix messages to control retrieval (prefix stripped before query):

| Prefix | Mode |
| ------ | ---- |
| `/local` | Entity-focused |
| `/global` | Relationship / theme |
| `/naive` | Vector only |
| `/hybrid` | Default combined |
| `/mix` | Adaptive blend |
| `/bypass` | Skip RAG |

Example: `/local Who is mentioned in the contracts?`

---

## Ollama endpoints implemented

| Endpoint | Description |
| -------- | ----------- |
| `GET /api/version` | Version info |
| `GET /api/tags` | Lists `edgequake:latest` |
| `GET /api/ps` | Running models |
| `POST /api/generate` | Completion |
| `POST /api/chat` | Chat (streaming supported) |

```bash
curl -s http://localhost:8080/api/tags | jq '.models[].name'
curl -X POST http://localhost:8080/api/chat \
  -H "Content-Type: application/json" \
  -d '{"model":"edgequake:latest","messages":[{"role":"user","content":"Summarize the docs"}],"stream":false}'
```

---

## Troubleshooting

| Symptom | Fix |
| ------- | --- |
| Connection error in Open WebUI | `curl http://localhost:8080/health`; fix `OLLAMA_BASE_URL` |
| Port 3000 already in use | EdgeQuake WebUI owns `:3000` — run Open WebUI on `:8081` |
| Empty / generic answers | Upload docs; wait for `display_status: completed` |
| Slow first reply | Enable streaming; warm up Ollama |
| Wrong GHCR org | Use `ghcr.io/raphaelmansuy/edgequake*` — not `ghcr.io/edgequake/*` |

List documents:

```bash
curl -s "http://localhost:8080/api/v1/documents" \
  -H "X-Workspace-ID: default" | jq '.documents[] | {title, display_status}'
```

---

## Limitations

| Feature | Status |
| ------- | ------ |
| Chat + streaming | ✅ |
| Query mode prefixes | ✅ |
| Document upload via Open WebUI | ❌ Use API or EdgeQuake WebUI `:3000` |
| Model pull / create | ❌ N/A (single emulated model) |

---

## See also

- [Extended API — Ollama emulation](/docs/api-reference/extended-api/)
- [PDF Ingestion Tutorial](/docs/tutorials/pdf-ingestion/)
- [Quick Start](/docs/getting-started/quick-start/)
