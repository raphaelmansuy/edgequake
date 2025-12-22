//! EdgeQuake LLM - LLM and Embedding Provider Abstraction
//!
//! This crate provides traits and implementations for:
//! - Text completion (LLM providers)
//! - Text embedding (embedding providers)
//! - Token counting and management
//!
//! # Architecture
//!
//! The crate uses trait-based abstraction to support multiple LLM backends:
//! - OpenAI (GPT-4, GPT-3.5)
//! - OpenAI-compatible APIs (Ollama, LM Studio, etc.)
//! - Future: Anthropic, Mistral, local models
//!
//! # Example
//!
//! ```ignore
//! use edgequake_llm::{LLMProvider, OpenAIProvider};
//!
//! let provider = OpenAIProvider::new("your-api-key");
//! let response = provider.complete("Hello, world!").await?;
//! ```

pub mod error;
pub mod providers;
pub mod tokenizer;
pub mod traits;

pub use error::{LlmError, Result};
pub use providers::openai::OpenAIProvider;
pub use tokenizer::Tokenizer;
pub use traits::{EmbeddingProvider, LLMProvider, LLMResponse};

pub use providers::mock::MockProvider;
