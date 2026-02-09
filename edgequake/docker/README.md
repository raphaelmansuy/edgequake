# EdgeQuake Docker Deployment

This directory contains Docker configuration for deploying EdgeQuake.

## ⚠️ Disclaimer

**PDF to Markdown Integration Status**: The PDF-to-Markdown feature is currently integrated in an **early prototype** stage. For testing and evaluating EdgeQuake's core functionality, please use **markdown documents** rather than PDFs. This ensures you can fully leverage the stable features of the system while we continue to refine the PDF extraction pipeline.

## Quick Start

```bash
# Build and start all services
docker-compose up -d

# View logs
docker-compose logs -f edgequake

# Stop services
docker-compose down
```

## Services

| Service     | Port | Description              |
| ----------- | ---- | ------------------------ |
| `edgequake` | 8080 | EdgeQuake API server     |
| `postgres`  | 5432 | PostgreSQL with pgvector |

## Environment Variables

Create a `.env` file:

```bash
# Required
OPENAI_API_KEY=sk-your-api-key

# Optional
EDGEQUAKE_PORT=8080
POSTGRES_PASSWORD=edgequake_secret
```

## Production Deployment

For production, use `docker-compose.prod.yml`:

```bash
docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d
```

## Building the Image

```bash
docker build -t edgequake:latest -f Dockerfile ..
```
