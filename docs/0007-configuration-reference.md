# EdgeQuake Configuration Reference

> Complete reference for all EdgeQuake configuration options

**Version**: 0.1.0 | **Last Updated**: December 2025

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
    pub host: String,               // Default: "127.0.0.1"
    
    /// Server listen port
    pub port: u16,                  // Default: 8080
    
    /// CORS allowed origins
    pub cors_origins: Vec<String>,  // Default: ["*"]
    
    /// Request body size limit
    pub max_body_size: usize,       // Default: 52_428_800 (50MB)
    
    /// Request timeout in seconds
    pub request_timeout: u64,       // Default: 300
    
    /// Enable request logging
    pub enable_logging: bool,       // Default: true
}
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `EDGEQUAKE_API_HOST` | `127.0.0.1` | Listen address |
| `EDGEQUAKE_API_PORT` | `8080` | Listen port |
| `EDGEQUAKE_CORS_ORIGINS` | `*` | Comma-separated origins |
| `EDGEQUAKE_MAX_BODY_SIZE` | `52428800` | Max request body (bytes) |
| `EDGEQUAKE_REQUEST_TIMEOUT` | `300` | Timeout (seconds) |

### Example

```bash
export EDGEQUAKE_API_HOST="0.0.0.0"
export EDGEQUAKE_API_PORT="8080"
export EDGEQUAKE_CORS_ORIGINS="http://localhost:3000,https://app.example.com"
```

---

## Storage Configuration

### StorageConfig

```rust
pub struct StorageConfig {
    /// Storage backend type: memory, postgresql
    pub storage_type: String,           // Default: "memory"
    
    /// PostgreSQL connection string
    pub connection_string: Option<String>,
    
    /// Connection pool size
    pub pool_size: u32,                 // Default: 10
    
    /// Connection timeout (seconds)
    pub connect_timeout: u64,           // Default: 30
    
    /// Idle connection timeout (seconds)
    pub idle_timeout: u64,              // Default: 600
}
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `EDGEQUAKE_STORAGE_TYPE` | `memory` | Backend: `memory`, `postgresql` |
| `DATABASE_URL` | - | PostgreSQL connection string |
| `EDGEQUAKE_POOL_SIZE` | `10` | Connection pool size |
| `EDGEQUAKE_CONNECT_TIMEOUT` | `30` | Connection timeout |

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
    
    /// Retry delay (seconds)
    pub retry_delay_secs: u64,          // Default: 1
}
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `OPENAI_API_KEY` | - | OpenAI API key |
| `EDGEQUAKE_LLM_PROVIDER` | `openai` | Provider: `openai`, `ollama` |
| `EDGEQUAKE_LLM_MODEL` | `gpt-4o-mini` | LLM model name |
| `EDGEQUAKE_EMBEDDING_MODEL` | `text-embedding-3-small` | Embedding model |
| `EDGEQUAKE_EMBEDDING_DIM` | `1536` | Vector dimension |
| `EDGEQUAKE_LLM_TEMPERATURE` | `0.0` | Generation temperature |
| `EDGEQUAKE_LLM_MAX_TOKENS` | `4096` | Max generation tokens |
| `OPENAI_BASE_URL` | - | Custom API endpoint |
| `OLLAMA_HOST` | `http://localhost:11434` | Ollama server |

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
    
    /// Maximum entities per chunk
    pub max_entities_per_chunk: usize,  // Default: 30
    
    /// Entity extraction gleaning passes
    pub gleaning_count: usize,          // Default: 1
    
    /// Enable entity deduplication
    pub enable_deduplication: bool,     // Default: true
    
    /// Similarity threshold for dedup (0-1)
    pub dedup_threshold: f32,           // Default: 0.85
    
    /// Parallel processing workers
    pub max_concurrent_chunks: usize,   // Default: 4
    
    /// Enable community detection
    pub enable_communities: bool,       // Default: true
    
    /// Leiden algorithm resolution
    pub community_resolution: f32,      // Default: 1.0
}
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `EDGEQUAKE_CHUNK_SIZE` | `1200` | Characters per chunk |
| `EDGEQUAKE_CHUNK_OVERLAP` | `100` | Overlap between chunks |
| `EDGEQUAKE_MAX_ENTITIES_PER_CHUNK` | `30` | Max entities extracted |
| `EDGEQUAKE_GLEANING_COUNT` | `1` | Extraction passes |
| `EDGEQUAKE_ENABLE_DEDUP` | `true` | Enable deduplication |
| `EDGEQUAKE_DEDUP_THRESHOLD` | `0.85` | Dedup similarity |
| `EDGEQUAKE_MAX_CONCURRENT_CHUNKS` | `4` | Parallel workers |

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
    
    /// Top-K results to retrieve
    pub top_k: usize,                   // Default: 60
    
    /// Similarity threshold (0-1)
    pub similarity_threshold: f32,      // Default: 0.5
    
    /// Max tokens for context
    pub max_context_tokens: usize,      // Default: 4000
    
    /// Enable query expansion
    pub enable_query_expansion: bool,   // Default: true
    
    /// Include sources in response
    pub include_sources: bool,          // Default: true
    
    /// Enable streaming responses
    pub enable_streaming: bool,         // Default: true
}

pub enum QueryMode {
    /// Simple vector search
    Naive,
    
    /// Entity-focused retrieval
    Local,
    
    /// Community summaries
    Global,
    
    /// Combined vector + graph
    Hybrid,
    
    /// Adaptive mode selection
    Mix,
}
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `EDGEQUAKE_DEFAULT_MODE` | `hybrid` | Query mode |
| `EDGEQUAKE_TOP_K` | `60` | Results to retrieve |
| `EDGEQUAKE_SIMILARITY_THRESHOLD` | `0.5` | Min similarity (0-1) |
| `EDGEQUAKE_MAX_CONTEXT_TOKENS` | `4000` | Max context size |
| `EDGEQUAKE_ENABLE_STREAMING` | `true` | Enable streaming |

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

# Adaptive (auto-selects)
export EDGEQUAKE_DEFAULT_MODE="mix"
```

---

## Environment Variables

### Complete Reference

```bash
# =============================================================================
# API Configuration
# =============================================================================
EDGEQUAKE_API_HOST=0.0.0.0           # Listen address
EDGEQUAKE_API_PORT=8080              # Listen port
EDGEQUAKE_CORS_ORIGINS=*             # CORS origins (comma-separated)
EDGEQUAKE_MAX_BODY_SIZE=52428800     # Max request body (bytes)
EDGEQUAKE_REQUEST_TIMEOUT=300        # Request timeout (seconds)

# =============================================================================
# Storage Configuration
# =============================================================================
EDGEQUAKE_STORAGE_TYPE=memory        # memory | postgresql
DATABASE_URL=postgresql://user:pass@localhost:5432/db
EDGEQUAKE_POOL_SIZE=10               # Connection pool size
EDGEQUAKE_CONNECT_TIMEOUT=30         # Connection timeout (seconds)

# =============================================================================
# LLM Configuration
# =============================================================================
OPENAI_API_KEY=sk-...                # OpenAI API key
EDGEQUAKE_LLM_PROVIDER=openai        # openai | ollama
EDGEQUAKE_LLM_MODEL=gpt-4o-mini      # LLM model
EDGEQUAKE_EMBEDDING_MODEL=text-embedding-3-small
EDGEQUAKE_EMBEDDING_DIM=1536         # Vector dimension
EDGEQUAKE_LLM_TEMPERATURE=0.0        # Generation temperature
EDGEQUAKE_LLM_MAX_TOKENS=4096        # Max tokens
OPENAI_BASE_URL=                     # Custom API endpoint
OLLAMA_HOST=http://localhost:11434   # Ollama server

# =============================================================================
# Pipeline Configuration
# =============================================================================
EDGEQUAKE_CHUNK_SIZE=1200            # Chunk size (chars)
EDGEQUAKE_CHUNK_OVERLAP=100          # Chunk overlap
EDGEQUAKE_MAX_ENTITIES_PER_CHUNK=30  # Max entities
EDGEQUAKE_GLEANING_COUNT=1           # Extraction passes
EDGEQUAKE_ENABLE_DEDUP=true          # Enable deduplication
EDGEQUAKE_DEDUP_THRESHOLD=0.85       # Dedup threshold

# =============================================================================
# Query Configuration
# =============================================================================
EDGEQUAKE_DEFAULT_MODE=hybrid        # naive|local|global|hybrid|mix
EDGEQUAKE_TOP_K=60                   # Top-K retrieval
EDGEQUAKE_SIMILARITY_THRESHOLD=0.5   # Min similarity
EDGEQUAKE_MAX_CONTEXT_TOKENS=4000    # Context limit
EDGEQUAKE_ENABLE_STREAMING=true      # Enable streaming

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
cors_origins = ["http://localhost:3000", "https://app.example.com"]
max_body_size = 52_428_800
request_timeout = 300

[storage]
storage_type = "postgresql"
connection_string = "postgresql://edgequake:password@localhost:5432/edgequake"
pool_size = 10
connect_timeout = 30
idle_timeout = 600

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
max_entities_per_chunk = 30
gleaning_count = 1
enable_deduplication = true
dedup_threshold = 0.85
max_concurrent_chunks = 4
enable_communities = true
community_resolution = 1.0

[query]
default_mode = "hybrid"
top_k = 60
similarity_threshold = 0.5
max_context_tokens = 4000
enable_query_expansion = true
include_sources = true
enable_streaming = true
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
