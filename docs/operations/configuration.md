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
| `OLLAMA_MODEL`           | String | `gemma4:latest`          | Default LLM model (`make dev` sets this when using Ollama)                                                       |
| `OLLAMA_EMBEDDING_MODEL` | String | `embeddinggemma:latest`  | Default embedding model (`make dev` sets this when using Ollama)                                                 |
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

#### Google Gemini (Developer API)

| Variable          | Type   | Default                                     | Description       |
| ----------------- | ------ | ------------------------------------------- | ----------------- |
| `GEMINI_API_KEY`  | String | None                                        | Google AI API key |
| `GOOGLE_API_KEY`  | String | None                                        | Alias for Gemini  |
| `GEMINI_BASE_URL` | String | `https://generativelanguage.googleapis.com` | API endpoint      |

> **Not Vertex AI.** Gemini Developer API uses a static API key. Enterprise Vertex AI uses OAuth2 identity — see below.

#### Google Vertex AI (Enterprise)

Vertex AI (`vertexai` provider) authenticates with **short-lived OAuth2 bearer tokens** minted from GCP identity — not a static API key. The Settings → Provider Status Hub shows **Identity (ADC)** and structured requirements.

| Variable                         | Type   | Default        | Description                                                                 |
| -------------------------------- | ------ | -------------- | --------------------------------------------------------------------------- |
| `GOOGLE_CLOUD_PROJECT`           | String | None           | **Required.** GCP project ID                                                |
| `GOOGLE_CLOUD_REGION`            | String | `us-central1`  | Regional endpoint (`{region}-aiplatform.googleapis.com`)                    |
| `GOOGLE_CLOUD_LOCATION`          | String | —              | Alias for region (some Google SDKs)                                         |
| `GOOGLE_ACCESS_TOKEN`            | String | None           | Explicit bearer token (~1 h TTL; CI/debug only)                             |
| `GOOGLE_APPLICATION_CREDENTIALS` | String | None           | Path to service account JSON or WIF config                                  |

**Auth resolution ladder** (first match wins at runtime):

1. `GOOGLE_ACCESS_TOKEN` — use as-is
2. GCE/GKE/Cloud Run metadata server (attached service account; auto-refresh)
3. ADC file (`~/.config/gcloud/application_default_credentials.json`)
4. Service account key via `GOOGLE_APPLICATION_CREDENTIALS`
5. `gcloud auth application-default print-access-token` (local dev)

**Local development:**

```bash
# Correct ADC login (common mistake: swapping the last two words)
gcloud auth application-default login

export GOOGLE_CLOUD_PROJECT=your-gcp-project
export GOOGLE_CLOUD_REGION=europe-west1   # optional

# If ~/.edgequake/models.toml lacks vertexai, use the bundled catalog:
export EDGEQUAKE_MODELS_CONFIG=/path/to/edgequake/edgequake/models.toml

make dev
```

**Production:** Prefer an attached workload service account (GCE/GKE/Cloud Run) with `roles/aiplatform.user`. Avoid long-lived SA key files when metadata-based auth is available.

**Stale ADC:** An expired token file may show requirements as satisfied while health remains offline — re-run `gcloud auth application-default login`.

Design reference: [SPEC-043 §011 — Vertex AI Authentication](../specs/043-update-edgequake-llm/011-vertexai-authentication.md).

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

> **Three layers of defaults** — docs and operators often conflate these:
>
> | Layer | When it applies | LLM provider / model | Embedding / dim |
> | ----- | --------------- | -------------------- | --------------- |
> | **Bundled catalog** (`models.toml` `[defaults]`, compiled constants) | No env overrides, direct `cargo run` | `openai` / `gpt-4.1-mini` | `text-embedding-3-small` / `1536` |
> | **`make dev`** (no `OPENAI_API_KEY`) | Local stack via Makefile | `ollama` / `gemma4:latest` | `embeddinggemma:latest` / `768` |
> | **`make dev`** (with `OPENAI_API_KEY`) | Local stack via Makefile | `openai` / `gpt-5-nano`¹ | `text-embedding-3-small` / `1536` |
>
> ¹ `gpt-5-nano` is set by the Makefile when an API key is present; the bundled catalog default LLM is `gpt-4.1-mini`. Override with `EDGEQUAKE_DEFAULT_LLM_MODEL` if needed.

**Primary variables** (recommended — `make dev` sets these):

| Variable                              | Type    | Default (bundled) | Description                         |
| ------------------------------------- | ------- | ----------------- | ----------------------------------- |
| `EDGEQUAKE_DEFAULT_LLM_PROVIDER`      | String  | `openai`          | Default LLM provider (priority 1)   |
| `EDGEQUAKE_DEFAULT_LLM_MODEL`         | String  | `gpt-4.1-mini`    | Default LLM model (priority 1)      |
| `EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER`| String  | `openai`          | Default embedding provider          |
| `EDGEQUAKE_DEFAULT_EMBEDDING_MODEL`   | String  | `text-embedding-3-small` | Default embedding model      |
| `EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION` | Integer | `1536`          | Default embedding vector dimension  |

**Secondary / deployment variables** (single-env aliases, lower priority than `EDGEQUAKE_DEFAULT_*`):

| Variable                        | Type    | Default (bundled) | Description                                    |
| ------------------------------- | ------- | ----------------- | ---------------------------------------------- |
| `EDGEQUAKE_MODELS_CONFIG`       | String  | None              | Path to custom models.toml                     |
| `EDGEQUAKE_LLM_PROVIDER`        | String  | (see above)       | LLM provider alias                             |
| `EDGEQUAKE_LLM_MODEL`           | String  | None              | LLM model alias                                |
| `EDGEQUAKE_EMBEDDING_PROVIDER`  | String  | (see above)       | Embedding provider alias (hybrid mode)         |
| `EDGEQUAKE_EMBEDDING_MODEL`     | String  | None              | Embedding model alias                          |
| `EDGEQUAKE_EMBEDDING_DIMENSION` | Integer | `1536`            | Embedding vector dimension alias               |

**Vision / PDF extraction** (inherits from LLM defaults when unset):

| Variable                   | Type   | Default              | Description                          |
| -------------------------- | ------ | -------------------- | ------------------------------------ |
| `EDGEQUAKE_VISION_PROVIDER`| String | same as LLM provider | Vision LLM provider for PDF→Markdown |
| `EDGEQUAKE_VISION_MODEL`   | String | same as LLM model    | Vision LLM model for PDF→Markdown    |

### Application Attribution (SPEC-043)

Identifies EdgeQuake to upstream LLM providers (OpenRouter HTTP referer, OpenAI client request ID, Anthropic application ID, Google `x-goog-api-client`, etc.). Built once per request via `ApplicationContext` and passed to `create_llm_provider_with_context`.

| Variable | Type | Default | Description |
| -------- | ---- | ------- | ----------- |
| `EDGEQUAKE_APP_ID` | String | None | Stable application identifier sent upstream |
| `EDGEQUAKE_APP_NAME` | String | None | Human-readable application name (OpenRouter title) |
| `EDGEQUAKE_APP_URL` | String | None | Application URL (OpenRouter HTTP-Referer) |
| `EDGEQUAKE_TENANT_ID` | String | None | Optional tenant identifier for multi-tenant attribution |

**Per-request overrides** (merged into `ApplicationContext` when present):

| Header | Maps to |
| ------ | ------- |
| `x-edgequake-app-id` | `app_id` |
| `x-edgequake-app-name` | `app_name` |
| `x-edgequake-app-url` | `app_url` |
| `x-edgequake-tenant-id` | `tenant_id` |
| `x-edgequake-request-id` | `request_id` |

**Example — identify EdgeQuake to OpenRouter:**

```bash
export EDGEQUAKE_APP_ID=edgequake
export EDGEQUAKE_APP_NAME="EdgeQuake"
export EDGEQUAKE_APP_URL=https://edgequake.example.com
```

**Resolution order for attribution fields:** `server_config.app_attribution` → overridden by env vars (`EDGEQUAKE_APP_*`) → overridden per-request by ingress headers.

**WebUI persistence:** Settings → Application Attribution → PATCH `/api/v1/settings/app-attribution` (admin, PostgreSQL). Applied immediately without restart. See [REST API — Application Attribution](/docs/api-reference/rest-api#application-attribution).

**Discovery:** `GET /api/v1/settings/attribution` returns the effective context plus per-provider header catalog. `/health` includes a compact `attribution` block (`app_id`, `app_name`, `active`).

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
| `EDGEQUAKE_EMBEDDING_DIMENSION` | Integer | `1536` (OpenAI) / `768` (Ollama) | Vector dimension for the embedding override |

**Priority:** `EDGEQUAKE_EMBEDDING_PROVIDER` → `OLLAMA_EMBEDDING_HOST` → default (from `from_env()`).

**Example — OpenAI for LLM, dedicated Ollama node for embeddings:**

```bash
export EDGEQUAKE_LLM_PROVIDER=openai
export OPENAI_API_KEY=sk-...
export OLLAMA_EMBEDDING_HOST=http://gpu-box:11434
export OLLAMA_EMBEDDING_MODEL=nomic-embed-text
```

### Pipeline Timeout & Concurrency (fixes [#194](https://github.com/raphaelmansuy/edgequake/issues/194))

Controls how aggressively the ingestion pipeline calls the LLM and how long it waits for each
response. These are the knobs to reach for when processing **large documents** or using **slow
local LLMs** (Ollama, LM Studio on CPU or a single GPU).

| Variable                               | Type    | Default | Min  | Max     | Description                                                |
| -------------------------------------- | ------- | ------- | ---- | ------- | ---------------------------------------------------------- |
| `EDGEQUAKE_CHUNK_TIMEOUT_SECS`         | Integer | `180`   | `10` | ∞       | Per-chunk LLM call timeout in seconds                      |
| `EDGEQUAKE_CHUNK_MAX_RETRIES`          | Integer | `3`     | `0`  | `20`    | Max retry attempts per chunk on timeout or error           |
| `EDGEQUAKE_CHUNK_RETRY_DELAY_MS`       | Integer | `1000`  | `0`  | `60000` | Initial backoff delay between retries (milliseconds)       |
| `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` | Integer | `16`    | `1`  | `256`   | Max parallel LLM extraction calls per document             |
| `EDGEQUAKE_LLM_TIMEOUT_SECS`           | Integer | `600`   | —    | `3600`  | HTTP safety-layer timeout (Layer 2, supports up to 1 hour) |

**Two-layer timeout architecture:**

```
┌─────────────────────────────────────────────────────────────────┐
│                   TIMEOUT LAYERS                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Layer 1 — EDGEQUAKE_CHUNK_TIMEOUT_SECS  (pipeline, fires first)│
│    └─ Set this to allow the LLM enough time per chunk           │
│                                                                 │
│  Layer 2 — EDGEQUAKE_LLM_TIMEOUT_SECS   (HTTP safety cap)      │
│    └─ Must be ≥ EDGEQUAKE_CHUNK_TIMEOUT_SECS                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Quick configuration for slow local LLMs:**

```bash
# Large document on a single-GPU Ollama instance
export EDGEQUAKE_CHUNK_TIMEOUT_SECS=600       # 10 minutes per chunk
export EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS=4  # reduce parallelism
export EDGEQUAKE_LLM_TIMEOUT_SECS=3600        # 1-hour HTTP cap
```

> **Note:** Values below the allowed minimum are automatically clamped.
> Non-numeric values are silently ignored and the default is used.

---

### Security / Authentication

| Variable                    | Type    | Default | Description                                                                 |
| --------------------------- | ------- | ------- | --------------------------------------------------------------------------- |
| `EDGEQUAKE_AUTH_ENABLED`    | Boolean | `true`  | Enable API authentication (secure default)                                  |
| `EDGEQUAKE_DEV_MODE`        | Boolean | `false` | Set `true` for local open API without keys (`make dev` sets this)           |
| `EDGEQUAKE_MASTER_API_KEY`  | String  | None    | Master API key for authenticated requests                                   |
| `EDGEQUAKE_API_KEYS`        | String  | None    | Comma-separated API keys (alternative to master key)                        |
| `EDGEQUAKE_STRICT_STARTUP`  | Boolean | `false` | Exit on insecure production config when `1`                                 |
| `EDGEQUAKE_CORS_ORIGINS`    | String  | None    | Comma-separated allowed CORS origins (`None` = allow any, legacy default)   |

### Security / Frontend

| Variable                         | Type   | Default | Description                                                                                                                             |
| -------------------------------- | ------ | ------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `NEXT_PUBLIC_DISABLE_DEMO_LOGIN` | String | `false` | Set to `true` to hide the demo "skip login" button in production (closes [#139](https://github.com/raphaelmansuy/edgequake/issues/139)) |

> **Production tip:** Keep `EDGEQUAKE_AUTH_ENABLED=true`, unset `EDGEQUAKE_DEV_MODE`, configure `EDGEQUAKE_MASTER_API_KEY` or `EDGEQUAKE_API_KEYS`, and set `NEXT_PUBLIC_DISABLE_DEMO_LOGIN=true` in your frontend build.

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
# Default provider selection (bundled models.toml)
[defaults]
llm_provider = "openai"
llm_model = "gpt-4.1-mini"
embedding_provider = "openai"
embedding_model = "text-embedding-3-small"
vision_provider = "openai"
vision_model = "gpt-4o"

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
name = "gpt-4.1-mini"
display_name = "GPT-4.1 Mini"
model_type = "llm"                   # or "embedding"
description = "Cost-effective model with 1M context"
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
| `mistral`    | Mistral AI models       | `MISTRAL_API_KEY`      |
| `gemini`     | Google Gemini Developer API | `GEMINI_API_KEY` / `GOOGLE_API_KEY` |
| `vertexai`   | Google Vertex AI (enterprise) | **Identity** — `GOOGLE_CLOUD_PROJECT` + ADC/SA (no static API key) |
| `xai`        | xAI Grok models         | `XAI_API_KEY`          |
| `openrouter` | OpenRouter aggregator   | `OPENROUTER_API_KEY`   |
| `minimax`    | MiniMax AI models       | `MINIMAX_API_KEY`      |
| `azure`      | Azure OpenAI            | `AZURE_OPENAI_API_KEY` |
| `ollama`     | Ollama local models     | None (local)           |
| `lmstudio`   | LM Studio local         | None (local)           |
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
name = "gpt-4.1-mini"
display_name = "GPT-4.1 Mini"
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
name = "gemma4:latest"
display_name = "Gemma 4 Latest"
model_type = "llm"
tags = ["recommended", "local"]

[providers.models.capabilities]
context_length = 128000
max_output_tokens = 8192
supports_vision = true
supports_streaming = true

[[providers.models]]
name = "embeddinggemma:latest"
display_name = "Embedding Gemma"
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
name = "gpt-4.1-mini"  # Your deployment name
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

### Google Vertex AI (Enterprise)

Vertex uses IAM identity auth — leave `api_key_env` empty in `models.toml`:

```toml
[[providers]]
name = "vertexai"
display_name = "Google Vertex AI"
type = "vertexai"
api_base = "https://aiplatform.googleapis.com"
api_key_env = ""   # OAuth2 identity — not a static key
enabled = true
priority = 7
description = "Enterprise Gemini on Vertex AI; IAM / ADC authentication"

[[providers.models]]
name = "gemini-2.5-flash"
display_name = "Gemini 2.5 Flash (Vertex)"
model_type = "llm"
tags = ["recommended", "enterprise"]

[providers.models.capabilities]
context_length = 1000000
max_output_tokens = 8192
supports_vision = true
supports_streaming = true

[[providers.models]]
name = "gemini-embedding-001"
display_name = "Gemini Embedding (Vertex)"
model_type = "embedding"

[providers.models.capabilities]
context_length = 10000
embedding_dimension = 3072
```

Set `GOOGLE_CLOUD_PROJECT` and authenticate via ADC or an attached service account before selecting `vertexai` in the UI or `EDGEQUAKE_LLM_PROVIDER=vertexai`.

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
name = "openai/gpt-4o-mini"
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
# Get current effective configuration (resolution chain)
curl http://localhost:8080/api/v1/config/effective | jq .

# List available providers (Settings UI source)
curl http://localhost:8080/api/v1/settings/providers

# List all models grouped by provider
curl http://localhost:8080/api/v1/models

# Get models for a specific provider
curl http://localhost:8080/api/v1/models/openai

# Query with specific provider (per-request)
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What is quantum computing?",
    "mode": "hybrid",
    "llm_provider": "openai",
    "llm_model": "gpt-4.1-mini"
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
| `max_tokens`   | Per query | 16384   | Max response tokens (HTTP safety layer default) |

---

## Example Configurations

### Development (Minimal)

```bash
# Requires DATABASE_URL — set via .env or environment
# Bundled default: openai/gpt-4.1-mini (requires OPENAI_API_KEY)
# make dev without API key uses ollama/gemma4:latest instead
make dev
```

### Development with Ollama

```bash
export OLLAMA_HOST="http://localhost:11434"
export EDGEQUAKE_DEFAULT_LLM_PROVIDER=ollama
export EDGEQUAKE_DEFAULT_LLM_MODEL="gemma4:latest"
export EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER=ollama
export EDGEQUAKE_DEFAULT_EMBEDDING_MODEL="embeddinggemma:latest"
make dev
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
