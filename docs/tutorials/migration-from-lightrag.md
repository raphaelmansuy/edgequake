# Migration Guide: LightRAG Python → EdgeQuake Rust

> **Transitioning from LightRAG Python to EdgeQuake**

This guide helps teams migrate from the [LightRAG Python](https://github.com/HKUDS/LightRAG) implementation to EdgeQuake Rust.

---

## Overview

EdgeQuake is a **production-grade Rust implementation** of the LightRAG algorithm. It provides the same core functionality with significant improvements:

| Aspect       | LightRAG Python   | EdgeQuake Rust       |
| ------------ | ----------------- | -------------------- |
| Performance  | Baseline          | 10-50x faster        |
| Memory       | Higher (GC)       | Lower (no GC)        |
| Multi-tenant | Not built-in      | Native support       |
| Deployment   | Complex           | Single binary        |
| Storage      | Multiple backends | PostgreSQL optimized |
| API          | Class-based       | REST + WebSocket     |

---

## Architecture Comparison

```
┌─────────────────────────────────────────────────────────────────┐
│               LIGHTRAG PYTHON ARCHITECTURE                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  lightrag = LightRAG(                                           │
│      working_dir="./rag_storage",                               │
│      llm_model=gpt_4o_mini_complete,                            │
│      embedding_func=openai_embedding                            │
│  )                                                              │
│                                                                 │
│  lightrag.insert(document_text)  # Blocking                     │
│  result = lightrag.query(question, mode="hybrid")               │
│                                                                 │
│  Storage: JSON files in working_dir                             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

                              ↓ Migration ↓

┌─────────────────────────────────────────────────────────────────┐
│               EDGEQUAKE RUST ARCHITECTURE                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  # Start server                                                 │
│  DATABASE_URL="postgresql://..." \                              │
│  OPENAI_API_KEY="sk-..." \                                      │
│  edgequake                                                      │
│                                                                 │
│  # API calls                                                    │
│  POST /api/v1/documents  # Async processing                     │
│  POST /api/v1/query      # {"mode": "hybrid"}                   │
│                                                                 │
│  Storage: PostgreSQL with pgvector + Apache AGE                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Step-by-Step Migration

### Step 1: Set Up EdgeQuake

**Install**:

```bash
# Binary installation
curl -sSL https://edgequake.dev/install.sh | sh

# Or from source
git clone https://github.com/edgequake/edgequake.git
cd edgequake
cargo build --release
```

**Start Server**:

```bash
# With PostgreSQL
export DATABASE_URL="postgresql://user:pass@localhost:5432/edgequake"
export OPENAI_API_KEY="sk-your-key"
./target/release/edgequake

# Or with Docker
make dev
```

### Step 2: Create Workspace

LightRAG uses `working_dir`. EdgeQuake uses workspaces:

**LightRAG Python**:

```python
lightrag = LightRAG(working_dir="./my_project")
```

**EdgeQuake**:

```bash
# Create workspace (equivalent to working_dir)
curl -X POST http://localhost:8080/api/v1/tenants/default/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-project",
    "slug": "my-project",
    "llm_provider": "openai",
    "llm_model": "gpt-4o-mini"
  }'

# Returns workspace_id to use in subsequent requests
```

### Step 3: Migrate Documents

**LightRAG Python**:

```python
lightrag.insert("Your document text here...")
lightrag.insert(Path("document.txt").read_text())
```

**EdgeQuake**:

```bash
# Text content
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: your-workspace-id" \
  -d '{
    "content": "Your document text here...",
    "title": "Document Title"
  }'

# File upload
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -H "X-Workspace-ID: your-workspace-id" \
  -F "file=@document.txt" \
  -F "title=Document Title"

# Batch upload (new capability)
curl -X POST http://localhost:8080/api/v1/documents/upload/batch \
  -H "X-Workspace-ID: your-workspace-id" \
  -F "files=@doc1.pdf" \
  -F "files=@doc2.pdf"
```

### Step 4: Migrate Queries

**LightRAG Python**:

```python
# Query modes
result = lightrag.query("What is X?", mode="naive")
result = lightrag.query("Tell me about Y", mode="local")
result = lightrag.query("Summarize Z", mode="global")
result = lightrag.query("How does A relate to B?", mode="hybrid")
```

**EdgeQuake**:

```bash
# Same modes, REST API
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: your-workspace-id" \
  -d '{"query": "What is X?", "mode": "naive"}'

curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: your-workspace-id" \
  -d '{"query": "Tell me about Y", "mode": "local"}'

curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: your-workspace-id" \
  -d '{"query": "Summarize Z", "mode": "global"}'

curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: your-workspace-id" \
  -d '{"query": "How does A relate to B?", "mode": "hybrid"}'
```

### Step 5: Update Client Code

**LightRAG Python SDK**:

```python
from lightrag import LightRAG

rag = LightRAG(working_dir="./storage")
rag.insert(document)
result = rag.query("question", mode="hybrid")
print(result)
```

**EdgeQuake Python Client**:

```python
import requests

class EdgeQuakeClient:
    def __init__(self, base_url: str, workspace_id: str, api_key: str = None):
        self.base_url = base_url
        self.workspace_id = workspace_id
        self.headers = {"X-Workspace-ID": workspace_id}
        if api_key:
            self.headers["X-API-Key"] = api_key

    def insert(self, content: str, title: str = None):
        """Insert document (equivalent to LightRAG.insert)"""
        response = requests.post(
            f"{self.base_url}/api/v1/documents",
            json={"content": content, "title": title or "Untitled"},
            headers=self.headers
        )
        return response.json()

    def query(self, question: str, mode: str = "hybrid"):
        """Query (equivalent to LightRAG.query)"""
        response = requests.post(
            f"{self.base_url}/api/v1/query",
            json={"query": question, "mode": mode},
            headers=self.headers
        )
        return response.json()["answer"]

# Usage (drop-in replacement)
rag = EdgeQuakeClient("http://localhost:8080", "workspace-uuid")
rag.insert(document)
result = rag.query("question", mode="hybrid")
print(result)
```

---

## Configuration Mapping

### LLM Configuration

**LightRAG**:

```python
from lightrag.llm import gpt_4o_mini_complete, openai_embedding

lightrag = LightRAG(
    llm_model=gpt_4o_mini_complete,
    embedding_func=openai_embedding,
)
```

**EdgeQuake**:

```bash
# Environment variables
export OPENAI_API_KEY="sk-..."
export EDGEQUAKE_LLM_PROVIDER="openai"
export EDGEQUAKE_LLM_MODEL="gpt-4o-mini"
export EDGEQUAKE_EMBEDDING_MODEL="text-embedding-3-small"

# Or per-workspace via API
curl -X PUT http://localhost:8080/api/v1/workspaces/$WORKSPACE_ID \
  -H "Content-Type: application/json" \
  -d '{
    "llm_provider": "openai",
    "llm_model": "gpt-4o-mini",
    "embedding_model": "text-embedding-3-small"
  }'
```

### Storage Configuration

**LightRAG**:

```python
# File-based storage
lightrag = LightRAG(working_dir="./rag_storage")

# Neo4j (optional)
lightrag = LightRAG(
    working_dir="./rag_storage",
    graph_storage="neo4j",
    neo4j_uri="bolt://localhost:7687"
)
```

**EdgeQuake**:

```bash
# PostgreSQL (recommended)
export DATABASE_URL="postgresql://user:pass@localhost:5432/edgequake"

# In-memory (development only)
# Simply don't set DATABASE_URL
edgequake
```

---

## Feature Mapping

| LightRAG Feature             | EdgeQuake Equivalent               |
| ---------------------------- | ---------------------------------- |
| `LightRAG()` constructor     | `/api/v1/workspaces` POST          |
| `lightrag.insert(text)`      | `/api/v1/documents` POST           |
| `lightrag.insert_file(path)` | `/api/v1/documents/upload` POST    |
| `lightrag.query(q, mode)`    | `/api/v1/query` POST               |
| `working_dir`                | Workspace (multi-tenant)           |
| Entity extraction            | Same algorithm                     |
| Relationship extraction      | Same algorithm                     |
| Query modes                  | Same: naive, local, global, hybrid |
| Neo4j storage                | Apache AGE (PostgreSQL)            |

---

## Data Migration

### Export from LightRAG

```python
import json
import os

def export_lightrag(working_dir: str, output_dir: str):
    """Export LightRAG data for EdgeQuake import"""
    os.makedirs(output_dir, exist_ok=True)

    # Export documents
    docs_path = os.path.join(working_dir, "documents.json")
    if os.path.exists(docs_path):
        with open(docs_path) as f:
            docs = json.load(f)
        with open(os.path.join(output_dir, "documents.json"), "w") as f:
            json.dump(docs, f)

    # Export entities
    entities_path = os.path.join(working_dir, "entities.json")
    if os.path.exists(entities_path):
        with open(entities_path) as f:
            entities = json.load(f)
        with open(os.path.join(output_dir, "entities.json"), "w") as f:
            json.dump(entities, f)

    # Export relationships
    rels_path = os.path.join(working_dir, "relationships.json")
    if os.path.exists(rels_path):
        with open(rels_path) as f:
            rels = json.load(f)
        with open(os.path.join(output_dir, "relationships.json"), "w") as f:
            json.dump(rels, f)

export_lightrag("./rag_storage", "./export")
```

### Import to EdgeQuake

```bash
# Re-process documents (recommended for consistency)
# The extracted entities may differ slightly due to LLM variance

for doc in export/documents/*.txt; do
  curl -X POST http://localhost:8080/api/v1/documents \
    -H "X-Workspace-ID: $WORKSPACE_ID" \
    -F "file=@$doc"
done
```

---

## Query Response Differences

**LightRAG Python**:

```python
result = lightrag.query("What is X?", mode="hybrid")
# Returns: str (just the answer)
```

**EdgeQuake**:

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -d '{"query": "What is X?", "mode": "hybrid"}'
```

```json
{
  "answer": "X is...",
  "mode": "hybrid",
  "sources": [
    {
      "source_type": "chunk",
      "id": "chunk-uuid",
      "score": 0.89,
      "snippet": "...",
      "document_id": "doc-uuid"
    }
  ],
  "stats": {
    "total_time_ms": 2500,
    "tokens_used": 256
  }
}
```

**Benefit**: EdgeQuake provides sources and statistics for transparency.

---

## New Capabilities in EdgeQuake

Features available in EdgeQuake but not LightRAG Python:

| Feature             | Description                    |
| ------------------- | ------------------------------ |
| Multi-tenancy       | Isolated workspaces per tenant |
| REST API            | Standard HTTP interface        |
| Streaming           | SSE for real-time responses    |
| Chat history        | Conversation management        |
| Graph visualization | Real-time graph UI             |
| Cost tracking       | Token usage and costs          |
| Batch upload        | Multiple files at once         |
| Task queue          | Background processing          |
| Lineage             | Document-to-entity tracing     |
| Reranking           | Cross-encoder reranking        |

---

## Common Migration Issues

### Issue 1: Different Entity Extraction

LightRAG and EdgeQuake use the same algorithm, but LLM variance may cause different entities:

```
LightRAG:  JOHN_SMITH, SMITH_JOHN
EdgeQuake: JOHN_SMITH (normalized)
```

**Solution**: Re-process documents in EdgeQuake for consistency.

### Issue 2: Query Mode Differences

Both support same modes, but EdgeQuake adds:

- `mix` mode (adaptive blending)
- `bypass` mode (direct LLM, no RAG)

### Issue 3: Blocking vs Async

**LightRAG**: Blocking calls

```python
lightrag.insert(large_document)  # Blocks until complete
```

**EdgeQuake**: Async by default

```bash
# Returns immediately with task_id
curl -X POST http://localhost:8080/api/v1/documents \
  -d '{"content": "large document..."}'
# Response: {"task_id": "...", "status": "processing"}

# Check status
curl http://localhost:8080/api/v1/tasks/$TASK_ID
```

---

## Rollback Plan

If you need to keep LightRAG temporarily:

```python
class DualRAG:
    """Use both LightRAG and EdgeQuake during migration"""

    def __init__(self, lightrag, edgequake_client):
        self.lightrag = lightrag
        self.edgequake = edgequake_client
        self.use_edgequake = False  # Feature flag

    def query(self, question: str, mode: str = "hybrid"):
        if self.use_edgequake:
            return self.edgequake.query(question, mode)
        else:
            return self.lightrag.query(question, mode)
```

---

## Migration Checklist

### Pre-Migration

- [ ] EdgeQuake installed and running
- [ ] PostgreSQL database configured
- [ ] OpenAI API key set
- [ ] Workspace created

### Data Migration

- [ ] Documents exported from LightRAG
- [ ] Documents uploaded to EdgeQuake
- [ ] Processing completed (check task status)
- [ ] Entity counts compared

### Client Migration

- [ ] API calls updated to REST
- [ ] Authentication added if needed
- [ ] Error handling updated
- [ ] Response parsing updated

### Validation

- [ ] Sample queries return similar results
- [ ] Performance benchmarked
- [ ] Monitoring configured
- [ ] Rollback plan tested

---

## Getting Help

- **Documentation**: https://edgequake.dev/docs
- **GitHub Issues**: https://github.com/edgequake/edgequake/issues
- **Discord**: https://discord.gg/edgequake

---

## See Also

- [Installation Guide](../getting-started/installation.md)
- [Quick Start](../getting-started/quick-start.md)
- [API Reference](../api-reference/rest-api.md)
- [LightRAG Algorithm](../deep-dives/lightrag-algorithm.md)
