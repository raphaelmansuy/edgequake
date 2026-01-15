//! EdgeQuake LLM - LLM and Embedding Provider Abstraction
//!
//! # Implements
//!
//! - **FEAT0017**: Multi-Provider LLM Support
//! - **FEAT0018**: Embedding Provider Abstraction
//! - **FEAT0019**: LLM Response Caching
//! - **FEAT0020**: API Rate Limiting
//! - **FEAT0005**: Embedding Generation (via providers)
//!
//! # Enforces
//!
//! - **BR0301**: LLM API rate limits (configurable per provider)
//! - **BR0302**: Document size limits (context window awareness)
//! - **BR0303**: Cost tracking per request
//! - **BR0010**: Embedding dimension validated (1536 default)
//!
//! This crate provides traits and implementations for:
//! - Text completion (LLM providers)
//! - Text embedding (embedding providers)
//! - Token counting and management
//! - Rate limiting for API calls
//! - Response caching for cost reduction
//!
//! # Providers
//!
//! | Provider | FEAT0017 | Chat | Embeddings | Notes |
//! |----------|----------|------|------------|-------|
//! | OpenAI | ✓ | ✓ | ✓ | Primary production provider |
//! | Azure OpenAI | ✓ | ✓ | ✓ | Enterprise deployments |
//! | Ollama | ✓ | ✓ | ✓ | Local/on-prem models |
//! | LM Studio | ✓ | ✓ | ✓ | Local OpenAI-compatible API |
//! | Gemini | ✓ | ✓ | ✓ | Google AI |
//! | Mock | ✓ | ✓ | ✓ | Testing (no API calls) |
//!
//! # Architecture
//!
//! The crate uses trait-based abstraction to support multiple LLM backends:
//! - OpenAI (GPT-4, GPT-3.5)
//! - OpenAI-compatible APIs (Ollama, LM Studio, etc.)
//! - Anthropic (Claude 3.5, Claude 3)
//! - Future: Mistral, local models
//!
//! # Example
//!
//! ```ignore
//! use edgequake_llm::{LLMProvider, OpenAIProvider};
//!
//! let provider = OpenAIProvider::new("your-api-key");
//! let response = provider.complete("Hello, world!").await?;
//! ```
//!
//! # See Also
//!
//! - [`crate::traits`] for provider trait definitions
//! - [`crate::providers`] for concrete implementations
//! - [`crate::cache`] for response caching

pub mod cache;
pub mod error;
pub mod factory;
pub mod model_config;
pub mod providers;
pub mod rate_limiter;
pub mod reranker;
pub mod safety_limits;
pub mod tokenizer;
pub mod traits;

pub use cache::{CacheConfig, CacheStats, CachedProvider, LLMCache};
pub use error::{LlmError, Result};
pub use factory::{ProviderFactory, ProviderType};
pub use model_config::{
    DefaultsConfig, ModelCapabilities, ModelCard, ModelConfigError, ModelCost, ModelType,
    ModelsConfig, ProviderConfig, ProviderType as ConfigProviderType,
};
pub use providers::azure_openai::AzureOpenAIProvider;
pub use providers::gemini::GeminiProvider;
pub use providers::jina::JinaProvider;
pub use providers::lmstudio::LMStudioProvider;
pub use providers::mock::MockProvider;
pub use providers::ollama::OllamaProvider;
pub use providers::openai::OpenAIProvider;
pub use rate_limiter::{RateLimitedProvider, RateLimiter, RateLimiterConfig};
pub use reranker::{
    BM25Reranker, HttpReranker, HybridReranker, MockReranker, RRFReranker, RerankConfig,
    RerankResult, Reranker, ScoreAggregation, TermOverlapReranker,
};
pub use safety_limits::{
    SafetyLimitedEmbeddingProvider, SafetyLimitedEmbeddingProviderWrapper, SafetyLimitedProvider,
    SafetyLimitedProviderWrapper, SafetyLimitsConfig, ABSOLUTE_MAX_TOKENS, DEFAULT_MAX_TOKENS,
    DEFAULT_TIMEOUT_SECS,
};
pub use tokenizer::Tokenizer;
pub use traits::{
    ChatMessage, ChatRole, CompletionOptions, EmbeddingProvider, LLMProvider, LLMResponse,
    StreamOrComplete,
};
