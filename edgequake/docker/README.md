# EdgeQuake Docker Deployment

This directory ships two supported Docker flows:

- `docker-compose.prebuilt.yml`: pull versioned GHCR images for API, frontend, and PostgreSQL
- `docker-compose.yml`: build the API and frontend locally, then run the PostgreSQL image locally

## PostgreSQL triple-track (SPEC-042)

EdgeQuake supports three PostgreSQL major tiers with a **single application binary**:

| Profile | PG | pgvector | AGE | Local build | GHCR tag |
| ------- | -- | -------- | --- | ----------- | -------- |
| `pg18` (default) | 18 | 0.8.3 | 1.7.0 | `Dockerfile.postgres.pg18` | `:latest` / `:VERSION` |
| `pg17` | 17 | 0.8.3 | 1.7.0 | `Dockerfile.postgres.pg17` | `:latest-pg17` |
| `pg16` (legacy) | 16 | 0.8.3 | 1.6.0 | `Dockerfile.postgres` | `:latest-pg16` |

**SSOT:** `extension-pins.sh` — all Dockerfiles, verify scripts, and Makefile targets source this file.

```bash
# Default (PG18 recommended)
make dev

# Explicit PostgreSQL major (recreates container + per-major volume when switching)
make dev-pg18   # same as make dev
make dev-pg17
make dev-pg16   # legacy

# Or override via env / .env
EQ_POSTGRES_PROFILE=pg16 make dev
make dev-bg-pg17   # background mode
```

# Prebuilt GHCR — PG18 default
docker compose -f docker-compose.prebuilt.yml up -d

# Prebuilt GHCR — stay on PG16
EDGEQUAKE_POSTGRES_TAG=latest-pg16 docker compose -f docker-compose.prebuilt.yml up -d
```

Migration guide: [postgres-triple-track-spec042.md](../docs/migrations/postgres-triple-track-spec042.md)

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
| `postgres` | `5432` | PostgreSQL (PG18 default) with `pgvector` 0.8.3, Apache AGE 1.7.0 |

## PostgreSQL Image (extensions)

Built from `Dockerfile.postgres.pg18` (default), `.pg17`, or `Dockerfile.postgres` (PG16):

| Extension | Version (PG18) | Purpose |
| --- | --- | --- |
| `vector` (pgvector) | 0.8.3 | embedding ANN + iterative scan |
| `age` (Apache AGE) | 1.7.0 | property graph / Cypher |
| `pg_trgm` | contrib | trigram fuzzy search |
| `btree_gin` | contrib | GIN btree operator classes |
| `uuid-ossp` | contrib | legacy UUID helpers |

Build and verify locally:

```bash
# PG18 (default dev profile)
make postgres-image-build-pg18

# All tiers + DRY pin check + battle test
make check-extension-pins
make postgres-image-build && make postgres-image-build-pg17 && make postgres-image-build-pg18
make postgres-battle-test
```

## Common Commands

```bash
# Logs
docker compose -f docker-compose.prebuilt.yml logs -f

# Stop
docker compose -f docker-compose.prebuilt.yml down

# Stop and remove data
docker compose -f docker-compose.prebuilt.yml down -v
```
