# EdgeQuake LLM Integration

> Complete guide to configuring LLM providers and embedding models

**Version**: 0.1.0 | **Last Updated**: December 2025

---

## Table of Contents

1. [Overview](#overview)
2. [Provider Architecture](#provider-architecture)
3. [OpenAI Integration](#openai-integration)
4. [Ollama Integration](#ollama-integration)
5. [OpenAI-Compatible APIs](#openai-compatible-apis)
6. [Mock Provider](#mock-provider)
7. [Configuration Reference](#configuration-reference)

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

```rust
// Located: edgequake/crates/edgequake-llm/src/traits.rs

#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Generate a completion from messages.
    async fn complete(
        &self, 
        messages: &[ChatMessage], 
        options: CompletionOptions
    ) -> Result<LLMResponse>;
    
    /// Generate a streaming completion.
    async fn complete_stream(
        &self, 
        messages: &[ChatMessage], 
        options: CompletionOptions
    ) -> Result<impl Stream<Item = Result<String>>>;
    
    /// Get the model name.
    fn model_name(&self) -> &str;
    
    /// Get the maximum context length.
    fn max_context_length(&self) -> usize;
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embeddings for texts.
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    
    /// Get embedding dimension.
    fn dimension(&self) -> usize;
    
    /// Get the model name.
    fn model_name(&self) -> &str;
}

pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

pub enum ChatRole {
    System,
    User,
    Assistant,
}

pub struct CompletionOptions {
    pub temperature: f32,
    pub max_tokens: Option<usize>,
    pub stop: Option<Vec<String>>,
}

pub struct LLMResponse {
    pub content: String,
    pub usage: TokenUsage,
    pub finish_reason: String,
}
```

### Supported Providers

| Provider | LLM | Embeddings | Production | Notes |
|----------|-----|------------|------------|-------|
| **OpenAI** | ✅ | ✅ | ✅ | Recommended |
| **Ollama** | ✅ | ✅ | ✅ | Local inference |
| **LM Studio** | ✅ | ✅ | ✅ | OpenAI-compatible |
| **Mock** | ✅ | ✅ | ❌ | Testing only |

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

| Model | Context | Cost (Input/Output per 1M) | Notes |
|-------|---------|---------------------------|-------|
| `gpt-4o-mini` | 128K | $0.15 / $0.60 | **Recommended** |
| `gpt-4o` | 128K | $2.50 / $10.00 | Highest quality |
| `gpt-4-turbo` | 128K | $10.00 / $30.00 | Legacy |
| `gpt-3.5-turbo` | 16K | $0.50 / $1.50 | Budget option |
| `o1-mini` | 128K | $3.00 / $12.00 | Reasoning model |

#### Embedding Models

| Model | Dimensions | Cost (per 1M tokens) | Notes |
|-------|------------|---------------------|-------|
| `text-embedding-3-small` | 1536 | $0.02 | **Recommended** |
| `text-embedding-3-large` | 3072 | $0.13 | Higher quality |
| `text-embedding-ada-002` | 1536 | $0.10 | Legacy |

### Cost Estimation

For `gpt-4o-mini` + `text-embedding-3-small`:

| Operation | Tokens | Cost |
|-----------|--------|------|
| Entity extraction (per chunk) | ~800 in + 200 out | $0.0012 |
| Embedding (per chunk) | ~1000 | $0.00002 |
| Query generation | ~2000 in + 500 out | $0.006 |

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

| Model | Size | Context | Notes |
|-------|------|---------|-------|
| `llama3.2:3b` | 2GB | 128K | Fast, good for extraction |
| `llama3.1:8b` | 4.7GB | 128K | Better quality |
| `mistral:7b` | 4GB | 32K | Good balance |
| `phi3:mini` | 2GB | 4K | Lightweight |
| `nomic-embed-text` | 274MB | - | Embeddings |
| `mxbai-embed-large` | 670MB | - | Higher quality embeddings |

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
- Configurable responses

### Usage

```rust
use edgequake_llm::MockProvider;

// Create mock provider
let mock = Arc::new(MockProvider::new());

// Or with custom configuration
let mock = Arc::new(
    MockProvider::new()
        .with_entity_extraction_mode()  // Returns realistic entity extractions
        .with_embedding_dimension(1536)
);
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

| Variable | Default | Description |
|----------|---------|-------------|
| `OPENAI_API_KEY` | - | OpenAI API key |
| `EDGEQUAKE_LLM_PROVIDER` | openai | Provider: openai, ollama |
| `EDGEQUAKE_LLM_MODEL` | gpt-4o-mini | LLM model name |
| `EDGEQUAKE_EMBEDDING_MODEL` | text-embedding-3-small | Embedding model |
| `EDGEQUAKE_EMBEDDING_DIM` | 1536 | Embedding dimension |
| `OPENAI_BASE_URL` | - | Custom API endpoint |
| `OLLAMA_HOST` | http://localhost:11434 | Ollama server URL |

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

### Error Handling

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
