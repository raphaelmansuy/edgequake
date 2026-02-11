# EdgeQuake Python SDK

Official Python SDK for the [EdgeQuake](https://github.com/edgequake/edgequake) RAG API.

## Features

- **Dual API**: Synchronous (`EdgeQuake`) and asynchronous (`AsyncEdgeQuake`) clients
- **Type-safe**: Full Pydantic v2 models for all request/response types
- **Streaming**: SSE streaming for query and chat endpoints
- **Auto-pagination**: Transparent iteration over paginated results
- **Auth**: API key, JWT, and multi-tenant authentication
- **Retry**: Automatic exponential backoff on 429/503 errors

## Installation

```bash
pip install edgequake-sdk
```

For WebSocket support (async pipeline progress):

```bash
pip install edgequake-sdk[ws]
```

## Quick Start

### Sync Client

```python
from edgequake import EdgeQuake

client = EdgeQuake(
    base_url="http://localhost:8080",
    api_key="your-api-key",
)

# Check health
health = client.health()
print(f"Status: {health.status}")

# Upload a document
doc = client.documents.upload(
    content="EdgeQuake is an advanced RAG framework...",
    title="About EdgeQuake",
)
print(f"Document ID: {doc.document_id}")

# Query the knowledge graph
result = client.query.execute(query="What is EdgeQuake?")
print(result.answer)

# Stream a query
for event in client.query.stream(query="Explain RAG"):
    if event.chunk:
        print(event.chunk, end="", flush=True)
```

### Async Client

```python
import asyncio
from edgequake import AsyncEdgeQuake

async def main():
    async with AsyncEdgeQuake(
        base_url="http://localhost:8080",
        api_key="your-api-key",
    ) as client:
        health = await client.health()
        print(f"Status: {health.status}")

        result = await client.query.execute(query="What is EdgeQuake?")
        print(result.answer)

asyncio.run(main())
```

## Authentication

```python
# API Key (recommended for server-side)
client = EdgeQuake(base_url="...", api_key="your-key")

# JWT Bearer token
client = EdgeQuake(base_url="...", jwt="eyJhbGciOi...")

# Multi-tenant
client = EdgeQuake(
    base_url="...",
    api_key="your-key",
    tenant_id="tenant-abc",
    workspace_id="workspace-xyz",
)
```

## Requirements

- Python >= 3.10
- httpx >= 0.27
- pydantic >= 2.0

## License

Apache License 2.0
