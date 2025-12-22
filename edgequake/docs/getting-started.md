# Getting Started with EdgeQuake

EdgeQuake is a high-performance Retrieval-Augmented Generation (RAG) system built in Rust. This guide will help you get up and running quickly.

## Prerequisites

- **Rust 1.78+** - Install via [rustup](https://rustup.rs/)
- **OpenAI API Key** - For LLM and embedding operations
- **PostgreSQL (optional)** - For production deployments

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/your-org/edgequake.git
cd edgequake/edgequake

# Build the project
cargo build --release

# Run tests to verify installation
cargo test --all
```

### Using Docker

```bash
docker pull edgequake/edgequake:latest
docker run -p 8080:8080 -e OPENAI_API_KEY=your_key edgequake/edgequake
```

## Quick Start

### 1. Configure Environment

Create a `.env` file with your configuration:

```bash
# Required
OPENAI_API_KEY=sk-your-openai-api-key

# Optional - defaults shown
EDGEQUAKE_HOST=127.0.0.1
EDGEQUAKE_PORT=8080
EDGEQUAKE_CHUNK_SIZE=1200
EDGEQUAKE_CHUNK_OVERLAP=100
EDGEQUAKE_MAX_DOCUMENT_SIZE=10485760
```

### 2. Start the Server

```bash
# Development mode
cargo run

# Production mode
cargo run --release
```

The API will be available at `http://localhost:8080`.

### 3. Upload a Document

```bash
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Albert Einstein developed the theory of relativity. He was born in Germany in 1879 and later became a US citizen. Einstein received the Nobel Prize in Physics in 1921 for his explanation of the photoelectric effect.",
    "title": "Einstein Biography"
  }'
```

**Response:**

```json
{
  "document_id": "abc123-...",
  "status": "processed",
  "chunk_count": 1,
  "entity_count": 3,
  "relationship_count": 2
}
```

### 4. Query the Knowledge Graph

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What did Einstein win the Nobel Prize for?",
    "mode": "hybrid"
  }'
```

**Response:**

```json
{
  "answer": "Einstein received the Nobel Prize in Physics in 1921 for his explanation of the photoelectric effect.",
  "mode": "hybrid",
  "sources": [
    {
      "source_type": "entity",
      "id": "ALBERT EINSTEIN",
      "score": 0.95,
      "snippet": "Physicist who developed the theory of relativity..."
    }
  ],
  "stats": {
    "embedding_time_ms": 45,
    "retrieval_time_ms": 12,
    "generation_time_ms": 234,
    "total_time_ms": 291,
    "sources_retrieved": 3
  }
}
```

## Query Modes

EdgeQuake supports five query modes:

| Mode     | Description                      | Best For                    |
| -------- | -------------------------------- | --------------------------- |
| `naive`  | Direct chunk similarity search   | Simple factual queries      |
| `local`  | Entity-centric with neighborhood | Specific entity questions   |
| `global` | Community-based search           | Broad topic queries         |
| `hybrid` | Combines local + global          | Balanced queries (default)  |
| `mix`    | Weighted naive + graph           | Custom retrieval strategies |

### Example: Different Query Modes

```bash
# Naive mode - fast but less context
curl -X POST http://localhost:8080/api/v1/query \
  -d '{"query": "When was Einstein born?", "mode": "naive"}'

# Local mode - entity-focused
curl -X POST http://localhost:8080/api/v1/query \
  -d '{"query": "What did Einstein discover?", "mode": "local"}'

# Global mode - broader context
curl -X POST http://localhost:8080/api/v1/query \
  -d '{"query": "What are the key physics theories from the 20th century?", "mode": "global"}'
```

## Streaming Responses

For longer responses, use the streaming endpoint:

```bash
curl -X POST http://localhost:8080/api/v1/query/stream \
  -H "Content-Type: application/json" \
  -d '{"query": "Explain the theory of relativity"}' \
  --no-buffer
```

## Exploring the Knowledge Graph

### Get Graph Starting from an Entity

```bash
curl "http://localhost:8080/api/v1/graph?label=EINSTEIN&max_depth=2&max_nodes=50"
```

### Search for Entities

```bash
curl "http://localhost:8080/api/v1/graph/labels/search?query=ein&limit=10"
```

### Get Node Details

```bash
curl "http://localhost:8080/api/v1/graph/nodes/ALBERT%20EINSTEIN"
```

## Document Management

### List Documents

```bash
curl http://localhost:8080/api/v1/documents
```

### Get Document Details

```bash
curl http://localhost:8080/api/v1/documents/{document_id}
```

### Delete a Document

```bash
curl -X DELETE http://localhost:8080/api/v1/documents/{document_id}
```

## Health Checks

```bash
# Liveness - is the server running?
curl http://localhost:8080/live

# Readiness - is the server ready to accept requests?
curl http://localhost:8080/ready

# Health - detailed health status
curl http://localhost:8080/health
```

## API Documentation

Once the server is running, access the OpenAPI documentation at:

- **Swagger UI**: http://localhost:8080/swagger-ui/
- **OpenAPI JSON**: http://localhost:8080/api-docs/openapi.json

## Next Steps

- **[Configuration Reference](configuration.md)** - Detailed configuration options
- **[Query Modes Tutorial](query-modes.md)** - Deep dive into query strategies
- **[Examples](../examples/)** - Code examples in Rust
- **[Architecture Overview](../../docs_retro/02-architecture.md)** - System design

## Troubleshooting

### Common Issues

**"LLM API error"**

- Verify your `OPENAI_API_KEY` is set correctly
- Check your API quota

**"Storage initialization failed"**

- For PostgreSQL: Verify connection string
- Check that required extensions (pgvector, Apache AGE) are installed

**"Rate limit exceeded"**

- EdgeQuake includes automatic retry with backoff
- Consider increasing `llm_model_max_async` for parallel requests

### Getting Help

- [GitHub Issues](https://github.com/your-org/edgequake/issues)
- [Documentation](../docs_retro/)
