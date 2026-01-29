# EdgeQuake Cookbook

> **Practical Recipes for Common Tasks**

This cookbook provides copy-paste solutions for common EdgeQuake operations. Each recipe includes complete code examples and expected outputs.

---

## Document Operations

### Recipe: Upload a PDF and Wait for Processing

```bash
#!/bin/bash
# Upload and poll for completion

# Upload the document
RESPONSE=$(curl -s -X POST http://localhost:8080/api/v1/documents/upload \
  -H "X-Workspace-ID: default" \
  -F "file=@document.pdf")

DOC_ID=$(echo $RESPONSE | jq -r '.id')
echo "Uploaded document: $DOC_ID"

# Poll for completion
while true; do
  STATUS=$(curl -s "http://localhost:8080/api/v1/documents/$DOC_ID" \
    -H "X-Workspace-ID: default" | jq -r '.status')

  echo "Status: $STATUS"

  if [ "$STATUS" = "completed" ]; then
    echo "Document processed successfully!"
    break
  elif [ "$STATUS" = "failed" ]; then
    echo "Document processing failed!"
    exit 1
  fi

  sleep 2
done
```

---

### Recipe: Bulk Upload Multiple Files

```bash
#!/bin/bash
# Upload all PDFs in a directory

WORKSPACE="default"
DIR="./documents"

for file in "$DIR"/*.pdf; do
  if [ -f "$file" ]; then
    echo "Uploading: $file"
    curl -s -X POST http://localhost:8080/api/v1/documents/upload \
      -H "X-Workspace-ID: $WORKSPACE" \
      -F "file=@$file" | jq '.id, .name'
    echo "---"
  fi
done

echo "All files uploaded!"
```

---

### Recipe: Upload Text Content Directly

```bash
# Upload raw text without a file
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: default" \
  -d '{
    "content": "EdgeQuake is a Graph-RAG framework built in Rust...",
    "name": "about-edgequake.txt",
    "metadata": {
      "source": "manual",
      "category": "documentation"
    }
  }'
```

---

### Recipe: Delete All Documents in a Workspace

```bash
#!/bin/bash
# Clean up a workspace (destructive!)

WORKSPACE="test-workspace"

# Get all document IDs
DOC_IDS=$(curl -s "http://localhost:8080/api/v1/documents?workspace_id=$WORKSPACE" \
  -H "X-Workspace-ID: $WORKSPACE" | jq -r '.documents[].id')

# Delete each document
for id in $DOC_IDS; do
  echo "Deleting: $id"
  curl -s -X DELETE "http://localhost:8080/api/v1/documents/$id" \
    -H "X-Workspace-ID: $WORKSPACE"
done

echo "Workspace cleaned!"
```

---

## Query Operations

### Recipe: Simple Query with Sources

```bash
# Query with source citations
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: default" \
  -d '{
    "query": "What are the main topics discussed?",
    "mode": "hybrid",
    "top_k": 5,
    "include_sources": true
  }' | jq '{
    answer: .answer,
    sources: [.chunks[] | {doc: .document_id, score: .score}]
  }'
```

---

### Recipe: Compare Query Modes

```bash
#!/bin/bash
# Compare results across query modes

QUERY="Who is mentioned in the documents?"

for MODE in naive local global hybrid; do
  echo "=== Mode: $MODE ==="

  RESPONSE=$(curl -s -X POST http://localhost:8080/api/v1/query \
    -H "Content-Type: application/json" \
    -H "X-Workspace-ID: default" \
    -d "{\"query\": \"$QUERY\", \"mode\": \"$MODE\"}")

  echo "$RESPONSE" | jq '{
    mode: "'$MODE'",
    entities: .entities | length,
    chunks: .chunks | length,
    answer_preview: .answer[:100]
  }'
  echo ""
done
```

---

### Recipe: Streaming Chat Response

```python
# Python: Stream chat response
import requests
import json

def stream_chat(message, workspace_id="default"):
    response = requests.post(
        "http://localhost:8080/api/v1/chat/stream",
        json={
            "message": message,
            "workspace_id": workspace_id,
            "mode": "hybrid"
        },
        stream=True
    )

    for line in response.iter_lines():
        if line:
            if line.startswith(b"data: "):
                data = json.loads(line[6:])
                if "content" in data:
                    print(data["content"], end="", flush=True)
                if data.get("done"):
                    print()  # New line at end
                    break

# Usage
stream_chat("What are the key findings?")
```

---

### Recipe: Query with Entity Filter

```bash
# Find information about a specific entity
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: default" \
  -d '{
    "query": "What projects is John Smith working on?",
    "mode": "local",
    "entity_filter": ["JOHN_SMITH"],
    "top_k": 10
  }'
```

---

## Graph Operations

### Recipe: Export Knowledge Graph to JSON

```bash
# Export graph for visualization
curl -s "http://localhost:8080/api/v1/graph?limit=1000" \
  -H "X-Workspace-ID: default" | jq '{
    nodes: [.nodes[] | {id: .id, label: .label, type: .type}],
    edges: [.edges[] | {source: .source, target: .target, label: .label}]
  }' > graph_export.json

echo "Exported $(jq '.nodes | length' graph_export.json) nodes"
echo "Exported $(jq '.edges | length' graph_export.json) edges"
```

---

### Recipe: Find Entity Relationships

```bash
# Get all relationships for an entity
ENTITY="JOHN_SMITH"

curl -s "http://localhost:8080/api/v1/graph/entities/$ENTITY" \
  -H "X-Workspace-ID: default" | jq '{
    entity: .name,
    type: .entity_type,
    outgoing: [.relationships[] | select(.source == .name) | {to: .target, type: .relation_type}],
    incoming: [.relationships[] | select(.target == .name) | {from: .source, type: .relation_type}]
  }'
```

---

### Recipe: Find Path Between Entities

```bash
# Find connection path between two entities
curl -s "http://localhost:8080/api/v1/graph/path?from=ENTITY_A&to=ENTITY_B" \
  -H "X-Workspace-ID: default" | jq '.path'
```

---

## Workspace Operations

### Recipe: Create Workspace with Settings

```bash
# Create workspace with custom configuration
curl -X POST http://localhost:8080/api/v1/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Research Project",
    "description": "Documents for Q3 research",
    "settings": {
      "default_query_mode": "hybrid",
      "enable_gleaning": true,
      "gleaning_iterations": 2,
      "chunk_size": 1000
    }
  }'
```

---

### Recipe: List All Workspaces with Stats

```bash
# Get workspaces with document counts
curl -s http://localhost:8080/api/v1/workspaces | jq '.workspaces[] | {
  id: .id,
  name: .name,
  documents: .document_count,
  entities: .entity_count,
  created: .created_at[:10]
}'
```

---

## Python Client Recipes

### Recipe: Complete Python Client

```python
"""EdgeQuake Python Client with common operations."""

import requests
from typing import Generator, Optional, List

class EdgeQuakeClient:
    def __init__(self, base_url: str = "http://localhost:8080", workspace: str = "default"):
        self.base_url = base_url
        self.workspace = workspace
        self.headers = {"X-Workspace-ID": workspace}

    def upload_text(self, content: str, name: str) -> dict:
        """Upload text content."""
        return requests.post(
            f"{self.base_url}/api/v1/documents",
            json={"content": content, "name": name},
            headers=self.headers
        ).json()

    def upload_file(self, file_path: str) -> dict:
        """Upload a file."""
        with open(file_path, "rb") as f:
            return requests.post(
                f"{self.base_url}/api/v1/documents/upload",
                files={"file": f},
                headers=self.headers
            ).json()

    def query(self, question: str, mode: str = "hybrid") -> dict:
        """Execute a query."""
        return requests.post(
            f"{self.base_url}/api/v1/query",
            json={"query": question, "mode": mode},
            headers=self.headers
        ).json()

    def chat_stream(self, message: str) -> Generator[str, None, None]:
        """Stream chat response."""
        import json
        response = requests.post(
            f"{self.base_url}/api/v1/chat/stream",
            json={"message": message},
            headers=self.headers,
            stream=True
        )
        for line in response.iter_lines():
            if line and line.startswith(b"data: "):
                data = json.loads(line[6:])
                if "content" in data:
                    yield data["content"]

    def list_documents(self) -> List[dict]:
        """List all documents."""
        return requests.get(
            f"{self.base_url}/api/v1/documents",
            headers=self.headers
        ).json().get("documents", [])

    def get_graph(self, limit: int = 100) -> dict:
        """Get knowledge graph."""
        return requests.get(
            f"{self.base_url}/api/v1/graph?limit={limit}",
            headers=self.headers
        ).json()


# Usage example
if __name__ == "__main__":
    client = EdgeQuakeClient()

    # Upload a document
    doc = client.upload_text("EdgeQuake is great!", "test.txt")
    print(f"Uploaded: {doc['id']}")

    # Query
    result = client.query("What is EdgeQuake?")
    print(f"Answer: {result['answer']}")

    # Stream chat
    print("Streaming: ", end="")
    for chunk in client.chat_stream("Tell me more"):
        print(chunk, end="", flush=True)
    print()
```

---

## Monitoring Recipes

### Recipe: Check System Health

```bash
#!/bin/bash
# Health check script

echo "=== EdgeQuake Health Check ==="

# API health
API_STATUS=$(curl -s http://localhost:8080/health | jq -r '.status')
echo "API: $API_STATUS"

# Database connection
DB_STATUS=$(curl -s http://localhost:8080/health | jq -r '.database_connected')
echo "Database: $DB_STATUS"

# Document count
DOC_COUNT=$(curl -s http://localhost:8080/api/v1/documents \
  -H "X-Workspace-ID: default" | jq '.documents | length')
echo "Documents: $DOC_COUNT"

# Entity count
ENTITY_COUNT=$(curl -s "http://localhost:8080/api/v1/graph?limit=1" \
  -H "X-Workspace-ID: default" | jq '.total_nodes')
echo "Entities: $ENTITY_COUNT"

echo "=== Done ==="
```

---

### Recipe: Monitor Processing Costs

```bash
# Get cost tracking data
curl -s http://localhost:8080/api/v1/costs \
  -H "X-Workspace-ID: default" | jq '{
    total_input_tokens: .total_input_tokens,
    total_output_tokens: .total_output_tokens,
    estimated_cost_usd: .estimated_cost,
    by_operation: .breakdown
  }'
```

---

## Docker Recipes

### Recipe: Docker Compose for Development

```yaml
# docker-compose.dev.yml
version: "3.8"

services:
  edgequake:
    build: .
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgresql://edgequake:edgequake@postgres:5432/edgequake
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - RUST_LOG=info,edgequake=debug
    depends_on:
      postgres:
        condition: service_healthy
    volumes:
      - ./data:/app/data

  postgres:
    image: ghcr.io/edgequake/postgres-age:16
    environment:
      POSTGRES_USER: edgequake
      POSTGRES_PASSWORD: edgequake
      POSTGRES_DB: edgequake
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U edgequake"]
      interval: 5s
      timeout: 5s
      retries: 5

volumes:
  postgres_data:
```

Start with:

```bash
docker compose -f docker-compose.dev.yml up -d
```

---

### Recipe: Backup and Restore

```bash
#!/bin/bash
# Backup EdgeQuake data

BACKUP_DIR="./backups/$(date +%Y%m%d)"
mkdir -p "$BACKUP_DIR"

# Backup PostgreSQL
docker compose exec postgres pg_dump -U edgequake edgequake > "$BACKUP_DIR/db.sql"

# Backup uploaded files (if any)
cp -r ./data/uploads "$BACKUP_DIR/uploads" 2>/dev/null || true

echo "Backup complete: $BACKUP_DIR"
```

Restore:

```bash
#!/bin/bash
# Restore from backup

BACKUP_DIR="$1"
if [ -z "$BACKUP_DIR" ]; then
  echo "Usage: ./restore.sh <backup_dir>"
  exit 1
fi

# Restore database
docker compose exec -T postgres psql -U edgequake edgequake < "$BACKUP_DIR/db.sql"

echo "Restore complete!"
```

---

## Performance Recipes

### Recipe: Benchmark Query Performance

```bash
#!/bin/bash
# Simple query benchmark

QUERY="What are the main topics?"
ITERATIONS=10

echo "Benchmarking $ITERATIONS queries..."

total_time=0

for i in $(seq 1 $ITERATIONS); do
  start=$(date +%s%N)

  curl -s -X POST http://localhost:8080/api/v1/query \
    -H "Content-Type: application/json" \
    -H "X-Workspace-ID: default" \
    -d "{\"query\": \"$QUERY\", \"mode\": \"hybrid\"}" > /dev/null

  end=$(date +%s%N)
  duration=$(( (end - start) / 1000000 ))
  total_time=$((total_time + duration))

  echo "Query $i: ${duration}ms"
done

avg=$((total_time / ITERATIONS))
echo "---"
echo "Average: ${avg}ms"
```

---

## Troubleshooting Recipes

### Recipe: Debug Document Processing

```bash
# Check document processing details
DOC_ID="your-doc-id"

curl -s "http://localhost:8080/api/v1/documents/$DOC_ID" \
  -H "X-Workspace-ID: default" | jq '{
    id: .id,
    name: .name,
    status: .status,
    chunks: .chunk_count,
    entities: .entity_count,
    relationships: .relationship_count,
    processing_time_ms: .processing_time_ms,
    error: .error_message
  }'
```

---

### Recipe: Check for Empty Responses

```bash
#!/bin/bash
# Diagnose empty query responses

QUERY="$1"
if [ -z "$QUERY" ]; then
  QUERY="test query"
fi

echo "Testing query: $QUERY"
echo ""

# Check document count
DOC_COUNT=$(curl -s http://localhost:8080/api/v1/documents \
  -H "X-Workspace-ID: default" | jq '.documents | length')
echo "Documents in workspace: $DOC_COUNT"

# Check entity count
ENTITY_COUNT=$(curl -s "http://localhost:8080/api/v1/graph?limit=1" \
  -H "X-Workspace-ID: default" | jq '.total_nodes')
echo "Entities in graph: $ENTITY_COUNT"

# Try query
RESPONSE=$(curl -s -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: default" \
  -d "{\"query\": \"$QUERY\", \"mode\": \"hybrid\"}")

echo ""
echo "Query result:"
echo "$RESPONSE" | jq '{
  has_answer: (.answer | length > 0),
  chunks_found: .chunks | length,
  entities_found: .entities | length
}'
```

---

## See Also

- [REST API Reference](./api-reference/rest-api.md) - Complete API documentation
- [Troubleshooting](./troubleshooting/common-issues.md) - Common problems and solutions
- [Performance Tuning](./operations/performance-tuning.md) - Optimization guide
