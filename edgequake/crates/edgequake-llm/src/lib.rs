//! EdgeQuake LLM - LLM and Embedding Provider Abstraction
//!
//! This crate provides traits and implementations for:
//! - Text completion (LLM providers)
//! - Text embedding (embedding providers)
//! - Token counting and management
//! - Rate limiting for API calls
//! - Response caching for cost reduction
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

pub mod cache;
pub mod error;
pub mod providers;
pub mod rate_limiter;
pub mod reranker;
pub mod tokenizer;
pub mod traits;

pub use cache::{CacheConfig, CacheStats, CachedProvider, LLMCache};
pub use error::{LlmError, Result};
pub use providers::openai::OpenAIProvider;
pub use rate_limiter::{RateLimitedProvider, RateLimiter, RateLimiterConfig};
pub use reranker::{HttpReranker, MockReranker, RerankConfig, RerankResult, Reranker, ScoreAggregation};
pub use tokenizer::Tokenizer;
pub use traits::{EmbeddingProvider, LLMProvider, LLMResponse};

pub use providers::mock::MockProvider;
