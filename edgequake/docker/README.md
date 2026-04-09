# EdgeQuake Docker Deployment

This directory ships two supported Docker flows:

- `docker-compose.prebuilt.yml`: pull versioned GHCR images for API, frontend, and PostgreSQL
- `docker-compose.yml`: build the API and frontend locally, then run the PostgreSQL image locally

## Prebuilt Flow

Use this when you want the fastest install path and a repeatable release version.

```bash
cd edgequake/docker
docker compose -f docker-compose.prebuilt.yml up -d
```

Pin a specific release:

```bash
EDGEQUAKE_VERSION=0.9.18 docker compose -f docker-compose.prebuilt.yml up -d
```

Use OpenAI:

```bash
EDGEQUAKE_LLM_PROVIDER=openai \
OPENAI_API_KEY=sk-... \
docker compose -f docker-compose.prebuilt.yml up -d
```

## Source-Build Flow

Use this when you are changing the backend or frontend locally and want Docker to rebuild them.

```bash
cd edgequake/docker
docker compose -f docker-compose.yml up -d --build
```

## Provider Configuration

Canonical EdgeQuake names:

```bash
EDGEQUAKE_LLM_PROVIDER=openai
EDGEQUAKE_LLM_MODEL=gpt-5-mini
EDGEQUAKE_EMBEDDING_PROVIDER=openai
EDGEQUAKE_EMBEDDING_MODEL=text-embedding-3-small
```

Compatibility aliases for migration from LightRAG-style env files:

```bash
MODEL_PROVIDER=openai
CHAT_MODEL=gpt-5-mini
EMBEDDING_PROVIDER=openai
EMBEDDING_MODEL=text-embedding-3-small
```

Canonical `EDGEQUAKE_*` variables take precedence when both are set.

## Services

| Service | Port | Description |
| --- | --- | --- |
| `edgequake` | `8080` | EdgeQuake API server |
| `frontend` | `3000` | Next.js web UI |
| `postgres` | `5432` | PostgreSQL with `pgvector` and Apache AGE |

## Common Commands

```bash
# Logs
docker compose -f docker-compose.prebuilt.yml logs -f

# Stop
docker compose -f docker-compose.prebuilt.yml down

# Stop and remove data
docker compose -f docker-compose.prebuilt.yml down -v
```
