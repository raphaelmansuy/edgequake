# EdgeQuake Configuration Reference

> Complete reference for all EdgeQuake configuration options

**Version**: 0.1.0 | **Last Updated**: December 2025

> **Code Reference**: See [edgequake/crates/edgequake-core/src/config.rs](../edgequake/crates/edgequake-core/src/config.rs) for configuration structures

---

## Table of Contents

1. [Configuration Overview](#configuration-overview)
2. [API Configuration](#api-configuration)
3. [Storage Configuration](#storage-configuration)
4. [LLM Configuration](#llm-configuration)
5. [Pipeline Configuration](#pipeline-configuration)
6. [Query Configuration](#query-configuration)
7. [Environment Variables](#environment-variables)
8. [Configuration File](#configuration-file)

---

## Configuration Overview

EdgeQuake supports configuration through:

1. **Environment Variables** - Recommended for production/Docker
2. **Configuration File** - TOML format for complex setups
3. **Programmatic** - Direct struct configuration in code

### Priority Order

```
Environment Variables > Config File > Default Values
```

### Configuration Structure

```rust
// edgequake/crates/edgequake-core/src/config.rs

pub struct Config {
    pub api: ApiConfig,
    pub storage: StorageConfig,
    pub llm: LlmConfig,
    pub pipeline: PipelineConfig,
    pub query: QueryConfig,
}
```

---

## API Configuration

### ApiConfig

```rust
pub struct ApiConfig {
    /// Server listen address
    pub host: String,               // Default: "0.0.0.0"

    /// Server listen port
    pub port: u16,                  // Default: 8080

    /// Enable CORS
    pub cors_enabled: bool,         // Default: true

    /// CORS allowed origins
    pub cors_origins: Vec<String>,  // Default: ["*"]

    /// Enable API key authentication
    pub auth_enabled: bool,         // Default: false

    /// API keys for authentication
    pub api_keys: Vec<String>,      // Default: []

    /// Request body size limit in bytes
    pub body_limit: usize,          // Default: 10_485_760 (10MB)

    /// Request timeout in seconds
    pub timeout_secs: u64,          // Default: 300
}
```

### Environment Variables

> **Note**: The actual server binary uses simpler environment variable names. See [edgequake/src/main.rs](../edgequake/src/main.rs) for the actual implementation.

| Variable | Default   | Description    |
| -------- | --------- | -------------- |
| `HOST`   | `0.0.0.0` | Listen address |
| `PORT`   | `8080`    | Listen port    |

### Example

```bash
export HOST="0.0.0.0"
export PORT="8080"
```

---

## Storage Configuration

### StorageConfig

```rust
pub struct StorageConfig {
    /// Database connection URL
    pub database_url: String,           // Default: "postgres://localhost:5432/edgequake"

    /// Maximum connections in pool
    pub max_connections: u32,            // Default: 10

    /// Minimum connections in pool
    pub min_connections: u32,            // Default: 1

    /// Connection timeout (seconds)
    pub connect_timeout_secs: u64,       // Default: 30

    /// Namespace for multi-tenancy
    pub namespace: Option<String>,       // Default: None
}
```

### Environment Variables

| Variable                 | Default | Description                  |
| ------------------------ | ------- | ---------------------------- |
| `EDGEQUAKE_DATABASE_URL` | -       | PostgreSQL connection string |
| `EDGEQUAKE_NAMESPACE`    | -       | Multi-tenant namespace       |

### PostgreSQL Connection String

```
postgresql://user:password@host:port/database?options
```

**Options:**

- `sslmode=require` - Enable SSL
- `application_name=edgequake` - App identifier
- `connect_timeout=10` - Connection timeout

**Example:**

```bash
export DATABASE_URL="postgresql://edgequake:secret@localhost:5432/edgequake?sslmode=prefer"
```

### Memory Storage

```bash
# Development mode - no database required
export EDGEQUAKE_STORAGE_TYPE="memory"
```

---

## LLM Configuration

### LlmConfig

```rust
pub struct LlmConfig {
    /// LLM provider: openai, ollama
    pub provider: String,               // Default: "openai"

    /// API key (from env or config)
    pub api_key: Option<String>,

    /// Custom API base URL
    pub base_url: Option<String>,

    /// LLM model for generation
    pub model: String,                  // Default: "gpt-4o-mini"

    /// Embedding model
    pub embedding_model: String,        // Default: "text-embedding-3-small"

    /// Embedding vector dimension
    pub embedding_dim: usize,           // Default: 1536

    /// Generation temperature (0-2)
    pub temperature: f32,               // Default: 0.0

    /// Max tokens for generation
    pub max_tokens: usize,              // Default: 4096

    /// API request timeout (seconds)
    pub timeout_secs: u64,              // Default: 60

    /// Max retries for failed requests
    pub max_retries: u32,               // Default: 3
}
```

### Environment Variables

| Variable                    | Default                  | Description                  |
| --------------------------- | ------------------------ | ---------------------------- |
| `OPENAI_API_KEY`            | -                        | OpenAI API key               |
| `EDGEQUAKE_LLM_PROVIDER`    | `openai`                 | Provider: `openai`, `ollama` |
| `EDGEQUAKE_LLM_MODEL`       | `gpt-4o-mini`            | LLM model name               |
| `EDGEQUAKE_EMBEDDING_MODEL` | `text-embedding-3-small` | Embedding model              |
| `EDGEQUAKE_EMBEDDING_DIM`   | `1536`                   | Vector dimension             |
| `EDGEQUAKE_LLM_TEMPERATURE` | `0.0`                    | Generation temperature       |
| `EDGEQUAKE_LLM_MAX_TOKENS`  | `4096`                   | Max generation tokens        |
| `OPENAI_BASE_URL`           | -                        | Custom API endpoint          |
| `OLLAMA_HOST`               | `http://localhost:11434` | Ollama server                |

### Provider Examples

**OpenAI:**

```bash
export OPENAI_API_KEY="sk-..."
export EDGEQUAKE_LLM_MODEL="gpt-4o-mini"
export EDGEQUAKE_EMBEDDING_MODEL="text-embedding-3-small"
```

**Ollama:**

```bash
export EDGEQUAKE_LLM_PROVIDER="ollama"
export OLLAMA_HOST="http://localhost:11434"
export EDGEQUAKE_LLM_MODEL="llama3.2:3b"
export EDGEQUAKE_EMBEDDING_MODEL="nomic-embed-text"
```

---

## Pipeline Configuration

### PipelineConfig

```rust
pub struct PipelineConfig {
    /// Chunk size in characters
    pub chunk_size: usize,              // Default: 1200

    /// Overlap between chunks
    pub chunk_overlap: usize,           // Default: 100

    /// Entity types to extract
    pub entity_types: Vec<String>,      // Default: ["PERSON", "ORGANIZATION", ...]

    /// Maximum entities per chunk
    pub max_entities_per_chunk: usize,  // Default: 20

    /// Maximum relations per chunk
    pub max_relations_per_chunk: usize, // Default: 20

    /// Summarize long descriptions
    pub summarize_descriptions: bool,   // Default: true

    /// Max description tokens before summarization
    pub max_description_tokens: usize,  // Default: 1200

    /// Concurrent extraction tasks
    pub concurrency: usize,             // Default: 4
}
```

### Environment Variables

| Variable                           | Default | Description            |
| ---------------------------------- | ------- | ---------------------- |
| `EDGEQUAKE_CHUNK_SIZE`             | `1200`  | Characters per chunk   |
| `EDGEQUAKE_CHUNK_OVERLAP`          | `100`   | Overlap between chunks |
| `EDGEQUAKE_MAX_ENTITIES_PER_CHUNK` | `20`    | Max entities extracted |

### Chunking Strategies

```bash
# Small documents - smaller chunks
export EDGEQUAKE_CHUNK_SIZE="800"
export EDGEQUAKE_CHUNK_OVERLAP="50"

# Large documents - larger chunks
export EDGEQUAKE_CHUNK_SIZE="2000"
export EDGEQUAKE_CHUNK_OVERLAP="200"
```

---

## Query Configuration

### QueryConfig

```rust
pub struct QueryConfig {
    /// Default query mode
    pub default_mode: QueryMode,        // Default: Hybrid

    /// Maximum results for vector search
    pub max_vector_results: usize,      // Default: 20

    /// Maximum graph traversal depth
    pub max_graph_depth: usize,         // Default: 3

    /// Maximum entities in context
    pub max_context_entities: usize,    // Default: 30

    /// Maximum relationships in context
    pub max_context_relationships: usize, // Default: 30

    /// Maximum chunks in context
    pub max_context_chunks: usize,      // Default: 20

    /// Whether to stream responses
    pub stream_responses: bool,         // Default: true
}

pub enum QueryMode {
    /// Simple vector search
    Naive,

    /// Entity-focused retrieval
    Local,

    /// Community summaries
    Global,

    /// Combined vector + graph (default)
    Hybrid,

    /// No RAG, direct LLM query
    Bypass,
}
```

### Environment Variables

| Variable                     | Default  | Description      |
| ---------------------------- | -------- | ---------------- |
| `EDGEQUAKE_DEFAULT_MODE`     | `hybrid` | Query mode       |
| `EDGEQUAKE_ENABLE_STREAMING` | `true`   | Enable streaming |

### Query Mode Examples

```bash
# Fast, simple queries
export EDGEQUAKE_DEFAULT_MODE="naive"

# Entity-focused
export EDGEQUAKE_DEFAULT_MODE="local"

# High-level summaries
export EDGEQUAKE_DEFAULT_MODE="global"

# Best quality (recommended)
export EDGEQUAKE_DEFAULT_MODE="hybrid"

# Direct LLM (no RAG)
export EDGEQUAKE_DEFAULT_MODE="bypass"
```

---

## Environment Variables

### Complete Reference

> **Important**: The actual binary uses simple environment variable names. Many `EDGEQUAKE_*` prefixed variables are for the configuration structs but the main binary uses simpler names like `HOST`, `PORT`, `OPENAI_API_KEY`, `WORKER_THREADS`.

```bash
# =============================================================================
# API Configuration (used by main binary - edgequake/src/main.rs)
# =============================================================================
HOST=0.0.0.0                         # Listen address
PORT=8080                            # Listen port
WORKER_THREADS=4                     # Number of worker threads

# =============================================================================
# LLM Configuration
# =============================================================================
OPENAI_API_KEY=sk-...                # OpenAI API key (required for production)

# =============================================================================
# Logging
# =============================================================================
RUST_LOG=info                        # Log level
RUST_LOG=edgequake=debug             # Debug EdgeQuake only
```

---

## Configuration File

### TOML Format

```toml
# config.toml

[api]
host = "0.0.0.0"
port = 8080
cors_enabled = true
cors_origins = ["http://localhost:3000", "https://app.example.com"]
auth_enabled = false
body_limit = 10485760  # 10MB
timeout_secs = 300

[storage]
database_url = "postgresql://edgequake:password@localhost:5432/edgequake"
max_connections = 10
min_connections = 1
connect_timeout_secs = 30
# namespace = "production"

[llm]
provider = "openai"
# api_key loaded from OPENAI_API_KEY env var
model = "gpt-4o-mini"
embedding_model = "text-embedding-3-small"
embedding_dim = 1536
temperature = 0.0
max_tokens = 4096
timeout_secs = 60
max_retries = 3

[pipeline]
chunk_size = 1200
chunk_overlap = 100
entity_types = ["PERSON", "ORGANIZATION", "LOCATION", "EVENT", "CONCEPT", "TECHNOLOGY", "PRODUCT"]
max_entities_per_chunk = 20
max_relations_per_chunk = 20
summarize_descriptions = true
max_description_tokens = 1200
concurrency = 4

[query]
default_mode = "hybrid"
max_vector_results = 20
max_graph_depth = 3
max_context_entities = 30
max_context_relationships = 30
max_context_chunks = 20
stream_responses = true
```

### Loading Config

```rust
use edgequake_core::Config;

// Load from file
let config = Config::from_file("config.toml")?;

// Or from environment (preferred for production)
let config = Config::from_env()?;

// Or combine (env overrides file)
let config = Config::from_file("config.toml")?
    .with_env_overrides();
```

---

## Profile Presets

### Development

```bash
# .env.development
EDGEQUAKE_STORAGE_TYPE=memory
EDGEQUAKE_LLM_PROVIDER=ollama
OLLAMA_HOST=http://localhost:11434
EDGEQUAKE_LLM_MODEL=llama3.2:3b
RUST_LOG=edgequake=debug
```

### Testing

```bash
# .env.test
EDGEQUAKE_STORAGE_TYPE=memory
# No OPENAI_API_KEY = uses mock provider
RUST_LOG=warn
```

### Production

```bash
# .env.production
EDGEQUAKE_API_HOST=0.0.0.0
EDGEQUAKE_STORAGE_TYPE=postgresql
DATABASE_URL=postgresql://...
OPENAI_API_KEY=sk-...
EDGEQUAKE_LLM_MODEL=gpt-4o-mini
EDGEQUAKE_EMBEDDING_MODEL=text-embedding-3-small
RUST_LOG=info
```

---

## Next Steps

- **[Quick Start](0001-quick-start.md)** - Get started in 5 minutes
- **[LLM Integration](0005-llm-integration.md)** - LLM provider setup
- **[Deployment Guide](0006-deployment-guide.md)** - Production deployment
