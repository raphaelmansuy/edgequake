# LightRAG External Integrations

## Overview

This document defines the contracts for external service integrations including LLM providers, embedding models, and optional observability services.

---

## LLM Provider Interface

### Contract: LLM Model Function

```yaml
interface: llm_model_func
description: Async function for LLM text generation

signature: |
  async def llm_model_func(
    prompt: str | list[dict],
    system_prompt: str | None = None,
    history_messages: list[dict] | None = None,
    keyword_extraction: bool = False,
    stream: bool = False,
    **kwargs
  ) -> str | AsyncIterator[str]

parameters:
  prompt:
    type: str | list[dict]
    description: User prompt or conversation messages
    
  system_prompt:
    type: str | None
    description: System instruction for the model
    
  history_messages:
    type: list[dict] | None
    description: Previous conversation turns
    format: [{"role": "user"|"assistant", "content": "..."}]
    
  keyword_extraction:
    type: bool
    default: false
    description: Enable structured JSON output for keywords
    
  stream:
    type: bool
    default: false
    description: Enable streaming response
    
  kwargs:
    _priority: int  # Queue priority (higher = more important)
    hashing_kv: BaseKVStorage  # LLM response cache
    temperature: float
    max_tokens: int

returns:
  non_streaming: str  # Complete response text
  streaming: AsyncIterator[str]  # Chunks of response

retry_behavior:
  triggers:
    - APIConnectionError
    - RateLimitError
    - APITimeoutError
  strategy: exponential_backoff
  max_attempts: 5
  
caching:
  mechanism: |
    Hash prompt + model params to generate cache key.
    Check cache before making LLM call.
    Store response with cache_type and timestamp.
```

---

## Supported LLM Providers

### OpenAI / OpenAI-Compatible

```yaml
provider: openai
file: lightrag/llm/openai.py

environment_variables:
  OPENAI_API_KEY: API key (required)
  OPENAI_BASE_URL: Custom endpoint (optional)
  
functions:
  - openai_complete_if_cache: Chat completion with caching
  - openai_embedding: Embedding generation
  
configuration:
  model: gpt-4o-mini (default), gpt-4o, gpt-4-turbo
  max_tokens: configurable
  temperature: 0-2
  
features:
  - Streaming support
  - Function calling
  - JSON mode
  - Langfuse observability integration
```

### Azure OpenAI

```yaml
provider: azure_openai
file: lightrag/llm/azure_openai.py

environment_variables:
  AZURE_OPENAI_API_KEY: API key
  AZURE_OPENAI_ENDPOINT: Azure endpoint URL
  AZURE_OPENAI_DEPLOYMENT: Deployment name
  AZURE_OPENAI_API_VERSION: API version (default: 2024-02-15-preview)
  
configuration:
  Same as OpenAI but uses Azure endpoints
  
features:
  - All OpenAI features
  - Azure AD authentication
  - Regional deployment support
```

### Anthropic (Claude)

```yaml
provider: anthropic
file: lightrag/llm/anthropic.py

environment_variables:
  ANTHROPIC_API_KEY: API key
  
functions:
  - anthropic_complete: Claude chat completion
  
configuration:
  model: claude-3-opus, claude-3-sonnet, claude-3-haiku
  max_tokens: 4096 (default)
  
features:
  - Streaming support
  - System prompt support
  - Extended context windows
```

### Ollama (Local)

```yaml
provider: ollama
file: lightrag/llm/ollama.py

environment_variables:
  OLLAMA_HOST: Server URL (default: http://localhost:11434)
  
functions:
  - ollama_complete: Local LLM completion
  - ollama_embedding: Local embedding generation
  
configuration:
  model: Any Ollama-supported model
  keep_alive: Connection keep-alive duration
  
features:
  - Local inference
  - No API costs
  - Custom model support
  - GPU acceleration
```

### Google Gemini

```yaml
provider: gemini
file: lightrag/llm/gemini.py

environment_variables:
  GOOGLE_API_KEY: Gemini API key
  
functions:
  - gemini_complete: Gemini completion
  
configuration:
  model: gemini-pro, gemini-1.5-pro
  
features:
  - Multimodal support
  - Large context windows
```

### AWS Bedrock

```yaml
provider: bedrock
file: lightrag/llm/bedrock.py

environment_variables:
  AWS_ACCESS_KEY_ID: AWS access key
  AWS_SECRET_ACCESS_KEY: AWS secret key
  AWS_REGION: AWS region
  
supported_models:
  - anthropic.claude-3-sonnet
  - anthropic.claude-3-haiku
  - amazon.titan-text
  - meta.llama3
  
features:
  - AWS native authentication
  - VPC endpoints
  - CloudWatch integration
```

### HuggingFace

```yaml
provider: huggingface
file: lightrag/llm/hf.py

environment_variables:
  HF_TOKEN: HuggingFace token
  
functions:
  - hf_complete: Inference API completion
  - hf_embedding: Inference API embedding
  
features:
  - Access to open-source models
  - Custom model deployment
  - Serverless inference
```

---

## Embedding Provider Interface

### Contract: Embedding Function

```yaml
interface: EmbeddingFunc
description: Async function for computing text embeddings

signature: |
  @dataclass
  class EmbeddingFunc:
    embedding_dim: int
    max_token_size: int
    func: Callable[[list[str]], Awaitable[np.ndarray]]

func_signature: |
  async def embedding_func(texts: list[str]) -> np.ndarray

parameters:
  texts:
    type: list[str]
    description: List of texts to embed
    
returns:
  type: np.ndarray
  shape: (len(texts), embedding_dim)
  dtype: float32

properties:
  embedding_dim:
    description: Dimension of embedding vectors
    examples:
      text-embedding-3-small: 1536
      text-embedding-3-large: 3072
      
  max_token_size:
    description: Maximum tokens per text
    default: 8192

batching:
  - Texts should be batched internally
  - Handle rate limits within function
  - Return all embeddings in single array
```

### Supported Embedding Providers

```yaml
providers:
  openai:
    models:
      - text-embedding-3-small (1536d)
      - text-embedding-3-large (3072d)
      - text-embedding-ada-002 (1536d)
    environment:
      OPENAI_API_KEY: Required
      
  ollama:
    models:
      - nomic-embed-text
      - mxbai-embed-large
      - all-minilm
    environment:
      OLLAMA_HOST: Optional
      
  huggingface:
    models:
      - sentence-transformers/all-MiniLM-L6-v2
      - BAAI/bge-large-en
      - intfloat/e5-large-v2
    environment:
      HF_TOKEN: Optional
      
  jina:
    models:
      - jina-embeddings-v2-base-en
      - jina-embeddings-v2-small-en
    environment:
      JINA_API_KEY: Required
      
  bedrock:
    models:
      - amazon.titan-embed-text-v1
      - cohere.embed-english-v3
    environment:
      AWS credentials required
```

---

## Rerank Provider Interface

### Contract: Rerank Function

```yaml
interface: rerank_model_func
description: Optional function for reranking retrieved documents

signature: |
  async def rerank_model_func(
    query: str,
    documents: list[str],
    top_k: int
  ) -> list[dict]

parameters:
  query:
    type: str
    description: Query to rerank documents against
    
  documents:
    type: list[str]
    description: List of document texts to rerank
    
  top_k:
    type: int
    description: Number of top results to return

returns:
  type: list[dict]
  schema:
    - index: int  # Original index in documents list
      score: float  # Relevance score
      text: str  # Document text (optional)

providers:
  cohere:
    model: rerank-english-v2.0
    environment:
      COHERE_API_KEY: Required
      
  jina:
    model: jina-reranker-v1-base-en
    environment:
      JINA_API_KEY: Required
      
  local:
    models:
      - cross-encoder/ms-marco-MiniLM-L-6-v2
      - BAAI/bge-reranker-large
```

---

## Observability Integration

### Langfuse Integration

```yaml
provider: langfuse
description: LLM observability and tracing

environment_variables:
  LANGFUSE_PUBLIC_KEY: Public key
  LANGFUSE_SECRET_KEY: Secret key
  LANGFUSE_HOST: Optional custom host

activation: |
  Automatically enabled when both keys are set.
  Wraps OpenAI client for transparent tracing.

features:
  - Request/response logging
  - Latency tracking
  - Token usage monitoring
  - Cost estimation
  - Session grouping
```

---

## Rate Limiting & Retry

### Retry Configuration

```yaml
retry_strategy:
  library: tenacity
  
  default_config:
    stop: stop_after_attempt(5)
    wait: wait_exponential(multiplier=1, min=1, max=60)
    retry_on:
      - APIConnectionError
      - RateLimitError
      - APITimeoutError
      
  custom_exceptions:
    InvalidResponseError: Trigger retry on malformed output

priority_queue:
  description: |
    LLM calls are queued by priority.
    Higher priority tasks (entity summary) preempt lower priority.
    
  priority_levels:
    8: Entity/relation summarization
    5: Query generation
    1: Entity extraction (default)
```

### Concurrency Control

```yaml
concurrency:
  llm_model_max_async:
    default: 16
    description: Maximum concurrent LLM calls
    environment: MAX_ASYNC
    
  embedding_func_max_async:
    default: 8
    description: Maximum concurrent embedding calls
    environment: EMBEDDING_FUNC_MAX_ASYNC
    
  implementation: |
    Uses asyncio.Semaphore for limiting.
    Priority queue ensures important tasks proceed first.
```

---

## Timeout Configuration

```yaml
timeouts:
  llm:
    default: 120
    environment: LLM_TIMEOUT
    description: LLM call timeout in seconds
    
  embedding:
    default: 60
    environment: EMBEDDING_TIMEOUT
    description: Embedding call timeout in seconds
    
  implementation: |
    Wrapped with asyncio.timeout.
    Raises TimeoutError on expiry.
```

---

## Provider Configuration Matrix

| Provider | LLM | Embedding | Rerank | Streaming | Local |
|----------|-----|-----------|--------|-----------|-------|
| OpenAI | ✅ | ✅ | ❌ | ✅ | ❌ |
| Azure OpenAI | ✅ | ✅ | ❌ | ✅ | ❌ |
| Anthropic | ✅ | ❌ | ❌ | ✅ | ❌ |
| Ollama | ✅ | ✅ | ❌ | ✅ | ✅ |
| Gemini | ✅ | ❌ | ❌ | ✅ | ❌ |
| Bedrock | ✅ | ✅ | ❌ | ❌ | ❌ |
| HuggingFace | ✅ | ✅ | ✅ | ❌ | ✅ |
| Jina | ❌ | ✅ | ✅ | ❌ | ❌ |
| Cohere | ❌ | ✅ | ✅ | ❌ | ❌ |

---

## Integration Examples

### OpenAI Configuration

```python
from lightrag.llm.openai import openai_complete_if_cache, openai_embedding
from lightrag.utils import EmbeddingFunc

# LLM function
llm_func = partial(
    openai_complete_if_cache,
    model="gpt-4o-mini",
    api_key=os.getenv("OPENAI_API_KEY"),
)

# Embedding function
embedding_func = EmbeddingFunc(
    embedding_dim=1536,
    max_token_size=8192,
    func=partial(
        openai_embedding,
        model="text-embedding-3-small",
        api_key=os.getenv("OPENAI_API_KEY"),
    ),
)

# Initialize LightRAG
rag = LightRAG(
    llm_model_func=llm_func,
    embedding_func=embedding_func,
    llm_model_name="gpt-4o-mini",
)
```

### Ollama Configuration

```python
from lightrag.llm.ollama import ollama_complete, ollama_embedding

# LLM function
llm_func = partial(
    ollama_complete,
    model="llama3.1:8b",
    host="http://localhost:11434",
)

# Embedding function
embedding_func = EmbeddingFunc(
    embedding_dim=768,
    max_token_size=8192,
    func=partial(
        ollama_embedding,
        model="nomic-embed-text",
        host="http://localhost:11434",
    ),
)
```

---

## Cross-References

- [API Contracts](04-api-contracts.md) - How LLM functions are used
- [Algorithms](05-algorithms.md) - Entity extraction and summarization
- [Configuration](08-configuration.md) - Environment variables
