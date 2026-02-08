# EdgeQuake Configuration Reference

This document provides a complete reference for all EdgeQuake configuration options.

## Configuration Methods

EdgeQuake supports multiple configuration methods, applied in this order of precedence:

1. **Environment Variables** (highest priority)
2. **Configuration File** (TOML/JSON)
3. **Programmatic Defaults** (lowest priority)

---

## Quick Start

### Minimal Configuration

```bash
# Required for LLM operations
export OPENAI_API_KEY=sk-your-api-key

# Start with defaults
cargo run --release
```

### Full Environment Configuration

```bash
# .env file
OPENAI_API_KEY=sk-your-api-key
EDGEQUAKE_HOST=0.0.0.0
EDGEQUAKE_PORT=8080
EDGEQUAKE_DATABASE_URL=postgres://localhost:5432/edgequake
EDGEQUAKE_NAMESPACE=default
```

---

## Storage Configuration

Configuration for database connections and storage backends.

| Variable                 | Config Field                   | Type    | Default                               | Description                          |
| ------------------------ | ------------------------------ | ------- | ------------------------------------- | ------------------------------------ |
| `EDGEQUAKE_DATABASE_URL` | `storage.database_url`         | string  | `postgres://localhost:5432/edgequake` | PostgreSQL connection URL            |
| `EDGEQUAKE_NAMESPACE`    | `storage.namespace`            | string? | `null`                                | Namespace for multi-tenant isolation |
| -                        | `storage.max_connections`      | u32     | `10`                                  | Maximum pool connections             |
| -                        | `storage.min_connections`      | u32     | `1`                                   | Minimum pool connections             |
| -                        | `storage.connect_timeout_secs` | u64     | `30`                                  | Connection timeout (seconds)         |

### Database URL Format

```
postgres://[user[:password]@]host[:port]/database[?options]
```

**Examples:**

```bash
# Local development
EDGEQUAKE_DATABASE_URL=postgres://localhost:5432/edgequake

# With credentials
EDGEQUAKE_DATABASE_URL=postgres://myuser:mypass@localhost:5432/edgequake

# SSL required
EDGEQUAKE_DATABASE_URL=postgres://myuser:mypass@db.example.com:5432/edgequake?sslmode=require
```

### Multi-Tenancy

Set a namespace to isolate data between tenants:

```bash
EDGEQUAKE_NAMESPACE=tenant_acme_corp
```

This prefixes all storage keys and tables with the namespace.

---

## LLM Configuration

Configuration for the language model and embedding providers.

| Variable                    | Config Field          | Type    | Default                  | Description                |
| --------------------------- | --------------------- | ------- | ------------------------ | -------------------------- |
| `OPENAI_API_KEY`            | `llm.api_key`         | string? | `null`                   | OpenAI API key             |
| `EDGEQUAKE_LLM_PROVIDER`    | `llm.provider`        | string  | `openai`                 | LLM provider name          |
| `EDGEQUAKE_LLM_MODEL`       | `llm.model`           | string  | `gpt-5-nano`             | Model for completions      |
| `EDGEQUAKE_EMBEDDING_MODEL` | `llm.embedding_model` | string  | `text-embedding-3-small` | Model for embeddings       |
| `EDGEQUAKE_LLM_BASE_URL`    | `llm.base_url`        | string? | `null`                   | Custom API base URL        |
| -                           | `llm.embedding_dim`   | usize   | `1536`                   | Embedding vector dimension |
| -                           | `llm.max_tokens`      | usize   | `4096`                   | Max tokens per request     |
| -                           | `llm.temperature`     | f32     | `0.0`                    | Generation temperature     |
| -                           | `llm.timeout_secs`    | u64     | `60`                     | Request timeout            |
| -                           | `llm.max_retries`     | u32     | `3`                      | Retry count for failures   |

### Supported Providers

| Provider     | Value    | Notes                                                    |
| ------------ | -------- | -------------------------------------------------------- |
| OpenAI       | `openai` | Default, requires `OPENAI_API_KEY`                       |
| Azure OpenAI | `azure`  | Set `base_url` to your Azure endpoint                    |
| Ollama       | `ollama` | Local models, set `base_url` to `http://localhost:11434` |

### Model Recommendations

| Use Case           | Completion Model | Embedding Model           |
| ------------------ | ---------------- | ------------------------- |
| **Cost-effective** | `gpt-5-nano`     | `text-embedding-3-small`  |
| **High quality**   | `gpt-4o`         | `text-embedding-3-large`  |
| **Local/Private**  | Ollama `llama3`  | Ollama `nomic-embed-text` |

---

## Pipeline Configuration

Configuration for document processing and entity extraction.

| Config Field                       | Type     | Default   | Description                     |
| ---------------------------------- | -------- | --------- | ------------------------------- |
| `pipeline.chunk_size`              | usize    | `1200`    | Target chunk size (tokens)      |
| `pipeline.chunk_overlap`           | usize    | `100`     | Overlap between chunks (tokens) |
| `pipeline.entity_types`            | string[] | See below | Entity types to extract         |
| `pipeline.max_entities_per_chunk`  | usize    | `20`      | Max entities per chunk          |
| `pipeline.max_relations_per_chunk` | usize    | `20`      | Max relationships per chunk     |
| `pipeline.summarize_descriptions`  | bool     | `true`    | Summarize long descriptions     |
| `pipeline.max_description_tokens`  | usize    | `1200`    | Threshold for summarization     |
| `pipeline.concurrency`             | usize    | `4`       | Parallel extraction tasks       |

### Default Entity Types

```rust
[
    "PERSON",
    "ORGANIZATION",
    "LOCATION",
    "EVENT",
    "CONCEPT",
    "TECHNOLOGY",
    "PRODUCT"
]
```

### Chunking Strategy

```
┌──────────────────────────────────────────────────────────┐
│                     Document                              │
├──────────────────────┬───────────────────────────────────┤
│      Chunk 1         │  Overlap │      Chunk 2           │
│    (1200 tokens)     │  (100)   │   (1200 tokens)        │
└──────────────────────┴──────────┴────────────────────────┘
```

---

## Query Configuration

Configuration for the query engine and retrieval strategies.

| Config Field                      | Type      | Default  | Description                  |
| --------------------------------- | --------- | -------- | ---------------------------- |
| `query.default_mode`              | QueryMode | `hybrid` | Default query mode           |
| `query.max_vector_results`        | usize     | `20`     | Max vector search results    |
| `query.max_graph_depth`           | usize     | `3`      | Max graph traversal depth    |
| `query.max_context_entities`      | usize     | `30`     | Max entities in context      |
| `query.max_context_relationships` | usize     | `30`     | Max relationships in context |
| `query.max_context_chunks`        | usize     | `20`     | Max chunks in context        |
| `query.stream_responses`          | bool      | `true`   | Enable streaming by default  |

### Query Modes

| Mode     | Description                      | Best For                  |
| -------- | -------------------------------- | ------------------------- |
| `naive`  | Direct chunk vector search       | Simple factual queries    |
| `local`  | Entity-focused with neighborhood | Specific entity questions |
| `global` | Community/cluster-based search   | Broad topic exploration   |
| `hybrid` | Combines local + global          | General use (default)     |
| `bypass` | Skip RAG, direct to LLM          | Chat without retrieval    |

---

## API Configuration

Configuration for the REST API server.

| Variable             | Config Field       | Type     | Default    | Description             |
| -------------------- | ------------------ | -------- | ---------- | ----------------------- |
| `EDGEQUAKE_HOST`     | `api.host`         | string   | `0.0.0.0`  | Server bind address     |
| `EDGEQUAKE_PORT`     | `api.port`         | u16      | `8080`     | Server port             |
| `EDGEQUAKE_API_KEYS` | `api.api_keys`     | string[] | `[]`       | Valid API keys          |
| -                    | `api.cors_enabled` | bool     | `true`     | Enable CORS             |
| -                    | `api.cors_origins` | string[] | `["*"]`    | Allowed origins         |
| -                    | `api.auth_enabled` | bool     | `false`    | Require authentication  |
| -                    | `api.body_limit`   | usize    | `10485760` | Max request body (10MB) |
| -                    | `api.timeout_secs` | u64      | `300`      | Request timeout         |

### Authentication

To enable API key authentication:

```bash
# Set API keys (comma-separated)
EDGEQUAKE_API_KEYS=key1,key2,key3
```

Then include the key in requests:

```bash
curl -H "Authorization: Bearer key1" http://localhost:8080/api/v1/query
# or
curl -H "X-API-Key: key1" http://localhost:8080/api/v1/query
```

### CORS Configuration

For production, restrict CORS origins:

```toml
[api]
cors_enabled = true
cors_origins = ["https://app.example.com", "https://admin.example.com"]
```

---

## Configuration File (TOML)

Create `edgequake.toml` in the working directory:

```toml
[storage]
database_url = "postgres://localhost:5432/edgequake"
max_connections = 20
min_connections = 5
namespace = "production"

[llm]
provider = "openai"
model = "gpt-4o"
embedding_model = "text-embedding-3-large"
embedding_dim = 3072
max_tokens = 8192
temperature = 0.1
timeout_secs = 120
max_retries = 5

[pipeline]
chunk_size = 1000
chunk_overlap = 150
concurrency = 8
summarize_descriptions = true

[query]
default_mode = "hybrid"
max_vector_results = 30
max_graph_depth = 4
stream_responses = true

[api]
host = "0.0.0.0"
port = 8080
cors_enabled = true
cors_origins = ["https://myapp.com"]
auth_enabled = true
body_limit = 52428800  # 50MB
```

---

## Environment-Specific Configurations

### Development

```bash
# .env.development
OPENAI_API_KEY=sk-dev-key
EDGEQUAKE_DATABASE_URL=postgres://localhost:5432/edgequake_dev
EDGEQUAKE_HOST=127.0.0.1
EDGEQUAKE_PORT=8080
RUST_LOG=debug
```

### Production

```bash
# .env.production
OPENAI_API_KEY=${OPENAI_API_KEY}  # From secrets manager
EDGEQUAKE_DATABASE_URL=${DATABASE_URL}
EDGEQUAKE_HOST=0.0.0.0
EDGEQUAKE_PORT=8080
EDGEQUAKE_NAMESPACE=prod
EDGEQUAKE_API_KEYS=${API_KEYS}
RUST_LOG=info
```

### Docker

```bash
docker run -d \
  -e OPENAI_API_KEY=sk-xxx \
  -e EDGEQUAKE_DATABASE_URL=postgres://host.docker.internal:5432/edgequake \
  -e EDGEQUAKE_HOST=0.0.0.0 \
  -p 8080:8080 \
  edgequake/edgequake:latest
```

---

## Logging Configuration

EdgeQuake uses the `tracing` crate for logging. Configure via `RUST_LOG`:

```bash
# Log levels: error, warn, info, debug, trace
RUST_LOG=info                          # General info
RUST_LOG=edgequake=debug              # Debug EdgeQuake only
RUST_LOG=edgequake_pipeline=trace     # Trace pipeline
RUST_LOG=warn,edgequake=info          # Warn default, info for edgequake
```

---

## Performance Tuning

### High Throughput

```toml
[storage]
max_connections = 50
min_connections = 10

[pipeline]
concurrency = 16
chunk_size = 800  # Smaller chunks = faster processing

[llm]
max_retries = 5
timeout_secs = 30
```

### Memory Constrained

```toml
[storage]
max_connections = 5
min_connections = 1

[pipeline]
concurrency = 2
chunk_size = 1500  # Larger chunks = fewer chunks

[query]
max_context_entities = 15
max_context_chunks = 10
```

---

## Related Documentation

- [Getting Started](getting-started.md)
- [Query Modes Tutorial](query-modes.md)
- [Deployment Guide](deployment.md)
