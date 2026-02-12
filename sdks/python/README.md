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

## 📍 Resource Namespaces

The Python SDK provides access to 20+ resource namespaces:

| Namespace       | Description                                | Example                                          |
| --------------- | ------------------------------------------ | ------------------------------------------------ |
| `documents`     | Document upload, retrieval, and management | `client.documents.upload(content="...")`         |
| `query`         | RAG query execution (sync and streaming)   | `client.query.execute(query="...")`              |
| `graph`         | Knowledge graph exploration and traversal  | `client.graph.get()`                             |
| `chat`          | OpenAI-compatible chat completions         | `client.chat.completions(messages=[...])`        |
| `conversations` | Multi-turn conversation management         | `client.conversations.create(title="...")`       |
| `auth`          | Authentication (login, JWT refresh)        | `client.auth.login(email="...", password="...")` |
| `operations`    | Long-running operation tracking            | `client.operations.list()`                       |
| `tenants`       | Multi-tenant management (admin only)       | `client.tenants.create(name="...")`              |
| `workspaces`    | Workspace management                       | `client.workspaces.stats(workspace_id="...")`    |

**API Coverage:** 100+ endpoints across 9 core resources

For complete API reference, see [`docs/API.md`](docs/API.md).

## ⚙️ Configuration

Configure the client with these options:

```python
from edgequake import EdgeQuake

client = EdgeQuake(
    api_key="YOUR_API_KEY",                # Required: API authentication key
    base_url="http://localhost:8080",      # Server URL (default: http://localhost:8080)
    timeout=30,                            # Request timeout in seconds (default: 30)
    max_retries=3,                         # Retry attempts on failure (default: 3)
    workspace_id="my-workspace",           # Optional: Default workspace context
    tenant_id="my-tenant",                 # Optional: Multi-tenant context
)
```

**Configuration Parameters:**

| Parameter      | Type  | Default                 | Description                           |
| -------------- | ----- | ----------------------- | ------------------------------------- |
| `api_key`      | `str` | -                       | API key for authentication (required) |
| `base_url`     | `str` | `http://localhost:8080` | EdgeQuake server URL                  |
| `timeout`      | `int` | `30`                    | Request timeout in seconds            |
| `max_retries`  | `int` | `3`                     | Number of retry attempts on failure   |
| `workspace_id` | `str` | `None`                  | Default workspace for multi-tenancy   |
| `tenant_id`    | `str` | `None`                  | Tenant ID for multi-tenant setup      |

## 🌍 Environment Variables

The SDK reads these environment variables automatically:

| Variable                 | Purpose                                      | Example                             |
| ------------------------ | -------------------------------------------- | ----------------------------------- |
| `EDGEQUAKE_API_KEY`      | API authentication key (overrides parameter) | `sk-your-api-key-here`              |
| `EDGEQUAKE_BASE_URL`     | Server URL                                   | `http://localhost:8080`             |
| `EDGEQUAKE_URL`          | Alternative to `EDGEQUAKE_BASE_URL`          | `https://api.edgequake.example.com` |
| `EDGEQUAKE_WORKSPACE_ID` | Default workspace ID                         | `my-workspace`                      |
| `EDGEQUAKE_TENANT_ID`    | Tenant ID for multi-tenancy                  | `tenant-123`                        |
| `EDGEQUAKE_TIMEOUT`      | Request timeout in seconds                   | `60`                                |

**Example:**

```bash
export EDGEQUAKE_API_KEY="your-api-key"
export EDGEQUAKE_BASE_URL="http://localhost:8080"
python your_script.py
```

Then in your code:

```python
from edgequake import EdgeQuake

# Reads from environment variables automatically
client = EdgeQuake()
```

## 💡 Examples

See the [`examples/`](examples/) directory for complete, runnable examples:

| Example                                                 | Description                                            |
| ------------------------------------------------------- | ------------------------------------------------------ |
| [`basic_usage.py`](examples/basic_usage.py)             | Hello world — client setup, health check, simple query |
| [`document_upload.py`](examples/document_upload.py)     | Document management (upload, track, list, delete)      |
| [`graph_exploration.py`](examples/graph_exploration.py) | Navigate the knowledge graph                           |
| [`query_demo.py`](examples/query_demo.py)               | Different query modes (simple, hybrid, chat)           |
| [`streaming_query.py`](examples/streaming_query.py)     | Real-time streaming responses (SSE)                    |
| [`error_handling.py`](examples/error_handling.py)       | Graceful error handling patterns                       |
| [`configuration.py`](examples/configuration.py)         | Advanced configuration and multi-environment setup     |
| [`multi_tenant.py`](examples/multi_tenant.py)           | Tenant and workspace management                        |

**Run any example:**

```bash
export EDGEQUAKE_API_KEY="your-key"
python examples/basic_usage.py
```

For detailed example descriptions, see [`examples/README.md`](examples/README.md).

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

## 🔧 Troubleshooting

### Connection Errors

**Problem:** `ConnectionError: [Errno 61] Connection refused`  
**Solution:** Ensure EdgeQuake server is running on `base_url`

```bash
# Check if server is reachable
curl http://localhost:8080/health
```

### Authentication Errors

**Problem:** `401 Unauthorized`  
**Solution:** Verify that `EDGEQUAKE_API_KEY` is set correctly

```bash
echo $EDGEQUAKE_API_KEY  # Should print your API key
```

### Timeout Errors

**Problem:** `ReadTimeout: HTTPSConnectionPool`  
**Solution:** Increase timeout for long-running operations:

```python
client = EdgeQuake(
    api_key="your-key",
    timeout=60  # Increase to 60 seconds
)
```

### Streaming Issues

**Problem:** SSE connection drops or no output  
**Solution:**

1. Ensure output is flushed: `print(chunk, end="", flush=True)`
2. Check network stability
3. See [`docs/STREAMING.md`](docs/STREAMING.md) for reconnection strategies

### Import Errors

**Problem:** `ModuleNotFoundError: No module named 'edgequake'`  
**Solution:** Install the SDK:

```bash
pip install edgequake-sdk
```

### Empty Query Results

**Problem:** Queries return empty or "I don't know" responses  
**Solution:** Upload and process documents first:

```python
doc = client.documents.upload(content="...", title="...")
# Wait for processing to complete (check track_id status)
```

For more troubleshooting, see:

- **API Reference:** [`docs/API.md`](docs/API.md)
- **Authentication Guide:** [`docs/AUTHENTICATION.md`](docs/AUTHENTICATION.md)
- **Streaming Guide:** [`docs/STREAMING.md`](docs/STREAMING.md)

## 📚 Documentation

- **API Reference:** [`docs/API.md`](docs/API.md) — Complete API documentation
- **Authentication:** [`docs/AUTHENTICATION.md`](docs/AUTHENTICATION.md) — API key, JWT, multi-tenant auth
- **Streaming:** [`docs/STREAMING.md`](docs/STREAMING.md) — SSE streaming guide
- **Examples:** [`examples/README.md`](examples/README.md) — Runnable code examples
- **Changelog:** [`CHANGELOG.md`](CHANGELOG.md) — Version history

## License

Apache License 2.0
