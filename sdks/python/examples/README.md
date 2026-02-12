# EdgeQuake Python SDK Examples

This directory contains runnable examples demonstrating various features of the EdgeQuake Python SDK.

## Prerequisites

1. **EdgeQuake server running:** Most examples require a server at `http://localhost:8080`
2. **API key:** Set the `EDGEQUAKE_API_KEY` environment variable
3. **Python dependencies:** Install the SDK:
   ```bash
   pip install edgequake
   ```

## Quick Start

```bash
# Set your API key
export EDGEQUAKE_API_KEY="demo-key"

# Run any example
python examples/basic_usage.py
```

## Examples

### 1. Basic Usage

**File:** [`basic_usage.py`](basic_usage.py)  
**Purpose:** Simplest possible setup — client creation, health check, document upload, query

```bash
python examples/basic_usage.py
```

**What it demonstrates:**

- Creating an EdgequakeClient
- Checking server health
- Uploading a text document
- Running a simple query
- Retrieving graph statistics

---

### 2. Document Upload

**File:** [`document_upload.py`](document_upload.py)  
**Purpose:** Complete document management workflow

```bash
python examples/document_upload.py
```

**What it demonstrates:**

- Text document upload with metadata
- PDF file upload (if `sample.pdf` exists)
- Tracking asynchronous processing status
- Paginated document listing
- Getting document details
- Deleting documents

---

### 3. Graph Exploration

**File:** [`graph_exploration.py`](graph_exploration.py)  
**Purpose:** Navigate and query the knowledge graph

```bash
python examples/graph_exploration.py
```

**What it demonstrates:**

- Getting graph overview statistics
- Searching for entities by keyword
- Listing all entities
- Getting entity neighborhood (1-hop connections)
- Listing relationships (edges)
- Searching and retrieving popular labels

**Prerequisites:** Documents must be uploaded and processed first

---

### 4. Query Demo

**File:** [`query_demo.py`](query_demo.py)  
**Purpose:** Different query modes and RAG retrieval

```bash
python examples/query_demo.py
```

**What it demonstrates:**

- Simple query (default mode)
- Hybrid mode query (local + global retrieval)
- Chat completion (OpenAI-compatible API)

**Prerequisites:** Documents uploaded and indexed

---

### 5. Streaming Query

**File:** [`streaming_query.py`](streaming_query.py)  
**Purpose:** Real-time streaming responses via Server-Sent Events (SSE)

```bash
python examples/streaming_query.py
```

**What it demonstrates:**

- Streaming query results (token-by-token)
- Streaming chat completions

**Prerequisites:** Documents uploaded and indexed

---

### 6. Error Handling

**File:** [`error_handling.py`](error_handling.py)  
**Purpose:** Graceful error handling patterns

```bash
python examples/error_handling.py
```

**What it demonstrates:**

- Specific error type handling (NotFound, Unauthorized, RateLimited)
- Retry with exponential backoff
- Graceful degradation when backend unavailable
- Validation error details
- Generic catch-all error handling

**Prerequisites:** None (intentionally triggers errors)

---

### 7. Configuration

**File:** [`configuration.py`](configuration.py)  
**Purpose:** Different client configuration patterns

```bash
# Minimal (uses defaults)
python examples/configuration.py

# Environment-based
export EDGEQUAKE_BASE_URL="https://api.edgequake.example.com"
export EDGEQUAKE_WORKSPACE_ID="my-workspace"
python examples/configuration.py
```

**What it demonstrates:**

- Minimal client configuration
- Explicit configuration with all options
- Environment variable-based configuration
- Multi-tenant configuration
- Per-environment factory pattern
- Health check before use

**Prerequisites:** None

---

### 8. Multi-Tenant

**File:** [`multi_tenant.py`](multi_tenant.py)  
**Purpose:** Tenant and workspace management

```bash
python examples/multi_tenant.py
```

**What it demonstrates:**

- Creating tenants
- Creating workspaces within tenants
- Scoped client (tenant + workspace context)
- Listing workspaces
- Workspace statistics
- Cleanup (deleting tenants and workspaces)

**Prerequisites:** Admin API key

---

## Environment Variables

| Variable                 | Required | Purpose                      | Example                           |
| ------------------------ | -------- | ---------------------------- | --------------------------------- |
| `EDGEQUAKE_API_KEY`      | **Yes**  | API authentication           | `demo-key`, `sk-...`              |
| `EDGEQUAKE_URL`          | No       | Server URL                   | `http://localhost:8080` (default) |
| `EDGEQUAKE_BASE_URL`     | No       | Alternative to EDGEQUAKE_URL | Same as above                     |
| `EDGEQUAKE_WORKSPACE_ID` | No       | Default workspace            | `my-workspace`                    |

## Troubleshooting

### Connection Errors

**Problem:** `ConnectionError: [Errno 61] Connection refused`  
**Solution:** Ensure EdgeQuake server is running on the configured `base_url`

### Authentication Errors

**Problem:** `401 Unauthorized`  
**Solution:** Check that `EDGEQUAKE_API_KEY` is set correctly

### Import Errors

**Problem:** `ModuleNotFoundError: No module named 'edgequake'`  
**Solution:** Install the SDK: `pip install edgequake`

### Empty Responses

**Problem:** Queries return empty results  
**Solution:** Upload and process documents first (see `document_upload.py`)

## Next Steps

- **API Reference:** See [`docs/API.md`](../docs/API.md) for complete resource documentation
- **Authentication:** See [`docs/AUTHENTICATION.md`](../docs/AUTHENTICATION.md) for auth methods
- **Streaming:** See [`docs/STREAMING.md`](../docs/STREAMING.md) for SSE streaming guide
- **Main README:** See [`../README.md`](../README.md) for SDK overview

## Contributing

Found a bug in an example? Want to add a new one? Please open an issue or pull request!
