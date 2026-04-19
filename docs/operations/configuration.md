---
title: "Configuration Reference"
---

# Configuration Reference

> **Complete EdgeQuake Configuration Options**

EdgeQuake is configured through environment variables and a `models.toml` file. This reference covers all available options.

---

## Configuration Sources

```
┌─────────────────────────────────────────────────────────────────┐
│                   CONFIGURATION PRIORITY                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. Environment Variables (highest priority)                    │
│     │                                                           │
│  2. models.toml (for LLM/embedding configuration)               │
│     │   - EDGEQUAKE_MODELS_CONFIG env var path                  │
│     │   - ./models.toml (current directory)                     │
│     │   - ~/.edgequake/models.toml                              │
│     │   - Built-in defaults                                     │
│     │                                                           │
│  3. Compile-time defaults (lowest priority)                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Environment Variables

### Core Settings

| Variable         | Type    | Default           | Description             |
| ---------------- | ------- | ----------------- | ----------------------- |
| `HOST`           | String  | `0.0.0.0`         | Server bind address     |
| `PORT`           | Integer | `8080`            | Server port             |
| `RUST_LOG`       | String  | `edgequake=debug` | Log level filter        |
| `WORKER_THREADS` | Integer | CPU count         | Background task workers |

### Database

| Variable       | Type   | Default | Description                  |
| -------------- | ------ | ------- | ---------------------------- |
| `DATABASE_URL` | String | None    | PostgreSQL connection string |

**Connection String Format:**

```
postgresql://user:password@host:port/database?sslmode=require
```

**Examples:**

```bash
# Local development
DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake"

# Production with SSL
DATABASE_URL="postgresql://edgequake:pass@db.example.com:5432/edgequake?sslmode=require"

# With connection pooling
DATABASE_URL="postgresql://edgequake:pass@pgbouncer:6432/edgequake"
```

### LLM Providers

#### OpenAI

| Variable          | Type   | Default                     | Description                          |
| ----------------- | ------ | --------------------------- | ------------------------------------ |
| `OPENAI_API_KEY`  | String | None                        | OpenAI API key (required for OpenAI) |
| `OPENAI_BASE_URL` | String | `https://api.openai.com/v1` | API endpoint                         |
| `OPENAI_ORG_ID`   | String | None                        | Organization ID (optional)           |

#### Ollama

| Variable                 | Type   | Default                  | Description                                                                                                      |
| ------------------------ | ------ | ------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| `OLLAMA_HOST`            | String | `http://localhost:11434` | Ollama server URL (LLM and embeddings)                                                                           |
| `OLLAMA_MODEL`           | String | `gemma3:latest`          | Default LLM model                                                                                                |
| `OLLAMA_EMBEDDING_MODEL` | String | `nomic-embed-text`       | Default embedding model                                                                                          |
| `OLLAMA_EMBEDDING_HOST`  | String | value of `OLLAMA_HOST`   | Dedicated Ollama host for embeddings only (closes [#140](https://github.com/raphaelmansuy/edgequake/issues/140)) |

#### LM Studio

| Variable             | Type   | Default                 | Description          |
| -------------------- | ------ | ----------------------- | -------------------- |
| `LM_STUDIO_BASE_URL` | String | `http://localhost:1234` | LM Studio server URL |

#### Anthropic

| Variable             | Type   | Default                     | Description                  |
| -------------------- | ------ | --------------------------- | ---------------------------- |
| `ANTHROPIC_API_KEY`  | String | None                        | Anthropic API key (required) |
| `ANTHROPIC_BASE_URL` | String | `https://api.anthropic.com` | API endpoint                 |

#### Google Gemini

| Variable          | Type   | Default                                     | Description       |
| ----------------- | ------ | ------------------------------------------- | ----------------- |
| `GEMINI_API_KEY`  | String | None                                        | Google AI API key |
| `GEMINI_BASE_URL` | String | `https://generativelanguage.googleapis.com` | API endpoint      |

#### xAI (Grok)

| Variable       | Type   | Default               | Description  |
| -------------- | ------ | --------------------- | ------------ |
| `XAI_API_KEY`  | String | None                  | xAI API key  |
| `XAI_BASE_URL` | String | `https://api.x.ai/v1` | API endpoint |

#### OpenRouter

| Variable              | Type   | Default                     | Description                   |
| --------------------- | ------ | --------------------------- | ----------------------------- |
| `OPENROUTER_API_KEY`  | String | None                        | OpenRouter API key (required) |
| `OPENROUTER_BASE_URL` | String | `https://openrouter.ai/api` | API endpoint                  |

#### MiniMax

| Variable           | Type   | Default                     | Description                                                |
| ------------------ | ------ | --------------------------- | ---------------------------------------------------------- |
| `MINIMAX_API_KEY`  | String | None                        | MiniMax API key (required)                                 |
| `MINIMAX_BASE_URL` | String | `https://api.minimax.io/v1` | API endpoint (use `https://api.minimaxi.com/v1` for China) |

#### Azure OpenAI

| Variable                   | Type   | Default              | Description                 |
| -------------------------- | ------ | -------------------- | --------------------------- |
| `AZURE_OPENAI_API_KEY`     | String | None                 | Azure OpenAI key (required) |
| `AZURE_OPENAI_ENDPOINT`    | String | None                 | Azure resource endpoint     |
| `AZURE_OPENAI_API_VERSION` | String | `2024-02-15-preview` | API version                 |

### Models Configuration

| Variable                        | Type    | Default  | Description                                    |
| ------------------------------- | ------- | -------- | ---------------------------------------------- |
| `EDGEQUAKE_MODELS_CONFIG`       | String  | None     | Path to custom models.toml                     |
| `EDGEQUAKE_LLM_PROVIDER`        | String  | `ollama` | Default LLM provider                           |
| `EDGEQUAKE_LLM_MODEL`           | String  | None     | Override LLM model name                        |
| `EDGEQUAKE_EMBEDDING_PROVIDER`  | String  | `ollama` | Override embedding provider type (hybrid mode) |
| `EDGEQUAKE_EMBEDDING_MODEL`     | String  | None     | Override embedding model name                  |
| `EDGEQUAKE_EMBEDDING_DIMENSION` | Integer | `768`    | Override embedding vector dimension            |

### Compatibility aliases

EdgeQuake also accepts the following migration aliases. They are normalized at startup so the rest
of the application continues to use the canonical `EDGEQUAKE_*` names:

| Alias                 | Canonical variable              |
| --------------------- | ------------------------------- |
| `MODEL_PROVIDER`      | `EDGEQUAKE_LLM_PROVIDER`        |
| `CHAT_MODEL`          | `EDGEQUAKE_LLM_MODEL`           |
| `EMBEDDING_PROVIDER`  | `EDGEQUAKE_EMBEDDING_PROVIDER`  |
| `EMBEDDING_MODEL`     | `EDGEQUAKE_EMBEDDING_MODEL`     |
| `EMBEDDING_DIMENSION` | `EDGEQUAKE_EMBEDDING_DIMENSION` |

When both an alias and a canonical variable are set, the canonical variable wins.

### Hybrid Provider Mode (closes [#140](https://github.com/raphaelmansuy/edgequake/issues/140))

Run a different provider or Ollama instance for embeddings vs. LLM inference:

| Variable                        | Type    | Default                | Description                                         |
| ------------------------------- | ------- | ---------------------- | --------------------------------------------------- |
| `OLLAMA_EMBEDDING_HOST`         | String  | value of `OLLAMA_HOST` | Dedicated Ollama host for embeddings                |
| `EDGEQUAKE_EMBEDDING_PROVIDER`  | String  | (same as LLM)          | Explicit embedding provider (`ollama`, `openai`, …) |
| `EDGEQUAKE_EMBEDDING_MODEL`     | String  | provider default       | Model for the embedding override                    |
| `EDGEQUAKE_EMBEDDING_DIMENSION` | Integer | `768`                  | Vector dimension for the embedding override         |

**Priority:** `EDGEQUAKE_EMBEDDING_PROVIDER` → `OLLAMA_EMBEDDING_HOST` → default (from `from_env()`).

**Example — OpenAI for LLM, dedicated Ollama node for embeddings:**

```bash
export EDGEQUAKE_LLM_PROVIDER=openai
export OPENAI_API_KEY=sk-...
export OLLAMA_EMBEDDING_HOST=http://gpu-box:11434
export OLLAMA_EMBEDDING_MODEL=nomic-embed-text
```

### Security / Frontend

| Variable                         | Type   | Default | Description                                                                                                                             |
| -------------------------------- | ------ | ------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `NEXT_PUBLIC_DISABLE_DEMO_LOGIN` | String | `false` | Set to `true` to hide the demo "skip login" button in production (closes [#139](https://github.com/raphaelmansuy/edgequake/issues/139)) |

> **Production tip:** Always set `NEXT_PUBLIC_DISABLE_DEMO_LOGIN=true` in
> your frontend build when deploying EdgeQuake to a public-facing environment.

---

## models.toml Reference

The `models.toml` file configures LLM providers and model cards.

### Location Priority

1. `EDGEQUAKE_MODELS_CONFIG` environment variable
2. `./models.toml` (current working directory)
3. `~/.edgequake/models.toml` (user home)
4. Built-in defaults

### Structure

```toml
# Default provider selection
[defaults]
llm_provider = "ollama"              # or "openai", "lm_studio"
llm_model = "gemma3:12b"
embedding_provider = "ollama"
embedding_model = "embeddinggemma"

# Provider definitions
[[providers]]
name = "openai"
display_name = "OpenAI"
type = "openai"
api_base = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
enabled = true
priority = 10
description = "OpenAI GPT models"

# Model definitions within provider
[[providers.models]]
name = "gpt-5-nano"
display_name = "GPT-4o Mini"
model_type = "llm"                   # or "embedding"
description = "Cost-effective model"
deprecated = false
tags = ["recommended", "fast"]

[providers.models.capabilities]
context_length = 128000
max_output_tokens = 16384
supports_vision = true
supports_function_calling = true
supports_json_mode = true
supports_streaming = true
supports_system_message = true
embedding_dimension = 0              # 0 for LLMs, >0 for embeddings

[providers.models.cost]
input_per_1k = 0.00015
output_per_1k = 0.0006
embedding_per_1k = 0.0
image_per_unit = 0.0
```

### Provider Types

| Type         | Description             | API Key Variable       |
| ------------ | ----------------------- | ---------------------- |
| `openai`     | OpenAI API compatible   | `OPENAI_API_KEY`       |
| `anthropic`  | Anthropic Claude models | `ANTHROPIC_API_KEY`    |
| `gemini`     | Google Gemini models    | `GEMINI_API_KEY`       |
| `xai`        | xAI Grok models         | `XAI_API_KEY`          |
| `openrouter` | OpenRouter aggregator   | `OPENROUTER_API_KEY`   |
| `minimax`    | MiniMax AI models       | `MINIMAX_API_KEY`      |
| `azure`      | Azure OpenAI            | `AZURE_OPENAI_API_KEY` |
| `ollama`     | Ollama local models     | None (local)           |
| `lm_studio`  | LM Studio local         | None (local)           |
| `mock`       | Testing without costs   | None                   |

### Model Types

| Type        | Purpose           | Key Capability                        |
| ----------- | ----------------- | ------------------------------------- |
| `llm`       | Text generation   | `context_length`, `max_output_tokens` |
| `embedding` | Vector embeddings | `embedding_dimension`                 |

---

## Provider Configuration Examples

### OpenAI (Production)

```toml
[[providers]]
name = "openai"
display_name = "OpenAI"
type = "openai"
api_base = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
enabled = true
priority = 10

[[providers.models]]
name = "gpt-5-nano"
display_name = "GPT-4o Mini"
model_type = "llm"
tags = ["recommended"]

[providers.models.capabilities]
context_length = 128000
max_output_tokens = 16384
supports_vision = true
supports_function_calling = true
supports_json_mode = true
supports_streaming = true

[[providers.models]]
name = "text-embedding-3-small"
display_name = "Text Embedding 3 Small"
model_type = "embedding"
tags = ["recommended"]

[providers.models.capabilities]
context_length = 8191
embedding_dimension = 1536
```

### Ollama (Local Development)

```toml
[[providers]]
name = "ollama"
display_name = "Ollama"
type = "ollama"
api_base = "http://localhost:11434"
enabled = true
priority = 20

[[providers.models]]
name = "gemma3:12b"
display_name = "Gemma 3 12B"
model_type = "llm"
tags = ["recommended", "local"]

[providers.models.capabilities]
context_length = 128000
max_output_tokens = 8192
supports_vision = true
supports_streaming = true

[[providers.models]]
name = "nomic-embed-text"
display_name = "Nomic Embed Text"
model_type = "embedding"

[providers.models.capabilities]
context_length = 8192
embedding_dimension = 768
```

### Azure OpenAI

```toml
[[providers]]
name = "azure-openai"
display_name = "Azure OpenAI"
type = "openai"  # Uses OpenAI-compatible API
api_base = "https://your-resource.openai.azure.com"
api_key_env = "AZURE_OPENAI_API_KEY"
enabled = true
priority = 5

[[providers.models]]
name = "gpt-5-nano"  # Your deployment name
display_name = "Azure GPT-4o Mini"
model_type = "llm"

[providers.models.capabilities]
context_length = 128000
max_output_tokens = 16384
supports_function_calling = true
supports_json_mode = true
supports_streaming = true
```

### Anthropic Claude

```toml
[[providers]]
name = "anthropic"
display_name = "Anthropic"
type = "anthropic"
api_base = "https://api.anthropic.com"
api_key_env = "ANTHROPIC_API_KEY"
enabled = true
priority = 8

[[providers.models]]
name = "claude-sonnet-4-5-20250929"
display_name = "Claude Sonnet 4.5"
model_type = "llm"
tags = ["recommended", "fast"]

[providers.models.capabilities]
context_length = 200000
max_output_tokens = 128000
supports_vision = true
supports_streaming = true
supports_system_message = true

[providers.models.cost]
input_per_1k = 0.003
output_per_1k = 0.015
```

### Google Gemini

```toml
[[providers]]
name = "gemini"
display_name = "Google Gemini"
type = "gemini"
api_base = "https://generativelanguage.googleapis.com"
api_key_env = "GEMINI_API_KEY"
enabled = true
priority = 9

[[providers.models]]
name = "gemini-2.5-flash"
display_name = "Gemini 2.5 Flash"
model_type = "llm"
tags = ["recommended", "fast", "thinking"]

[providers.models.capabilities]
context_length = 1000000
max_output_tokens = 8192
supports_vision = true
supports_streaming = true

[providers.models.cost]
input_per_1k = 0.00015
output_per_1k = 0.0006

[[providers.models]]
name = "gemini-embedding-001"
display_name = "Gemini Embedding"
model_type = "embedding"

[providers.models.capabilities]
context_length = 10000
embedding_dimension = 3072

[providers.models.cost]
input_per_1k = 0.00015
```

### xAI (Grok)

```toml
[[providers]]
name = "xai"
display_name = "xAI"
type = "xai"
api_base = "https://api.x.ai/v1"
api_key_env = "XAI_API_KEY"
enabled = true
priority = 7

[[providers.models]]
name = "grok-4-1-fast"
display_name = "Grok 4.1 Fast"
model_type = "llm"
tags = ["recommended", "fast", "large-context"]

[providers.models.capabilities]
context_length = 2000000
max_output_tokens = 16384
supports_vision = false
supports_streaming = true

[providers.models.cost]
input_per_1k = 0.0002
output_per_1k = 0.0005
```

### OpenRouter

```toml
[[providers]]
name = "openrouter"
display_name = "OpenRouter"
type = "openrouter"
api_base = "https://openrouter.ai/api"
api_key_env = "OPENROUTER_API_KEY"
enabled = true
priority = 6

[[providers.models]]
name = "openai/gpt-5-nano"
display_name = "OpenRouter GPT-4o Mini"
model_type = "llm"
tags = ["recommended"]

[providers.models.capabilities]
context_length = 128000
max_output_tokens = 16384
supports_vision = true
supports_streaming = true

[providers.models.cost]
input_per_1k = 0.00015
output_per_1k = 0.0006
```

---

## Runtime Provider Switching

EdgeQuake supports switching providers at runtime via API:

```bash
# Get current providers
curl http://localhost:8080/api/v1/providers

# Get available models for a provider
curl http://localhost:8080/api/v1/providers/openai/models

# Query with specific provider (per-request)
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What is quantum computing?",
    "mode": "hybrid",
    "llm_provider": "openai",
    "llm_model": "gpt-5-nano"
  }'
```

---

## Workspace-Level Configuration

Each workspace can have its own LLM/embedding configuration:

```bash
# Create workspace with custom providers
curl -X POST http://localhost:8080/api/v1/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Production Workspace",
    "llm_provider": "openai",
    "llm_model": "gpt-4o",
    "embedding_provider": "openai",
    "embedding_model": "text-embedding-3-large"
  }'
```

Workspace configuration overrides server defaults for all operations within that workspace.

---

## Logging Configuration

The `RUST_LOG` environment variable controls logging:

```bash
# Debug all EdgeQuake components
RUST_LOG="edgequake=debug"

# Production logging
RUST_LOG="edgequake=info,tower_http=info"

# Verbose debugging
RUST_LOG="edgequake=trace,sqlx=debug,tower_http=debug"

# Specific component debugging
RUST_LOG="edgequake_pipeline=debug,edgequake_query=debug"
```

### Log Levels

| Level   | Use Case              |
| ------- | --------------------- |
| `error` | Errors only           |
| `warn`  | Errors + warnings     |
| `info`  | Standard production   |
| `debug` | Development debugging |
| `trace` | Detailed tracing      |

---

## Performance Tuning

### Worker Threads

```bash
# Set worker count (default: CPU count)
WORKER_THREADS=8
```

Workers handle background document processing. More workers = faster ingestion but higher memory.

### Connection Pool (PostgreSQL)

Connection pooling is built into SQLx. For high-load scenarios, use an external pooler:

```bash
# Use PgBouncer
DATABASE_URL="postgresql://user:pass@pgbouncer:6432/edgequake?application_name=edgequake"
```

### Query Tuning

| Setting        | Via API   | Default | Description            |
| -------------- | --------- | ------- | ---------------------- |
| `max_chunks`   | Per query | 10      | Max chunks retrieved   |
| `max_entities` | Per query | 20      | Max entities retrieved |
| `temperature`  | Per query | 0.7     | LLM temperature        |
| `max_tokens`   | Per query | 4096    | Max response tokens    |

---

## Example Configurations

### Development (Minimal)

```bash
# Requires DATABASE_URL — set via .env or environment
# Mock LLM if no OPENAI_API_KEY is set
cargo run
```

### Development with Ollama

```bash
export OLLAMA_HOST="http://localhost:11434"
export OLLAMA_MODEL="gemma3:12b"
cargo run
```

### Development with PostgreSQL

```bash
export DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake"
export OPENAI_API_KEY="sk-..."
cargo run
```

### Production

```bash
export DATABASE_URL="postgresql://edgequake:$DB_PASS@db.example.com:5432/edgequake?sslmode=require"
export OPENAI_API_KEY="$OPENAI_KEY"
export RUST_LOG="edgequake=info,tower_http=info"
export HOST="0.0.0.0"
export PORT="8080"
export WORKER_THREADS="8"
./edgequake
```

---

## Validation

EdgeQuake validates configuration at startup:

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   ⚡ EdgeQuake v0.10.x                                        ║
║                                                              ║
║   🐘 Storage: POSTGRESQL (persistent)
║   🌐 Server:  http://0.0.0.0:8080
║   📚 Swagger: http://0.0.0.0:8080/swagger-ui/
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

Validation errors are logged with actionable messages:

```
ERROR: DATABASE_URL is invalid: invalid connection string
HINT: Format: postgresql://user:password@host:port/database
```

---

## See Also

- [Deployment Guide](/docs/operations/deployment/) - Production deployment
- [Monitoring Guide](/docs/operations/monitoring/) - Observability setup
- [REST API Reference](/docs/api-reference/rest-api/) - API documentation
- [LLM Provider Docs](/docs/concepts/hybrid-retrieval/) - Provider integration
