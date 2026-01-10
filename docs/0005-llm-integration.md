# EdgeQuake LLM Integration

> **Implements**: [FEAT0020](features.md#feat0020) LLM Provider Abstraction, [FEAT0021](features.md#feat0021) OpenAI Integration
>
> Complete guide to configuring LLM providers and embedding models

**Version**: 2.0.0 | **Last Updated**: January 2026

> **Code Reference**: See [edgequake/crates/edgequake-llm/](../edgequake/crates/edgequake-llm/) for provider implementations

---

## Quick Provider Selection

| Provider         | LLM            | Embeddings                | Dimension | Cost | Best For               |
| ---------------- | -------------- | ------------------------- | --------- | ---- | ---------------------- |
| **OpenAI**       | ✅ gpt-4o-mini | ✅ text-embedding-3-small | 1536      | $$   | Production, quality    |
| **Azure OpenAI** | ✅             | ✅                        | 1536      | $$   | Enterprise, compliance |
| **Ollama**       | ✅ gemma3:12b  | ✅ embeddinggemma:latest  | **768**   | Free | Local dev, privacy     |
| **LM Studio**    | ✅ (custom)    | ✅ (compatible)           | 1536 typ. | Free | Local experimentation  |
| **Mock**         | ✅             | ✅                        | 1536      | Free | Testing, CI/CD         |

### Cost Estimation (OpenAI)

| Operation    | Model                  | Cost per 1K tokens | Typical Usage          |
| ------------ | ---------------------- | ------------------ | ---------------------- |
| Embedding    | text-embedding-3-small | $0.00002           | ~$0.02 per 1000 chunks |
| LLM (input)  | gpt-4o-mini            | $0.00015           | ~$0.15 per 100 queries |
| LLM (output) | gpt-4o-mini            | $0.0006            | ~$0.06 per 100 queries |

> **Enforces**: [BR0020](business_rules.md#br0020) Cost Tracking - All LLM calls are metered
> **NEW**: [Provider Switching Guide](#provider-switching) - Easy switching between providers

---

## Table of Contents

1. [Overview](#overview)
2. [Provider Architecture](#provider-architecture)
3. [OpenAI Integration](#openai-integration)
4. [Ollama Integration](#ollama-integration)
5. [OpenAI-Compatible APIs](#openai-compatible-apis)
6. [Mock Provider](#mock-provider)
7. [Configuration Reference](#configuration-reference)
8. **[Provider Switching](#provider-switching)** ← NEW
9. [Best Practices](#best-practices)
10. [Troubleshooting](#troubleshooting)
11. [Next Steps](#next-steps)

---

## Overview

EdgeQuake requires two AI components for RAG operations:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        LLM Integration Architecture                     │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                     EdgeQuake Core                               │   │
│  └───────────────────────────┬─────────────────────────────────────┘   │
│                              │                                          │
│              ┌───────────────┴───────────────┐                         │
│              │                               │                         │
│              ▼                               ▼                         │
│  ┌───────────────────────┐       ┌───────────────────────┐            │
│  │    LLMProvider        │       │   EmbeddingProvider   │            │
│  │    (Text Generation)  │       │   (Vector Creation)   │            │
│  └───────────┬───────────┘       └───────────┬───────────┘            │
│              │                               │                         │
│              ▼                               ▼                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      Provider Implementations                    │   │
│  │                                                                  │   │
│  │       OpenAI  │  Ollama  │  LM Studio  │  Mock (Testing)        │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### LLM Provider Uses

- Entity and relationship extraction from text chunks
- Description summarization during entity merging
- Query keyword extraction
- Answer generation from retrieved context

### Embedding Provider Uses

- Converting text chunks to vectors for similarity search
- Converting entities/relationships to vectors
- Query embedding for retrieval

---

## Provider Architecture

### Provider Traits

> **Code Reference**: [edgequake/crates/edgequake-llm/src/traits.rs](../edgequake/crates/edgequake-llm/src/traits.rs)

```rust
// Located: edgequake/crates/edgequake-llm/src/traits.rs

#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Get the name of this provider.
    fn name(&self) -> &str;

    /// Get the current model.
    fn model(&self) -> &str;

    /// Get the maximum context length for the model.
    fn max_context_length(&self) -> usize;

    /// Generate a completion for the given prompt.
    async fn complete(&self, prompt: &str) -> Result<LLMResponse>;

    /// Generate a completion with custom options.
    async fn complete_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> Result<LLMResponse>;

    /// Generate a chat completion with messages.
    async fn chat(
        &self,
        messages: &[ChatMessage],
        options: Option<&CompletionOptions>,
    ) -> Result<LLMResponse>;

    /// Generate a streaming completion.
    async fn stream(&self, prompt: &str) -> Result<BoxStream<'static, Result<String>>>;

    /// Check if the model supports streaming.
    fn supports_streaming(&self) -> bool;

    /// Check if the model supports JSON mode.
    fn supports_json_mode(&self) -> bool;
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Get the name of this provider.
    fn name(&self) -> &str;

    /// Get the embedding model.
    fn model(&self) -> &str;

    /// Get the dimension of the embeddings.
    fn dimension(&self) -> usize;

    /// Get the maximum number of tokens per input.
    fn max_tokens(&self) -> usize;

    /// Generate embeddings for a batch of texts.
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Generate embedding for a single text.
    async fn embed_one(&self, text: &str) -> Result<Vec<f32>>;
}

pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub name: Option<String>,
}

pub enum ChatRole {
    System,
    User,
    Assistant,
    Function,
}

#[derive(Debug, Clone, Default)]
pub struct CompletionOptions {
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub response_format: Option<String>,
    pub system_prompt: Option<String>,
}

pub struct LLMResponse {
    pub content: String,
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    pub model: String,
    pub finish_reason: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

### Supported Providers

| Provider      | LLM | Embeddings | Production | Notes             |
| ------------- | --- | ---------- | ---------- | ----------------- |
| **OpenAI**    | ✅  | ✅         | ✅         | Recommended       |
| **Ollama**    | ✅  | ✅         | ✅         | Local inference   |
| **LM Studio** | ✅  | ✅         | ✅         | OpenAI-compatible |
| **Mock**      | ✅  | ✅         | ❌         | Testing only      |

---

## OpenAI Integration

### Basic Setup

```rust
use edgequake_llm::OpenAIProvider;
use std::sync::Arc;

// Create provider with API key
let api_key = std::env::var("OPENAI_API_KEY")?;
let provider = Arc::new(
    OpenAIProvider::new(&api_key)
        .with_model("gpt-4o-mini")
        .with_embedding_model("text-embedding-3-small")
);

// Use for both LLM and embeddings
let llm_provider: Arc<dyn LLMProvider> = provider.clone();
let embedding_provider: Arc<dyn EmbeddingProvider> = provider.clone();
```

### Environment Variables

```bash
# Required
OPENAI_API_KEY=sk-...

# Optional (defaults shown)
EDGEQUAKE_LLM_MODEL=gpt-4o-mini
EDGEQUAKE_EMBEDDING_MODEL=text-embedding-3-small
OPENAI_BASE_URL=https://api.openai.com/v1
```

### Available Models

#### LLM Models

| Model           | Context | Cost (Input/Output per 1M) | Notes           |
| --------------- | ------- | -------------------------- | --------------- |
| `gpt-4o-mini`   | 128K    | $0.15 / $0.60              | **Recommended** |
| `gpt-4o`        | 128K    | $2.50 / $10.00             | Highest quality |
| `gpt-4-turbo`   | 128K    | $10.00 / $30.00            | Legacy          |
| `gpt-3.5-turbo` | 16K     | $0.50 / $1.50              | Budget option   |
| `o1-mini`       | 128K    | $3.00 / $12.00             | Reasoning model |

#### Embedding Models

| Model                    | Dimensions | Cost (per 1M tokens) | Notes           |
| ------------------------ | ---------- | -------------------- | --------------- |
| `text-embedding-3-small` | 1536       | $0.02                | **Recommended** |
| `text-embedding-3-large` | 3072       | $0.13                | Higher quality  |
| `text-embedding-ada-002` | 1536       | $0.10                | Legacy          |

### Cost Estimation

For `gpt-4o-mini` + `text-embedding-3-small`:

| Operation                     | Tokens             | Cost     |
| ----------------------------- | ------------------ | -------- |
| Entity extraction (per chunk) | ~800 in + 200 out  | $0.0012  |
| Embedding (per chunk)         | ~1000              | $0.00002 |
| Query generation              | ~2000 in + 500 out | $0.006   |

**Per document (avg 5 chunks):** ~$0.007

---

## Ollama Integration

### Prerequisites

```bash
# Install Ollama
curl -fsSL https://ollama.ai/install.sh | sh

# Pull models
ollama pull llama3.2:3b       # LLM
ollama pull nomic-embed-text  # Embeddings
```

### Setup

```rust
use edgequake_llm::OpenAIProvider;

// Ollama exposes OpenAI-compatible API
let provider = Arc::new(
    OpenAIProvider::compatible("ollama", "http://localhost:11434/v1")
        .with_model("llama3.2:3b")
        .with_embedding_model("nomic-embed-text")
);
```

### Environment Variables

```bash
OLLAMA_HOST=http://localhost:11434
EDGEQUAKE_LLM_MODEL=llama3.2:3b
EDGEQUAKE_EMBEDDING_MODEL=nomic-embed-text
```

### Recommended Ollama Models

| Model               | Size  | Context | Notes                     |
| ------------------- | ----- | ------- | ------------------------- |
| `llama3.2:3b`       | 2GB   | 128K    | Fast, good for extraction |
| `llama3.1:8b`       | 4.7GB | 128K    | Better quality            |
| `mistral:7b`        | 4GB   | 32K     | Good balance              |
| `phi3:mini`         | 2GB   | 4K      | Lightweight               |
| `nomic-embed-text`  | 274MB | -       | Embeddings                |
| `mxbai-embed-large` | 670MB | -       | Higher quality embeddings |

---

## OpenAI-Compatible APIs

EdgeQuake works with any OpenAI-compatible API.

### LM Studio

```rust
let provider = Arc::new(
    OpenAIProvider::compatible("lm-studio", "http://localhost:1234/v1")
        .with_model("local-model")
);
```

### Azure OpenAI

```rust
let provider = Arc::new(
    OpenAIProvider::new(&api_key)
        .with_base_url("https://your-resource.openai.azure.com/openai/deployments/gpt-4")
        .with_model("gpt-4")
);
```

### Groq

```rust
let provider = Arc::new(
    OpenAIProvider::compatible(&groq_api_key, "https://api.groq.com/openai/v1")
        .with_model("llama-3.1-70b-versatile")
);
```

### Together AI

```rust
let provider = Arc::new(
    OpenAIProvider::compatible(&together_api_key, "https://api.together.xyz/v1")
        .with_model("meta-llama/Llama-3-70b-chat-hf")
);
```

---

## Mock Provider

For testing without API calls.

### Features

- Deterministic outputs for testing
- No API costs
- Fast execution
- Queue-based response system

### Usage

```rust
use edgequake_llm::MockProvider;

// Create mock provider with default responses
let mock = Arc::new(MockProvider::new());

// Add custom responses to the queue
mock.add_response("Custom response 1").await;
mock.add_response("Custom response 2").await;

// Add custom embeddings
mock.add_embedding(vec![0.1; 1536]).await;
```

### Implementation Details

```rust
// Located: edgequake/crates/edgequake-llm/src/providers/mock.rs

pub struct MockProvider {
    responses: Arc<Mutex<Vec<String>>>,
    embeddings: Arc<Mutex<Vec<Vec<f32>>>>,
}

impl MockProvider {
    /// Create a new mock provider with default responses.
    pub fn new() -> Self { ... }

    /// Add a response to the queue (consumed in order).
    pub async fn add_response(&self, response: impl Into<String>) { ... }

    /// Add an embedding to the queue.
    pub async fn add_embedding(&self, embedding: Vec<f32>) { ... }
}
```

### Automatic Selection

EdgeQuake automatically uses MockProvider when `OPENAI_API_KEY` is not set:

```rust
// In tests - uses mock automatically
async fn create_provider() -> (Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>) {
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        if !api_key.is_empty() && api_key != "test-key" {
            let provider = Arc::new(OpenAIProvider::new(&api_key));
            return (provider.clone(), provider);
        }
    }

    // Fall back to mock
    let mock = Arc::new(MockProvider::new());
    (mock.clone(), mock)
}
```

### Running Tests

```bash
# With mock provider (fast, free)
cargo test

# With real OpenAI (slower, costs money)
export OPENAI_API_KEY="sk-..."
cargo test -- --nocapture
```

---

## Configuration Reference

### LLM Configuration Structure

```rust
pub struct LlmConfig {
    /// Provider name: openai, ollama
    pub provider: String,

    /// API key (optional for local providers)
    pub api_key: Option<String>,

    /// Custom API base URL
    pub base_url: Option<String>,

    /// LLM model name
    pub model: String,

    /// Embedding model name
    pub embedding_model: String,

    /// Embedding vector dimension
    pub embedding_dim: usize,

    /// Maximum tokens for generation
    pub max_tokens: usize,

    /// Temperature (0.0 = deterministic)
    pub temperature: f32,

    /// Request timeout in seconds
    pub timeout_secs: u64,

    /// Maximum retries for failed requests
    pub max_retries: u32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            api_key: None,
            base_url: None,
            model: "gpt-4o-mini".to_string(),
            embedding_model: "text-embedding-3-small".to_string(),
            embedding_dim: 1536,
            max_tokens: 4096,
            temperature: 0.0,
            timeout_secs: 60,
            max_retries: 3,
        }
    }
}
```

### Environment Variables

| Variable                    | Default                | Description              |
| --------------------------- | ---------------------- | ------------------------ |
| `OPENAI_API_KEY`            | -                      | OpenAI API key           |
| `EDGEQUAKE_LLM_PROVIDER`    | openai                 | Provider: openai, ollama |
| `EDGEQUAKE_LLM_MODEL`       | gpt-4o-mini            | LLM model name           |
| `EDGEQUAKE_EMBEDDING_MODEL` | text-embedding-3-small | Embedding model          |
| `EDGEQUAKE_EMBEDDING_DIM`   | 1536                   | Embedding dimension      |
| `OPENAI_BASE_URL`           | -                      | Custom API endpoint      |
| `OLLAMA_HOST`               | http://localhost:11434 | Ollama server URL        |

---

## Provider Switching

> **NEW**: As of v2.1.0, EdgeQuake supports automatic provider detection and easy switching between OpenAI, Ollama, LM Studio, and Mock providers.

### Environment-Based Provider Selection

EdgeQuake automatically detects which LLM provider to use based on environment variables. This makes it easy to switch between providers without code changes:

```bash
# OpenAI (Cloud) - Recommended for production
export OPENAI_API_KEY="sk-..."
cargo run  # Auto-selects OpenAI provider

# Ollama (Local) - Recommended for development
export OLLAMA_HOST="http://localhost:11434"
cargo run  # Auto-selects Ollama provider

# LM Studio (Local) - For experimentation
export EDGEQUAKE_LLM_PROVIDER=lmstudio
export OPENAI_BASE_URL="http://localhost:1234/v1"
export OPENAI_API_KEY="lm-studio"  # Can be any value
cargo run  # Uses LM Studio via OpenAI-compatible API

# Mock Provider (Testing) - No external dependencies
cargo test  # Auto-selects Mock provider
```

### Provider Auto-Detection Priority

When `EDGEQUAKE_LLM_PROVIDER` is **not explicitly set**, EdgeQuake uses this detection order:

1. **Ollama**: If `OLLAMA_HOST` or `OLLAMA_MODEL` is set
2. **OpenAI**: If `OPENAI_API_KEY` is set
3. **Mock**: Fallback for testing (no API calls)

**Example:**

```bash
# Both variables set - which one wins?
export OPENAI_API_KEY="sk-..."
export OLLAMA_HOST="http://localhost:11434"
cargo run
# Result: Ollama selected (higher priority)

# Force OpenAI explicitly
export EDGEQUAKE_LLM_PROVIDER=openai
export OPENAI_API_KEY="sk-..."
export OLLAMA_HOST="http://localhost:11434"  # Ignored
cargo run
# Result: OpenAI selected (explicit override)
```

### Quick Start by Provider

#### OpenAI (Cloud)

**Best for:** Production deployments, highest quality

```bash
# Set API key
export OPENAI_API_KEY="sk-proj-..."

# Optional: Customize models
export EDGEQUAKE_LLM_MODEL="gpt-4o-mini"
export EDGEQUAKE_EMBEDDING_MODEL="text-embedding-3-small"

# Run EdgeQuake
cargo run --release
```

**Vector Dimension:** 1536 (auto-detected)

#### Ollama (Local)

**Best for:** Local development, privacy-focused deployments

```bash
# Step 1: Install and start Ollama
ollama serve &

# Step 2: Pull models (NEW DEFAULTS)
ollama pull gemma3:12b          # LLM model (upgraded from llama3)
ollama pull embeddinggemma:latest  # Embedding model (upgraded from nomic-embed-text)

# Step 3: Configure EdgeQuake
export OLLAMA_HOST="http://localhost:11434"

# Optional: Use different models
export OLLAMA_MODEL="llama3.1:70b"
export OLLAMA_EMBEDDING_MODEL="nomic-embed-text"

# Run EdgeQuake
cargo run
```

**Vector Dimension:** 768 (auto-detected, different from OpenAI!)

⚠️ **Important**: Ollama uses 768-dimensional embeddings. If you switch from OpenAI (1536 dimensions) to Ollama, you must recreate your PostgreSQL database or migrate vectors.

#### LM Studio (Local)

**Best for:** Experimenting with custom models

```bash
# Step 1: Download and start LM Studio
# - Enable "Server" mode in LM Studio settings
# - Default port: 1234

# Step 2: Configure EdgeQuake
export EDGEQUAKE_LLM_PROVIDER=lmstudio
export OPENAI_BASE_URL="http://localhost:1234/v1"
export OPENAI_API_KEY="lm-studio"  # Can be any value

# Optional: Specify model name
export OPENAI_MODEL="gemma-3n-e4b-it-mlxmodel"

# Run EdgeQuake
cargo run
```

**Vector Dimension:** Varies by model (typically 1536, auto-detected)

**Note:** LM Studio uses OpenAI-compatible API, so it reuses the OpenAI provider implementation internally.

#### Mock Provider (Testing)

**Best for:** CI/CD, unit tests, development without API costs

```bash
# Automatically used when no other provider configured
cargo test

# Or explicitly enable
export EDGEQUAKE_LLM_PROVIDER=mock
cargo run
```

**Vector Dimension:** 1536 (compatible with OpenAI format)

### Provider Comparison

| Feature        | OpenAI     | Ollama         | LM Studio         | Mock       |
| -------------- | ---------- | -------------- | ----------------- | ---------- |
| **Cost**       | $$         | Free           | Free              | Free       |
| **Latency**    | 200-500ms  | 100-2000ms†    | 100-2000ms†       | <1ms       |
| **Quality**    | Excellent  | Good           | Varies            | Synthetic  |
| **Privacy**    | Cloud      | **Local only** | **Local only**    | Local only |
| **Internet**   | Required   | No             | No                | No         |
| **Vector Dim** | 1536       | **768**        | 1536 typical      | 1536       |
| **Setup**      | API key    | Install Ollama | Install LM Studio | None       |
| **Best For**   | Production | Development    | Experimentation   | Testing    |

† Latency depends on local hardware and model size

### Switching Providers (Step-by-Step)

#### Scenario: OpenAI → Ollama (Local Development)

**Problem:** You want to develop locally without API costs.

**Solution:**

```bash
# Step 1: Install Ollama (if not already)
brew install ollama  # macOS
# or: curl -fsSL https://ollama.com/install.sh | sh  # Linux

# Step 2: Start Ollama service
ollama serve &

# Step 3: Pull required models
ollama pull gemma3:12b
ollama pull embeddinggemma:latest

# Step 4: Verify models available
ollama list
# Expected output:
# NAME                        SIZE
# gemma3:12b                  7.4 GB
# embeddinggemma:latest       274 MB

# Step 5: Configure environment
export OLLAMA_HOST="http://localhost:11434"
# Remove OpenAI key (or it will be ignored due to priority)
unset OPENAI_API_KEY

# Step 6: ⚠️ CRITICAL - Recreate database (dimension change)
psql -c "DROP DATABASE IF EXISTS edgequake; CREATE DATABASE edgequake;"
# Why? OpenAI uses 1536 dimensions, Ollama uses 768 dimensions

# Step 7: Run EdgeQuake
cargo run
# Check logs for: "Using vector dimension 768 from ollama provider"
```

#### Scenario: Ollama → OpenAI (Production Deployment)

**Problem:** You developed locally with Ollama, now deploying to production with OpenAI.

**Solution:**

```bash
# Step 1: Set OpenAI API key
export OPENAI_API_KEY="sk-proj-..."

# Step 2: Remove Ollama configuration
unset OLLAMA_HOST
unset OLLAMA_MODEL

# Step 3: ⚠️ CRITICAL - Recreate database (dimension change)
# On production database:
psql $DATABASE_URL -c "DROP DATABASE IF EXISTS edgequake; CREATE DATABASE edgequake;"
# Why? Ollama uses 768 dimensions, OpenAI uses 1536 dimensions

# Step 4: Run migrations
cargo run -- migrate

# Step 5: Deploy and verify
cargo run --release
# Check logs for: "Using vector dimension 1536 from openai provider"
```

### Vector Dimension Migration (Advanced)

**Problem:** Existing database vectors don't match new provider's embedding dimension.

**Current Limitations:**

- EdgeQuake does not yet support automatic vector dimension migration
- Switching providers with different dimensions requires database recreation

**Workaround:**

1. Export documents: `curl http://localhost:8080/api/v1/documents > backup.json`
2. Drop and recreate database: `psql -c "DROP DATABASE edgequake; CREATE DATABASE edgequake;"`
3. Restart EdgeQuake with new provider
4. Re-upload documents: Use backup.json to restore content

**Future Enhancement (Planned):**

- Vector dimension migration utility
- Automatic re-embedding on provider change
- Dimension compatibility validation on startup

---

## Best Practices

### Production

```rust
// Use gpt-4o-mini for cost-effective production
let provider = OpenAIProvider::new(&api_key)
    .with_model("gpt-4o-mini")
    .with_embedding_model("text-embedding-3-small");
```

### Development

```rust
// Use Ollama for free local development
let provider = OpenAIProvider::compatible("ollama", "http://localhost:11434/v1")
    .with_model("llama3.2:3b");
```

### Testing

```rust
#[tokio::test]
async fn test_with_mock() {
    let mock = MockProvider::new();
    // Tests run instantly, no API calls
}
```

---

## Troubleshooting

| Problem                           | Cause                      | Solution                                                              |
| --------------------------------- | -------------------------- | --------------------------------------------------------------------- |
| "invalid api key"                 | Wrong or missing API key   | Verify `OPENAI_API_KEY` is set correctly                              |
| "model not found"                 | Invalid model name         | Check [OpenAI models](https://platform.openai.com/docs/models)        |
| "rate limit exceeded"             | Too many requests          | Implement exponential backoff, reduce concurrency                     |
| "context length exceeded"         | Prompt too long            | Reduce chunk size, use summarization                                  |
| Ollama "connection refused"       | Ollama not running         | Start with `ollama serve`                                             |
| Slow embeddings                   | Large batch size           | Reduce batch size, use async                                          |
| High costs                        | Wrong model                | Switch to gpt-4o-mini and text-embedding-3-small                      |
| **"Mock provider being used"**    | **No provider configured** | **Set `OPENAI_API_KEY` or `OLLAMA_HOST` explicitly**                  |
| **"Dimension mismatch error"**    | **Switched providers**     | **Recreate database - see [Provider Switching](#provider-switching)** |
| **"LM Studio connection failed"** | **Server not running**     | **Enable "Server" mode in LM Studio settings**                        |

### Debug Logging

```bash
# Enable LLM debug logging
RUST_LOG=edgequake_llm=debug ./target/release/edgequake

# Trace all API calls
RUST_LOG=edgequake_llm=trace ./target/release/edgequake

# Check provider selection at startup
RUST_LOG=edgequake_llm::factory=debug cargo run
# Look for: "Selected provider: openai/ollama/lmstudio/mock"
```

### Provider Selection Debugging

**Problem:** "EdgeQuake is using the wrong provider"

```bash
# Check which provider is being used
export RUST_LOG=edgequake_llm=debug
cargo run 2>&1 | grep -i "provider"
# Expected log: "Using vector dimension 1536 from openai provider"
# or:            "Using vector dimension 768 from ollama provider"

# Force specific provider
export EDGEQUAKE_LLM_PROVIDER=openai  # or: ollama, lmstudio, mock

# Verify environment variables
env | grep -E "OPENAI|OLLAMA|EDGEQUAKE_LLM"
```

**Problem:** "Vector dimension mismatch in PostgreSQL"

```bash
# Check current database vectors
psql $DATABASE_URL -c "SELECT dimension FROM embeddings LIMIT 1;"

# Compare with provider dimension
export RUST_LOG=edgequake_llm=debug
cargo run 2>&1 | grep "dimension"

# If mismatch detected:
# Option 1: Recreate database (data loss)
psql -c "DROP DATABASE edgequake; CREATE DATABASE edgequake;"

# Option 2: Switch back to original provider
export OPENAI_API_KEY="..."  # If was using OpenAI (1536 dim)
# or:
export OLLAMA_HOST="..."      # If was using Ollama (768 dim)
```

---

## Next Steps

| Your Goal             | Next Document                                              |
| --------------------- | ---------------------------------------------------------- |
| Deploy to production  | [Deployment Guide](0006-deployment-guide.md)               |
| Configure all options | [Configuration Reference](0007-configuration-reference.md) |
| Track LLM costs       | [Production LLM](production-llm-integration.md)            |

> **See Also**: [Features Registry](features.md) | [Cost Tracking](cost-tracking-sota-evaluation.md)

```rust
let result = provider.complete(&messages, options).await;
match result {
    Ok(response) => println!("Answer: {}", response.content),
    Err(LlmError::RateLimit) => {
        // Wait and retry
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
    Err(LlmError::ApiError(e)) => {
        tracing::error!("API error: {}", e);
    }
    Err(e) => return Err(e.into()),
}
```

---

## Next Steps

- **[Deployment Guide](0006-deployment-guide.md)** - Production deployment
- **[Configuration Reference](0007-configuration-reference.md)** - All config options
- **[production-llm-integration.md](production-llm-integration.md)** - Detailed production guide
