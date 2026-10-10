---
title: Installation
description: Install EdgeQuake with prebuilt Docker images or from source, set up PostgreSQL, apply the schema, and connect a model provider.
---

> **Released: v0.32.2** (schema 168) · Contract: [OpenAPI snapshot](../../edgequake_webui/openapi/openapi.snapshot.json) · Upgrades: [Upgrading](../operations/upgrading.md)

# Installation

This page shows every supported way to run EdgeQuake on one machine. It is for developers and operators. After you finish, go to the [Quick Start](quick-start.md).

## Pick a path

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
    A["Goal"] --> B["Try it"]
    A --> C["Develop"]
    A --> D["Deploy"]
    B --> E["Option 2: prebuilt images"]
    C --> F["Option 1: make dev"]
    C --> G["Option 3: backend only"]
    D --> H["Compose or Helm"]
    H --> I["Deployment guide"]
```

Start at your goal. Each leaf is a section below, except the last box, which links to the deployment guide.

| Path | Needs | API | UI |
|------|-------|-----|----|
| [Option 1: `make dev`](#option-1-full-stack-from-source-make-dev) | Docker, Rust, Node.js, pnpm | 8090 | 3010 |
| [Option 2: prebuilt images](#option-2-prebuilt-images-docker-only) | Docker | 8080 | 3000 |
| [Option 3: backend only](#option-3-backend-only) | Docker, Rust | 8090 | none |
| [Option 4: release binary](#option-4-release-binary-from-source) | PostgreSQL, Rust | 8080 | none |
| Production | See [Deployment](../operations/deployment.md) | your choice | your choice |

`make dev` picks the first free port at or above its default and prints the result. It also writes the ports to `.edgequake-dev-ports.env`.

## Prerequisites

| Tool | Version | Check | Needed for |
|------|---------|-------|------------|
| Docker | A current Docker Engine or Docker Desktop | `docker --version` | All options (PostgreSQL runs in a container) |
| Rust | 1.95 (pinned in `edgequake/rust-toolchain.toml`) | `rustc --version` | Options 1, 3, 4 |
| Node.js | A current LTS release | `node --version` | Option 1 |
| pnpm (or Bun) | pnpm 10 | `pnpm --version` | Option 1 |

Hardware: plan for enough disk for the container images and the database, and more RAM when you run local models.

### PostgreSQL

EdgeQuake stores everything in PostgreSQL 16, 17 or 18 with the **pgvector** and **Apache AGE** extensions. There is no in-memory mode, and the server refuses to start without `DATABASE_URL`.

| Profile | AGE | pgvector | How to select |
|---------|-----|----------|---------------|
| `pg18` (default) | 1.8.0 | 0.8.5 | `make dev` |
| `pg17` | 1.7.0 | 0.8.5 | `make dev-pg17` |
| `pg16` | 1.6.0 | 0.8.5 | `make dev-pg16` |

The Makefile starts a container with user `edgequake`, password `edgequake_secret` and database `edgequake`. A matching connection string looks like this:

```bash
export DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake?options=-c%20search_path%3Dpublic"
```

`make db-start` uses port 5432 when it is free and another port when it is not. `make dev` reads the real port for you.

## The schema is explicit

The API never changes the database schema. Only the `edgequake migrate` command does. `make dev`, `make migrate` and the Docker quickstart all run it for you. If you start the binary yourself, run it first.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    A["PostgreSQL up"] --> B["edgequake migrate"]
    B --> C["Schema current"]
    C --> D["edgequake serve"]
    D --> E["/ready returns 200"]
%% eq-classes
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class A eqStore
```

Read the chart left to right. If you skip `migrate`, the API either exits (the default) or waits and answers `/ready` with 503 (`EDGEQUAKE_SCHEMA_GATE=wait`, which Compose and Helm set). See [Upgrading](../operations/upgrading.md).

## Option 1: Full stack from source (`make dev`)

Use this when you want to read or change the code.

```bash
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake
make dev
```

`make dev` does these things in order:

1. Starts PostgreSQL (profile `pg18` unless you choose another).
2. Runs `edgequake migrate`.
3. Builds and starts the Rust API on port 8090 or the next free port.
4. Starts the Next.js UI on port 3010 or the next free port.
5. Picks OpenAI if `OPENAI_API_KEY` is set. Otherwise it picks Ollama with `gemma4:latest` and `embeddinggemma:latest`.

Auth is off by default (`EDGEQUAKE_DEV_MODE=true`). Use `make dev-auth` to turn it on.

Verify:

```bash
curl -s http://localhost:8090/health | jq '{status, storage_mode, llm_provider_name}'
```

Expected output:

```json
{
  "status": "healthy",
  "storage_mode": "postgresql",
  "llm_provider_name": "ollama"
}
```

Open the UI at <http://localhost:3010>. Swagger is at <http://localhost:8090/swagger-ui>.

Useful commands: `make status` shows what runs, `make stop` stops it, and `make dev-bg` runs everything in the background (logs in `/tmp/edgequake-backend.log` and `/tmp/edgequake-frontend.log`).

## Option 2: Prebuilt images (Docker only)

Use this to try EdgeQuake without installing Rust or Node.js. The images come from GitHub Container Registry.

```bash
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake
EDGEQUAKE_VERSION=0.32.2 docker compose -f docker-compose.quickstart.yml up -d
```

Or run `make stack`. Or run the interactive script, which also writes the secrets for you:

```bash
curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/quickstart.sh | sh
```

Compose starts four containers:

| Service | Image | Port |
|---------|-------|------|
| `postgres` | `ghcr.io/raphaelmansuy/edgequake-postgres:0.32.2` (PG18) | 5432 |
| `migrate` | `ghcr.io/raphaelmansuy/edgequake:0.32.2` with `migrate`; runs once and exits | none |
| `api` | `ghcr.io/raphaelmansuy/edgequake:0.32.2` | 8080 |
| `frontend` | `ghcr.io/raphaelmansuy/edgequake-frontend:0.32.2` | 3000 |

If you leave `EDGEQUAKE_VERSION` unset, Compose uses the `latest` tag. To pin the PostgreSQL major version, set `EDGEQUAKE_POSTGRES_TAG` to `0.32.2-pg16`, `0.32.2-pg17` or `0.32.2-pg18`.

The default Compose file sets `EDGEQUAKE_DEV_MODE=true`, so the API is open. It binds ports to `127.0.0.1` only. Do not expose it as is. By default the API calls Ollama on the host at `http://host.docker.internal:11434`.

Verify:

```bash
curl -s http://localhost:8080/health | jq '{status, llm_provider_name}'
docker compose -f docker-compose.quickstart.yml ps
```

Read API logs with `docker compose logs api`. For more detail see [Docker quickstart](../operations/docker-quickstart.md) and [Docker deployment options](../operations/docker-deployment-options.md).

## Option 3: Backend only

Use this when you work on the API and do not need the UI.

```bash
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake
make backend-bg
```

`make backend-bg` starts PostgreSQL if needed, sets `DATABASE_URL`, and starts the API in the background on port 8090. Check it with `curl http://localhost:8090/health`.

## Option 4: Release binary from source

Use this when you manage PostgreSQL yourself.

```bash
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake/edgequake
cargo build --release

export DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake?options=-c%20search_path%3Dpublic"
./target/release/edgequake migrate
./target/release/edgequake
```

The first command applies the schema and exits. The second starts the server on port 8080. For hot reload, run PostgreSQL with `make db-start`, run `make migrate`, then start the backend with `cargo watch -x run` in `edgequake/` and the UI with `pnpm dev` in `edgequake_webui/`.

The offline `sqlx` build is described in [sqlx offline mode](../sqlx-offline-mode.md).

## Connect a model provider

EdgeQuake needs a chat model and an embedding model. Pick a provider and set it before you start the stack. See [Providers](../providers/index.md) for every supported provider, and [Configuration](../operations/configuration.md) for all variables.

| Provider | Quick setup |
|----------|-------------|
| Ollama (local, free) | `ollama pull gemma4:latest && ollama pull embeddinggemma:latest && ollama serve` |
| OpenAI | `export OPENAI_API_KEY="sk-..."` then start the stack |
| Anthropic, LM Studio, oMLX, other servers | Follow the [provider guides](../providers/index.md) |
| Google Vertex AI | Uses GCP identity, not an API key. See [Vertex AI](../operations/configuration.md#google-vertex-ai-enterprise) |

To choose the provider explicitly, set `EDGEQUAKE_LLM_PROVIDER`, `EDGEQUAKE_LLM_MODEL`, `EDGEQUAKE_EMBEDDING_PROVIDER` and `EDGEQUAKE_EMBEDDING_MODEL`. Use different providers for chat and embeddings if you like.

Check what the server actually uses:

```bash
curl -s http://localhost:8090/api/v1/config/effective | jq '{llm: .llm.effective_provider, embedding: .embedding.effective_provider}'
```

Replace `8090` with `8080` for the Docker stack.

> SPEC-163 (on `main`, shipping in v0.33.0) adds stored provider Connections in the UI under **Settings**, with keys encrypted by `EDGEQUAKE_SECRETS_KEY`. See [Upgrade to v0.33.0](../operations/upgrade-to-0.33.0.md).

### Vision model for PDFs

By default, PDFs are converted page by page with a vision-capable model. The default parser backend is `vision`. Choose a model that accepts images:

```bash
# Cloud
export EDGEQUAKE_VISION_PROVIDER=openai
export EDGEQUAKE_VISION_MODEL=gpt-4.1-mini

# Local (Ollama)
ollama pull gemma4:latest
export EDGEQUAKE_VISION_PROVIDER=ollama
export EDGEQUAKE_VISION_MODEL=gemma4:latest
```

If you set nothing, EdgeQuake resolves the vision provider and model from several configuration levels (server config, inherited chat LLM settings and compiled defaults). A parser backend that needs no model, `edgeparse`, also exists. Check the result with `GET /api/v1/config/effective` and read the `vision` section.

## Authentication

| Mode | Auth | How |
|------|------|-----|
| `make dev`, Docker quickstart | Off (dev mode) | Nothing to do |
| `make dev-auth` | On | Starts the full stack with authentication enabled. Set credentials as in the production row |
| Production | On | Set `JWT_SECRET` (32 or more characters), an admin password, and `EDGEQUAKE_DEV_MODE=false`. Details in [Runtime auth hardening](../operations/runtime-auth-hardening.md) |

## Verify the install

```bash
# Server and provider status
curl -s http://localhost:8090/health | jq

# OpenAPI document
curl -s http://localhost:8090/api-docs/openapi.json | jq .info.title

# Local environment checks (v0.33.0 and later)
edgequake doctor

# Ollama, if you use it
curl -s http://localhost:11434/api/tags | jq '.models[].name'
```

In `/health`, `components` shows storage and provider status. `schema` shows the migration state. Use the Docker ports (8080 and 3000) if you chose option 2.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `DATABASE_URL not set`, server exits | Use `make dev`, or export `DATABASE_URL` as shown above |
| API exits at start, or `/ready` returns 503 | The schema is behind. Run `make migrate` or `edgequake migrate` |
| Port already in use | `lsof -i :8090` (API), `lsof -i :3010` (UI), `lsof -i :5432` (database); or run `make stop` |
| Docker not running | `docker info` |
| Change the API port | Set `PORT` (default 8080) and `HOST` (default 0.0.0.0) for the binary |
| Rust build fails on Linux | `sudo apt-get install pkg-config libssl-dev libpq-dev` |
| Ingest fails with a network error | Provider is down. Start it (`ollama serve`) and check `ollama list` |
| `401` on API calls | Auth is on. Send `Authorization: Bearer ...` or `X-API-Key`. Use dev mode only on your laptop |

Still stuck? See the [FAQ](../faq.md).

## Next steps

1. [Quick Start](quick-start.md): ingest a document and query it.
2. [Providers](../providers/index.md): choose and tune models.
3. [Architecture overview](../architecture/overview.md): how the pieces fit.
4. [REST API reference](../api-reference/rest-api.md)
