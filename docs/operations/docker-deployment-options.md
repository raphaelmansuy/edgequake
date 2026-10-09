---
title: "Docker deployment options"
description: "Choose between the quickstart stack, an API-only container, a prebuilt full stack, or a source build, and know what each needs before it starts."
---

# Docker deployment options

This page is for operators who must pick a Docker layout. For a five-minute demo use the [Docker quickstart](docker-quickstart.md). For Kubernetes or bare metal use [Deployment](deployment.md).

## Pick an option

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
  A["Need EdgeQuake in Docker"] --> B{"Have your own PostgreSQL?"}
  B -->|Yes| C["Option A: API only"]
  B -->|No| D{"Build from source?"}
  D -->|Yes| E["Option C: source build"]
  D -->|No| F{"Just trying it?"}
  F -->|Yes| G["Quickstart file at repo root"]
  F -->|No| H["Option B: prebuilt stack"]
%% eq-classes
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class B eqStore
```

How to read it: the first question decides whether you need a database container. Only Option A skips PostgreSQL. All options use the same API image.

| Option | File | Starts | Runs migrate for you? |
|--------|------|--------|-----------------------|
| Quickstart | `docker-compose.quickstart.yml` (repo root) | postgres, migrate, api, frontend | Yes (`migrate` service). |
| A. API only | `edgequake/docker/docker-compose.api-only.yml` | api | **No.** Run `migrate` yourself. |
| B. Prebuilt stack | `edgequake/docker/docker-compose.prebuilt.yml` | postgres, migrate, edgequake, frontend | Yes. |
| C. Source build | `edgequake/docker/docker-compose.yml` | same as B, built locally; optional Jaeger (`observability` profile) | Yes. |

Whatever you pick, the API refuses to start without a safe auth setup (see the warning below).

## Required before the API starts

The API checks its security settings at boot ([details](runtime-auth-hardening.md#what-the-api-checks-at-startup)). The three files under `edgequake/docker/` do not pass `EDGEQUAKE_DEV_MODE`, `JWT_SECRET` or `EDGEQUAKE_CORS_ORIGINS` into the container. With default settings the API exits with "JWT_SECRET is the insecure default". Fix it with an override file next to the compose file.

For a local demo:

```yaml
# docker-compose.override.yml
services:
  edgequake:
    environment:
      EDGEQUAKE_DEV_MODE: "true"   # open API, local only
```

For a real deployment, set `EDGEQUAKE_DEV_MODE: "false"`, `JWT_SECRET`, `EDGEQUAKE_CORS_ORIGINS`, and the bootstrap admin password (see [Enable login](auth-quickstart.md)). Docker Compose picks up `docker-compose.override.yml` automatically. With `-f` you must list it: `-f docker-compose.prebuilt.yml -f docker-compose.override.yml`.

## Option A: API only (your own PostgreSQL)

Your database needs the `vector` (pgvector 0.8.5 or newer) and `age` (Apache AGE) extensions. Run migrate once, then start the API.

```bash
IMAGE=ghcr.io/raphaelmansuy/edgequake:0.32.2
DB="postgres://user:pass@your-db:5432/edgequake"

# 1. Apply the schema (one-shot)
docker run --rm -e DATABASE_URL="$DB" \
  -e EDGEQUAKE_LLM_PROVIDER=mock -e EDGEQUAKE_EMBEDDING_PROVIDER=mock -e EDGEQUAKE_ALLOW_MOCK_PROVIDER=1 \
  "$IMAGE" migrate

# 2. Start the API
docker run -d --name edgequake -p 8080:8080 \
  -e DATABASE_URL="$DB" \
  -e EDGEQUAKE_LLM_PROVIDER=openai -e OPENAI_API_KEY="sk-..." \
  -e EDGEQUAKE_AUTH_ENABLED=true -e EDGEQUAKE_DEV_MODE=false \
  -e JWT_SECRET="$(openssl rand -hex 32)" \
  -e EDGEQUAKE_CORS_ORIGINS="https://app.example.com" \
  -e EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD='a-long-unique-password' \
  "$IMAGE"
```

The mock provider variables only let the one-shot migrate container start without LLM keys. They are the same ones the quickstart `migrate` service uses. Without step 1 the API exits with code 78 (schema pending). Set `EDGEQUAKE_SCHEMA_GATE=wait` if you run migrate in parallel from another job. The Compose version is `make docker-api-only`, which reads `edgequake/docker/.env`.

## Option B: prebuilt full stack

```bash
cd edgequake/docker
cp .env.example .env            # then edit it
docker compose -f docker-compose.prebuilt.yml up -d
```

| Service | Host port | Image |
|---------|-----------|-------|
| API | 8080 | `ghcr.io/raphaelmansuy/edgequake:X.Y.Z` |
| Frontend | 3000 | `ghcr.io/raphaelmansuy/edgequake-frontend:X.Y.Z` |
| PostgreSQL | 5432 | `ghcr.io/raphaelmansuy/edgequake-postgres:X.Y.Z` (PG18 by default) |

Unlike the quickstart, these ports bind to all interfaces. Do not expose 5432 beyond a trusted network. `make docker-prebuilt` is the Makefile shortcut.

PostgreSQL tags (multi-arch):

| Tag | PostgreSQL |
|-----|------------|
| `X.Y.Z`, `latest`, `X.Y.Z-pg18`, `latest-pg18` | 18 |
| `X.Y.Z-pg17`, `latest-pg17` | 17 |
| `X.Y.Z-pg16`, `latest-pg16` | 16 |

```bash
# Pin the stack to a release and the PostgreSQL major
EDGEQUAKE_VERSION=0.32.2 EDGEQUAKE_POSTGRES_TAG=0.32.2-pg16 \
  docker compose -f docker-compose.prebuilt.yml up -d
```

## Option C: build from source

```bash
cd edgequake/docker && docker compose up -d      # or: make docker-up
```

This builds the API and web UI locally. It sets a 4 GB memory cap on the API (`EDGEQUAKE_MEM_LIMIT`). PostgreSQL data uses one volume per major (`postgres-data-pg16`, `-pg17`, `-pg18`). Jaeger starts only with `--profile observability`.

## Settings that matter most

This is a short list. Every variable and default is in [Configuration](configuration.md) and the generated [env reference](env-reference.md).

| Variable | Default in the compose files | Purpose |
|----------|------------------------------|---------|
| `EDGEQUAKE_LLM_PROVIDER` | `ollama` | `openai`, `anthropic`, `gemini`, `mistral`, `ollama`, `azure`, `vertexai`, ... See [Providers](../providers/index.md). |
| `EDGEQUAKE_EMBEDDING_PROVIDER` | follows the LLM | Use a different embedding provider (hybrid mode). |
| `EDGEQUAKE_MODELS_CONFIG` | none | Path to a custom `models.toml`. |
| `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, `MISTRAL_API_KEY` | empty | Provider keys. |
| `OLLAMA_HOST` | `http://host.docker.internal:11434` | Ollama address from inside a container. |
| `EDGEQUAKE_VERSION` | `latest` | Image tag. Pin it. |
| `EDGEQUAKE_SCHEMA_GATE` | `wait` (Compose) | Wait for migrate instead of exit 78. |
| `RUST_LOG` | `info` | Log filter. |

### Vertex AI

`gemini` uses an API key (`GEMINI_API_KEY`). `vertexai` uses Google identity (ADC or a service account), not a key.

```bash
gcloud auth application-default login
export GOOGLE_CLOUD_PROJECT=your-gcp-project
export GOOGLE_CLOUD_REGION=europe-west1   # optional; default us-central1
```

If your `models.toml` omits `vertexai`, point at the bundled catalog: `export EDGEQUAKE_MODELS_CONFIG=edgequake/models.toml`. See [Configuration: Google Vertex AI](configuration.md#google-vertex-ai-enterprise).

`X.Y.Z` is a published release (see the [changelog](../../CHANGELOG.md)). `latest` tracks the newest release.
