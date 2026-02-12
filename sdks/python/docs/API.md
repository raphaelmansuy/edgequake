# EdgeQuake Python SDK - API Reference

Complete reference documentation for all API resources and methods in the EdgeQuake Python SDK.

## Table of Contents

- [Client Initialization](#client-initialization)
- [Health Check](#health-check)
- [Documents](#documents)
- [Query](#query)
- [Graph](#graph)
- [Chat](#chat)
- [Conversations](#conversations)
- [Authentication](#authentication)
- [Operations](#operations)
- [Error Handling](#error-handling)
- [Pagination](#pagination)

---

## Client Initialization

### EdgequakeClient

Create a client instance to interact with the EdgeQuake API.

```python
from edgequake import EdgequakeClient

# Minimal configuration
client = EdgequakeClient()

# Full configuration
client = EdgequakeClient(
    api_key="your-api-key",           # Required: API authentication key
    base_url="http://localhost:8080",  # Server URL
    timeout=30,                        # Request timeout in seconds
    max_retries=3,                     # Number of retry attempts
    workspace_id="my-workspace",       # Default workspace context
    tenant_id="my-tenant",             # Multi-tenant context
)
```

**Parameters:**

- `api_key` (str): API key for authentication (can also use `EDGEQUAKE_API_KEY` env var)
- `base_url` (str, optional): EdgeQuake server URL (default: `http://localhost:8080`)
- `timeout` (int, optional): Request timeout in seconds (default: 30)
- `max_retries` (int, optional): Number of retry attempts on failure (default: 3)
- `workspace_id` (str, optional): Default workspace ID for multi-tenancy
- `tenant_id` (str, optional): Tenant ID for multi-tenant setup

---

## Health Check

### `client.health()`

Check the health status of the EdgeQuake backend.

```python
health = client.health()
print(health)
# {
#   "status": "healthy",
#   "version": "0.1.0",
#   "storage_mode": "postgresql",
#   "llm_provider_name": "ollama"
# }
```

**Returns:** Dictionary with health status information

---

## Documents

### `client.documents.upload(content, title=None, metadata=None)`

Upload a text document to EdgeQuake.

```python
doc = client.documents.upload(
    content="Knowledge graphs enhance RAG by providing structured context.",
    title="RAG Enhancement Whitepaper",
    metadata={"author": "EdgeQuake Team", "category": "research"}
)
print(doc["document_id"])
```

**Parameters:**

- `content` (str): Document content (text)
- `title` (str, optional): Document title
- `metadata` (dict, optional): Custom metadata key-value pairs

**Returns:** Dictionary with `document_id`, `status`, `track_id`

---

### `client.documents.pdf.upload(file, title=None, metadata=None)`

Upload a PDF document.

```python
with open("research.pdf", "rb") as f:
    doc = client.documents.pdf.upload(
        file=f.read(),
        title="Research Paper",
        metadata={"source": "arxiv"}
    )
```

**Parameters:**

- `file` (bytes): PDF file content
- `title` (str, optional): Document title
- `metadata` (dict, optional): Custom metadata

**Returns:** Dictionary with document details

---

### `client.documents.list(page=1, page_size=20, status=None)`

List all documents (paginated).

```python
result = client.documents.list(page=1, page_size=10)
for doc in result.get("items", []):
    print(f"{doc['id']}: {doc['title']} ({doc['status']})")
```

**Parameters:**

- `page` (int, optional): Page number (default: 1)
- `page_size` (int, optional): Items per page (default: 20, max: 100)
- `status` (str, optional): Filter by status (`uploading`, `processing`, `completed`, `failed`)

**Returns:** Dictionary with `items`, `total`, `page`, `page_size`, `pages`

---

### `client.documents.get(document_id)`

Get document details by ID.

```python
doc = client.documents.get("doc_123")
print(doc["title"], doc["status"], doc["chunk_count"])
```

**Parameters:**

- `document_id` (str): Document ID

**Returns:** Dictionary with full document details

**Raises:** `NotFoundError` if document doesn't exist

---

### `client.documents.delete(document_id)`

Delete a document.

```python
client.documents.delete("doc_123")
```

**Parameters:**

- `document_id` (str): Document ID

**Returns:** None

---

### `client.documents.get_track_status(track_id)`

Get async processing status for a document.

```python
status = client.documents.get_track_status("track_abc123")
print(status["status"])  # "processing", "completed", "failed"
```

**Parameters:**

- `track_id` (str): Track ID returned from upload

**Returns:** Dictionary with `status`, `message`, `progress`

---

## Query

### `client.query.execute(query, mode="hybrid", top_k=10, **kwargs)`

Execute a RAG query against the knowledge base.

```python
result = client.query.execute(
    query="What are knowledge graphs?",
    mode="hybrid",
    top_k=5
)
print(result["answer"])
```

**Parameters:**

- `query` (str): Natural language query
- `mode` (str, optional): Retrieval mode (`"simple"`, `"hybrid"`, `"local"`, `"global"`)
- `top_k` (int, optional): Number of results to retrieve (default: 10)

**Returns:** Dictionary with:

- `answer` (str): Generated answer
- `sources` (list): Source documents/entities used
- `context` (str): Retrieved context

---

### `client.query.stream(query, mode="hybrid", **kwargs)`

Execute a streaming query (returns tokens incrementally).

```python
import sys
for chunk in client.query.stream(query="Explain RAG"):
    if isinstance(chunk, dict) and "chunk" in chunk:
        sys.stdout.write(chunk["chunk"])
        sys.stdout.flush()
```

**Parameters:** Same as `execute()`

**Returns:** Iterator yielding response chunks

---

## Graph

### `client.graph.get()`

Get knowledge graph overview statistics.

```python
graph = client.graph.get()
print(graph)
# {
#   "node_count": 1234,
#   "edge_count": 5678,
#   "entity_types": ["PERSON", "ORGANIZATION", ...]
# }
```

**Returns:** Dictionary with graph statistics

---

### `client.graph.search_nodes(query, limit=20)`

Search for entities/nodes by keyword.

```python
nodes = client.graph.search_nodes(query="machine learning", limit=10)
for node in nodes:
    print(node["name"], node["entity_type"])
```

**Parameters:**

- `query` (str): Search keyword
- `limit` (int, optional): Max results (default: 20)

**Returns:** List of matching nodes

---

### `client.graph.entities.list(page=1, page_size=20)`

List all entities in the knowledge graph.

```python
entities = client.graph.entities.list()
for entity in entities.get("items", []):
    print(entity["name"], entity["description"])
```

**Parameters:**

- `page` (int, optional): Page number
- `page_size` (int, optional): Items per page

**Returns:** Dictionary with paginated entities

---

### `client.graph.entities.neighborhood(entity_name)`

Get the neighborhood (1-hop connections) for an entity.

```python
neighborhood = client.graph.entities.neighborhood("MACHINE_LEARNING")
print(neighborhood)
```

**Parameters:**

- `entity_name` (str): Entity name (normalized to UPPERCASE_WITH_UNDERSCORES)

**Returns:** Dictionary with entity and connected nodes/edges

---

### `client.graph.relationships.list(page=1, page_size=20)`

List all relationships (edges) in the graph.

```python
relationships = client.graph.relationships.list()
for rel in relationships.get("items", []):
    print(f"{rel['source_name']} --[{rel['relationship_type']}]--> {rel['target_name']}")
```

**Returns:** Dictionary with paginated relationships

---

### `client.graph.search_labels(query)`

Search for entity labels.

```python
labels = client.graph.search_labels(query="PER")
print(labels)
```

**Parameters:**

- `query` (str): Label search keyword

**Returns:** List of matching labels

---

### `client.graph.get_popular_labels(limit=10)`

Get most popular entity labels.

```python
popular = client.graph.get_popular_labels(limit=5)
print(popular)
```

**Parameters:**

- `limit` (int, optional): Max labels to return

**Returns:** List of popular labels with counts

---

## Chat

### `client.chat.completions(model, messages, **kwargs)`

OpenAI-compatible chat completion with RAG context injection.

```python
response = client.chat.completions(
    model="edgequake",
    messages=[
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "What is EdgeQuake?"}
    ]
)
print(response["choices"][0]["message"]["content"])
```

**Parameters:**

- `model` (str): Model name (use `"edgequake"`)
- `messages` (list): List of message dictionaries with `role` and `content`
- Additional OpenAI-compatible parameters

**Returns:** OpenAI-style completion response

---

### `client.chat.stream(model, messages, **kwargs)`

Streaming chat completion.

```python
import sys
for chunk in client.chat.stream(model="edgequake", messages=[...]):
    delta = chunk.get("choices", [{}])[0].get("delta", {}).get("content")
    if delta:
        sys.stdout.write(delta)
```

**Returns:** Iterator yielding OpenAI-style delta chunks

---

## Conversations

### `client.conversations.create(title=None)`

Create a new conversation thread.

```python
conversation = client.conversations.create(title="RAG Discussion")
print(conversation["id"])
```

**Returns:** Dictionary with conversation details

---

### `client.conversations.list()`

List all conversations.

```python
conversations = client.conversations.list()
for conv in conversations.get("items", []):
    print(conv["id"], conv["title"])
```

**Returns:** Dictionary with paginated conversations

---

## Authentication

### `client.auth.login(email, password)`

Authenticate with email/password (returns JWT).

```python
auth = client.auth.login(
    email="user@example.com",
    password="secret"
)
jwt_token = auth["access_token"]
```

**Returns:** Dictionary with `access_token`, `refresh_token`, `expires_in`

---

### `client.auth.refresh(refresh_token)`

Refresh an expired JWT token.

```python
new_auth = client.auth.refresh(refresh_token="...")
```

**Returns:** New access token

---

## Operations

### `client.operations.list()`

List long-running operations.

```python
ops = client.operations.list()
for op in ops.get("items", []):
    print(op["id"], op["status"], op["progress"])
```

**Returns:** Dictionary with paginated operations

---

## Error Handling

All SDK errors extend `EdgeQuakeError`:

```python
from edgequake.exceptions import (
    EdgeQuakeError,       # Base class for all errors
    NotFoundError,        # 404 errors
    UnauthorizedError,    # 401 errors
    ValidationError,      # 400/422 errors
    RateLimitedError,     # 429 errors
    NetworkError,         # Connection errors
    TimeoutError,         # Timeout errors
)

try:
    client.documents.get("invalid-id")
except NotFoundError:
    print("Document not found")
except EdgeQuakeError as e:
    print(f"API error: {e}")
```

**Error attributes:**

- `message` (str): Error description
- `status` (int, optional): HTTP status code
- `code` (str, optional): Error code

---

## Pagination

For paginated endpoints (`documents.list`, `graph.entities.list`, etc.):

```python
# Basic pagination
result = client.documents.list(page=1, page_size=20)
print(result["total"])     # Total items
print(result["page"])      # Current page
print(result["pages"])     # Total pages

# Iterate all pages
page = 1
while True:
    result = client.documents.list(page=page, page_size=100)
    for doc in result.get("items", []):
        print(doc["id"])
    if page >= result["pages"]:
        break
    page += 1
```

**Response structure:**

```python
{
    "items": [...],
    "total": 250,
    "page": 1,
    "page_size": 20,
    "pages": 13
}
```

---

## See Also

- **Authentication Guide:** [`AUTHENTICATION.md`](AUTHENTICATION.md)
- **Streaming Guide:** [`STREAMING.md`](STREAMING.md)
- **Examples:** [`examples/README.md`](../examples/README.md)
- **Main README:** [`../README.md`](../README.md)
