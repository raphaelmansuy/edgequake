//! LLM provider factory for environment-based selection.
//!
//! @implements SPEC-032: Ollama/LM Studio provider support
//! @implements FEAT0017: Multi-provider LLM support
//!
//! # Environment Variables
//!
//! ## Provider Selection
//!
//! - `EDGEQUAKE_LLM_PROVIDER`: Override provider selection (openai|ollama|lmstudio|mock)
//!
//! ## Provider-Specific Configuration
//!
//! See individual provider documentation for configuration variables:
//! - OpenAI: `OPENAI_API_KEY`, `OPENAI_BASE_URL`
//! - Ollama: `OLLAMA_HOST`, `OLLAMA_MODEL`, `OLLAMA_EMBEDDING_MODEL`
//! - LM Studio: `LMSTUDIO_HOST`, `LMSTUDIO_MODEL`, `LMSTUDIO_EMBEDDING_MODEL`
//!
//! # Auto-Detection Priority
//!
//! When `EDGEQUAKE_LLM_PROVIDER` is not set:
//! 1. Check for OLLAMA_HOST or OLLAMA_MODEL → Use Ollama
//! 2. Check for OPENAI_API_KEY → Use OpenAI
//! 3. Fallback → Use Mock provider
//!
//! # Example
//!
//! ```rust,ignore
//! use edgequake_llm::ProviderFactory;
//!
//! // Auto-detect from environment
//! let (llm, embedding) = ProviderFactory::from_env()?;
//!
//! // Explicit provider selection
//! std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "ollama");
//! let (llm, embedding) = ProviderFactory::from_env()?;
//! ```

use std::sync::Arc;

use crate::error::{LlmError, Result};
use crate::traits::{EmbeddingProvider, LLMProvider};
use crate::{MockProvider, OllamaProvider, OpenAIProvider};

/// Supported provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    /// OpenAI provider (cloud API)
    OpenAI,
    /// Ollama provider (local models)
    Ollama,
    /// LM Studio provider (OpenAI-compatible local API)
    LMStudio,
    /// Mock provider (testing only)
    Mock,
}

impl ProviderType {
    /// Parse provider type from string (case-insensitive)
    ///
    /// # Examples
    ///
    /// ```
    /// use edgequake_llm::ProviderType;
    ///
    /// assert_eq!(ProviderType::from_str("openai"), Some(ProviderType::OpenAI));
    /// assert_eq!(ProviderType::from_str("OLLAMA"), Some(ProviderType::Ollama));
    /// assert_eq!(ProviderType::from_str("lm-studio"), Some(ProviderType::LMStudio));
    /// ```
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(Self::OpenAI),
            "ollama" => Some(Self::Ollama),
            "lmstudio" | "lm-studio" | "lm_studio" => Some(Self::LMStudio),
            "mock" => Some(Self::Mock),
            _ => None,
        }
    }
}

/// Provider factory for creating LLM and embedding providers.
///
/// Provides environment-based auto-detection and explicit provider selection.
pub struct ProviderFactory;

impl ProviderFactory {
    /// Auto-detect and create providers from environment.
    ///
    /// # Priority
    ///
    /// 1. `EDGEQUAKE_LLM_PROVIDER` environment variable (explicit selection)
    /// 2. Auto-detect: OLLAMA_HOST → OPENAI_API_KEY → Mock
    ///
    /// # Returns
    ///
    /// Returns a tuple of (LLMProvider, EmbeddingProvider). In most cases,
    /// the same provider implementation is used for both.
    ///
    /// # Errors
    ///
    /// Returns error if required configuration for selected provider is missing.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
    /// let (llm, embedding) = ProviderFactory::from_env()?;
    /// assert_eq!(llm.name(), "ollama");
    /// ```
    pub fn from_env() -> Result<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>)> {
        // Check explicit provider selection
        if let Ok(provider_str) = std::env::var("EDGEQUAKE_LLM_PROVIDER") {
            if let Some(provider_type) = ProviderType::from_str(&provider_str) {
                return Self::create(provider_type);
            }
            return Err(LlmError::ConfigError(format!(
                "Unknown provider type: {}. Valid options: openai, ollama, lmstudio, mock",
                provider_str
            )));
        }

        // Auto-detect based on environment
        if std::env::var("OLLAMA_HOST").is_ok() || std::env::var("OLLAMA_MODEL").is_ok() {
            return Self::create(ProviderType::Ollama);
        }

        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            if !api_key.is_empty() && api_key != "test-key" {
                return Self::create(ProviderType::OpenAI);
            }
        }

        // Fallback to mock
        Ok(Self::create_mock())
    }

    /// Create specific provider type.
    ///
    /// # Arguments
    ///
    /// * `provider_type` - The type of provider to create
    ///
    /// # Returns
    ///
    /// Returns a tuple of (LLMProvider, EmbeddingProvider).
    ///
    /// # Errors
    ///
    /// Returns error if required configuration for the provider is missing.
    pub fn create(
        provider_type: ProviderType,
    ) -> Result<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>)> {
        match provider_type {
            ProviderType::OpenAI => Self::create_openai(),
            ProviderType::Ollama => Self::create_ollama(),
            ProviderType::LMStudio => Self::create_lmstudio(),
            ProviderType::Mock => Ok(Self::create_mock()),
        }
    }

    /// Create OpenAI provider from environment.
    ///
    /// Reads `OPENAI_API_KEY` environment variable.
    fn create_openai() -> Result<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>)> {
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
            LlmError::ConfigError("OPENAI_API_KEY not set for OpenAI provider".to_string())
        })?;

        if api_key.is_empty() || api_key == "test-key" {
            return Err(LlmError::ConfigError(
                "OPENAI_API_KEY is empty or invalid".to_string(),
            ));
        }

        let provider = Arc::new(OpenAIProvider::new(api_key));
        Ok((provider.clone(), provider))
    }

    /// Create Ollama provider from environment.
    ///
    /// Uses OllamaProvider::from_env() which reads:
    /// - OLLAMA_HOST
    /// - OLLAMA_MODEL
    /// - OLLAMA_EMBEDDING_MODEL
    fn create_ollama() -> Result<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>)> {
        let provider = Arc::new(OllamaProvider::from_env()?);
        Ok((provider.clone(), provider))
    }

    /// Create LM Studio provider (OpenAI-compatible).
    ///
    /// LM Studio provides an OpenAI-compatible API, so we use OpenAIProvider
    /// with custom configuration.
    ///
    /// Environment variables:
    /// - `LMSTUDIO_HOST`: LM Studio server URL (default: http://localhost:1234)
    /// - `LMSTUDIO_MODEL`: Chat model name (default: gemma2-9b-it)
    /// - `LMSTUDIO_EMBEDDING_MODEL`: Embedding model (default: text-embedding-ada-002)
    /// - `LMSTUDIO_EMBEDDING_DIM`: Embedding dimension (default: 1536)
    fn create_lmstudio() -> Result<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>)> {
        let host = std::env::var("LMSTUDIO_HOST")
            .unwrap_or_else(|_| "http://localhost:1234".to_string());

        let model =
            std::env::var("LMSTUDIO_MODEL").unwrap_or_else(|_| "gemma2-9b-it".to_string());

        let embedding_model = std::env::var("LMSTUDIO_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-ada-002".to_string());

        // OpenAI-compatible endpoint requires /v1 suffix
        let base_url = if host.ends_with("/v1") {
            host
        } else {
            format!("{}/v1", host)
        };

        let provider = Arc::new(
            OpenAIProvider::compatible("lmstudio-key", base_url)
                .with_model(model)
                .with_embedding_model(embedding_model),
        );

        Ok((provider.clone(), provider))
    }

    /// Create mock provider for testing.
    ///
    /// Always returns deterministic responses.
    fn create_mock() -> (Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>) {
        let provider = Arc::new(MockProvider::new());
        (provider.clone(), provider)
    }

    /// Get embedding dimension for current provider configuration.
    ///
    /// Useful for configuring vector storage with the correct dimension.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "ollama");
    /// let dim = ProviderFactory::embedding_dimension()?;
    /// assert_eq!(dim, 768); // embeddinggemma dimension
    /// ```
    pub fn embedding_dimension() -> Result<usize> {
        let (_, embedding_provider) = Self::from_env()?;
        Ok(embedding_provider.dimension())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type_parsing() {
        assert_eq!(
            ProviderType::from_str("openai"),
            Some(ProviderType::OpenAI)
        );
        assert_eq!(
            ProviderType::from_str("OLLAMA"),
            Some(ProviderType::Ollama)
        );
        assert_eq!(
            ProviderType::from_str("lmstudio"),
            Some(ProviderType::LMStudio)
        );
        assert_eq!(
            ProviderType::from_str("lm-studio"),
            Some(ProviderType::LMStudio)
        );
        assert_eq!(
            ProviderType::from_str("lm_studio"),
            Some(ProviderType::LMStudio)
        );
        assert_eq!(ProviderType::from_str("mock"), Some(ProviderType::Mock));
        assert_eq!(ProviderType::from_str("invalid"), None);
        assert_eq!(ProviderType::from_str(""), None);
    }

    #[test]
    fn test_mock_creation() {
        let (llm, embedding) = ProviderFactory::create_mock();
        assert_eq!(llm.name(), "mock");
        assert_eq!(embedding.name(), "mock");
        assert_eq!(embedding.dimension(), 1536);
    }

    #[test]
    fn test_explicit_mock_creation() {
        let (llm, embedding) = ProviderFactory::create(ProviderType::Mock).unwrap();
        assert_eq!(llm.name(), "mock");
        assert_eq!(embedding.dimension(), 1536);
    }

    #[test]
    fn test_from_env_fallback_to_mock() {
        // Clear all provider environment variables
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("OLLAMA_MODEL");

        let (llm, _) = ProviderFactory::from_env().unwrap();
        assert_eq!(llm.name(), "mock");
    }

    #[test]
    fn test_explicit_provider_env() {
        // Clean up first to avoid interference from other tests
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("OPENAI_API_KEY");
        
        std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
        let (llm, _) = ProviderFactory::from_env().unwrap();
        assert_eq!(llm.name(), "mock");
        
        // Clean up after
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    }

    #[test]
    fn test_invalid_provider_env() {
        // Clean up first to avoid interference from other tests
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("OPENAI_API_KEY");
        
        std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "invalid_provider");
        let result = ProviderFactory::from_env();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Unknown provider type"));
        }
        
        // Clean up after
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    }

    #[test]
    fn test_openai_creation_requires_api_key() {
        // Clean up first to avoid interference from other tests
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("OLLAMA_HOST");
        
        let result = ProviderFactory::create(ProviderType::OpenAI);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("OPENAI_API_KEY not set"));
        }
    }

    #[test]
    fn test_embedding_dimension_detection() {
        // Clean up first to avoid interference from other tests
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("OPENAI_API_KEY");
        
        std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
        let dim = ProviderFactory::embedding_dimension().unwrap();
        assert_eq!(dim, 1536);
        
        // Clean up after
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    }
}
