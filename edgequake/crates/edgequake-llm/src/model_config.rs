//! Model Configuration Module
//!
//! This module provides TOML-based configuration for LLM and embedding models,
//! including model cards with capabilities (vision, context length, costs, etc.).
//!
//! @implements SPEC-032: Ollama/LM Studio provider support - Model cards configuration
//! @iteration OODA Loop #51-55 - TOML Config Schema Design
//!
//! # Overview
//!
//! The configuration file (`models.toml`) defines:
//! - Available LLM providers and models
//! - Embedding providers and models  
//! - Model capabilities (vision, max tokens, context length)
//! - Cost information (per 1K tokens)
//! - Default selections for LLM and embedding
//!
//! # Configuration File Location
//!
//! The config file is loaded from (in order of priority):
//! 1. `EDGEQUAKE_MODELS_CONFIG` environment variable
//! 2. `./models.toml` (current working directory)
//! 3. `~/.edgequake/models.toml` (user config)
//! 4. Built-in default configuration
//!
//! # Example Configuration
//!
//! ## Pricing Note (2026-02)
//!
//! OpenAI's cheapest models for document ingestion:
//! - LLM: `gpt-4.1-nano` - $0.05/1M input tokens (3x cheaper than gpt-4o-mini)
//! - Embedding: `text-embedding-3-small` - $0.02/1M tokens (5x cheaper than ada-002)
//!
//! ```toml
//! [defaults]
//! llm_provider = "openai"
//! llm_model = "gpt-4.1-nano"  # Cheapest: $0.05/1M input, $0.40/1M output
//! embedding_provider = "openai"
//! embedding_model = "text-embedding-3-small"  # Cheapest: $0.02/1M tokens
//!
//! [[providers]]
//! name = "openai"
//! display_name = "OpenAI"
//! type = "openai"
//! api_key_env = "OPENAI_API_KEY"
//!
//! [[providers.models]]
//! name = "gpt-4o"
//! display_name = "GPT-4 Omni"
//! type = "llm"
//! context_length = 128000
//! max_output_tokens = 16384
//! supports_vision = true
//! supports_function_calling = true
//! cost_per_1k_input = 0.0025
//! cost_per_1k_output = 0.01
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during model configuration loading.
#[derive(Error, Debug)]
pub enum ModelConfigError {
    /// Failed to read configuration file.
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),

    /// Failed to parse TOML configuration.
    #[error("Failed to parse TOML config: {0}")]
    ParseError(String),

    /// Invalid configuration (missing required fields, invalid values).
    #[error("Invalid configuration: {0}")]
    ValidationError(String),

    /// Provider not found in configuration.
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    /// Model not found in configuration.
    #[error("Model not found: {0}")]
    ModelNotFound(String),
}

// ============================================================================
// Model Types
// ============================================================================

/// Type of model (LLM for chat/completion, Embedding for vectors).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModelType {
    /// Language model for chat/completion.
    #[default]
    Llm,
    /// Embedding model for vector generation.
    Embedding,
    /// Multi-modal model supporting both.
    Multimodal,
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelType::Llm => write!(f, "llm"),
            ModelType::Embedding => write!(f, "embedding"),
            ModelType::Multimodal => write!(f, "multimodal"),
        }
    }
}

/// Provider type for API compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    /// OpenAI API.
    #[default]
    OpenAI,
    /// Ollama local server.
    Ollama,
    /// LM Studio local server.
    LMStudio,
    /// Azure OpenAI.
    Azure,
    /// Anthropic Claude.
    Anthropic,
    /// Generic OpenAI-compatible API.
    OpenAICompatible,
    /// Mock provider for testing.
    Mock,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Ollama => write!(f, "ollama"),
            ProviderType::LMStudio => write!(f, "lmstudio"),
            ProviderType::Azure => write!(f, "azure"),
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::OpenAICompatible => write!(f, "openai_compatible"),
            ProviderType::Mock => write!(f, "mock"),
        }
    }
}

// ============================================================================
// Model Capabilities
// ============================================================================

/// Capabilities of a specific model.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelCapabilities {
    /// Maximum context length (input + output tokens).
    #[serde(default)]
    pub context_length: usize,

    /// Maximum output tokens the model can generate.
    #[serde(default)]
    pub max_output_tokens: usize,

    /// Whether the model supports vision/image input.
    #[serde(default)]
    pub supports_vision: bool,

    /// Whether the model supports function/tool calling.
    #[serde(default)]
    pub supports_function_calling: bool,

    /// Whether the model supports structured JSON output.
    #[serde(default)]
    pub supports_json_mode: bool,

    /// Whether the model supports streaming responses.
    #[serde(default = "default_true")]
    pub supports_streaming: bool,

    /// Whether the model supports system messages.
    #[serde(default = "default_true")]
    pub supports_system_message: bool,

    /// Embedding dimension (only for embedding models).
    #[serde(default)]
    pub embedding_dimension: usize,

    /// Maximum tokens for embedding input.
    #[serde(default)]
    pub max_embedding_tokens: usize,
}

fn default_true() -> bool {
    true
}

// ============================================================================
// Cost Information
// ============================================================================

/// Cost information for a model (per 1000 tokens).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelCost {
    /// Cost per 1000 input tokens (USD).
    #[serde(default)]
    pub input_per_1k: f64,

    /// Cost per 1000 output tokens (USD).
    #[serde(default)]
    pub output_per_1k: f64,

    /// Cost per 1000 embedding tokens (USD, for embedding models).
    #[serde(default)]
    pub embedding_per_1k: f64,

    /// Cost per image processed (USD, for vision models).
    #[serde(default)]
    pub image_per_unit: f64,

    /// Currency code (default: USD).
    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_currency() -> String {
    "USD".to_string()
}

// ============================================================================
// Model Card
// ============================================================================

/// Complete model card with all metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCard {
    /// Unique model identifier (e.g., "gpt-4o", "nomic-embed-text").
    pub name: String,

    /// Human-readable display name.
    pub display_name: String,

    /// Model type (LLM, Embedding, Multimodal).
    #[serde(default)]
    pub model_type: ModelType,

    /// Model capabilities.
    #[serde(default)]
    pub capabilities: ModelCapabilities,

    /// Cost information.
    #[serde(default)]
    pub cost: ModelCost,

    /// Optional description of the model.
    #[serde(default)]
    pub description: String,

    /// Release date or version.
    #[serde(default)]
    pub version: String,

    /// Whether the model is deprecated.
    #[serde(default)]
    pub deprecated: bool,

    /// Recommended replacement if deprecated.
    #[serde(default)]
    pub replacement: Option<String>,

    /// Tags for categorization (e.g., "recommended", "fast", "vision").
    #[serde(default)]
    pub tags: Vec<String>,

    /// Additional metadata as key-value pairs.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl Default for ModelCard {
    fn default() -> Self {
        Self {
            name: "unknown".to_string(),
            display_name: "Unknown Model".to_string(),
            model_type: ModelType::Llm,
            capabilities: ModelCapabilities::default(),
            cost: ModelCost::default(),
            description: String::new(),
            version: String::new(),
            deprecated: false,
            replacement: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

// ============================================================================
// Provider Configuration
// ============================================================================

/// Configuration for a provider (OpenAI, Ollama, LM Studio, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Unique provider identifier (e.g., "openai", "ollama").
    pub name: String,

    /// Human-readable display name.
    pub display_name: String,

    /// Provider type for API compatibility.
    #[serde(rename = "type")]
    pub provider_type: ProviderType,

    /// Environment variable name for API key (if required).
    #[serde(default)]
    pub api_key_env: Option<String>,

    /// Base URL for the provider API.
    #[serde(default)]
    pub base_url: Option<String>,

    /// Environment variable for base URL override.
    #[serde(default)]
    pub base_url_env: Option<String>,

    /// Default model for LLM operations.
    #[serde(default)]
    pub default_llm_model: Option<String>,

    /// Default model for embedding operations.
    #[serde(default)]
    pub default_embedding_model: Option<String>,

    /// List of available models for this provider.
    #[serde(default)]
    pub models: Vec<ModelCard>,

    /// Whether this provider is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Priority for auto-selection (lower = higher priority).
    #[serde(default = "default_priority")]
    pub priority: u32,

    /// Description of the provider.
    #[serde(default)]
    pub description: String,

    /// Additional provider-specific settings.
    #[serde(default)]
    pub settings: HashMap<String, String>,
}

fn default_priority() -> u32 {
    100
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            name: "unknown".to_string(),
            display_name: "Unknown Provider".to_string(),
            provider_type: ProviderType::OpenAI,
            api_key_env: None,
            base_url: None,
            base_url_env: None,
            default_llm_model: None,
            default_embedding_model: None,
            models: Vec::new(),
            enabled: true,
            priority: 100,
            description: String::new(),
            settings: HashMap::new(),
        }
    }
}

// ============================================================================
// Default Configuration
// ============================================================================

/// Default provider and model selections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    /// Default LLM provider name.
    #[serde(default = "default_llm_provider")]
    pub llm_provider: String,

    /// Default LLM model name.
    #[serde(default = "default_llm_model")]
    pub llm_model: String,

    /// Default embedding provider name.
    #[serde(default = "default_embedding_provider")]
    pub embedding_provider: String,

    /// Default embedding model name.
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
}

fn default_llm_provider() -> String {
    "openai".to_string()
}

fn default_llm_model() -> String {
    // WHY: gpt-4.1-nano is the recommended default (2025-02).
    // gpt-4o-mini has quota issues and is being phased out.
    // See: OODA-06 in specs/001-reliable-ingestion-mission/
    "gpt-4.1-nano".to_string()
}

fn default_embedding_provider() -> String {
    "openai".to_string()
}

fn default_embedding_model() -> String {
    "text-embedding-3-small".to_string()
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            llm_provider: default_llm_provider(),
            llm_model: default_llm_model(),
            embedding_provider: default_embedding_provider(),
            embedding_model: default_embedding_model(),
        }
    }
}

// ============================================================================
// Root Configuration
// ============================================================================

/// Root configuration structure for models.toml.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelsConfig {
    /// Default selections.
    #[serde(default)]
    pub defaults: DefaultsConfig,

    /// List of configured providers.
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
}

impl ModelsConfig {
    /// Load configuration from the default location.
    ///
    /// Searches in order:
    /// 1. `EDGEQUAKE_MODELS_CONFIG` environment variable
    /// 2. `./models.toml`
    /// 3. `~/.edgequake/models.toml`
    /// 4. Built-in defaults
    pub fn load() -> Result<Self, ModelConfigError> {
        // Check environment variable first
        if let Ok(path) = std::env::var("EDGEQUAKE_MODELS_CONFIG") {
            if Path::new(&path).exists() {
                return Self::from_file(&path);
            }
        }

        // Check current directory
        let local_path = Path::new("models.toml");
        if local_path.exists() {
            return Self::from_file(local_path);
        }

        // Check user config directory
        if let Some(home) = dirs::home_dir() {
            let user_path = home.join(".edgequake").join("models.toml");
            if user_path.exists() {
                return Self::from_file(&user_path);
            }
        }

        // Fall back to built-in defaults
        Ok(Self::builtin_defaults())
    }

    /// Load configuration from a specific file path.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ModelConfigError> {
        let content = std::fs::read_to_string(path.as_ref())?;
        Self::from_toml(&content)
    }

    /// Parse configuration from TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, ModelConfigError> {
        toml::from_str(toml_str).map_err(|e| ModelConfigError::ParseError(e.to_string()))
    }

    /// Serialize configuration to TOML string.
    pub fn to_toml(&self) -> Result<String, ModelConfigError> {
        toml::to_string_pretty(self).map_err(|e| ModelConfigError::ParseError(e.to_string()))
    }

    /// Save configuration to a file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ModelConfigError> {
        let toml_str = self.to_toml()?;
        std::fs::write(path.as_ref(), toml_str)?;
        Ok(())
    }

    /// Get built-in default configuration with common providers.
    pub fn builtin_defaults() -> Self {
        Self {
            defaults: DefaultsConfig::default(),
            providers: vec![
                // OpenAI provider
                ProviderConfig {
                    name: "openai".to_string(),
                    display_name: "OpenAI".to_string(),
                    provider_type: ProviderType::OpenAI,
                    api_key_env: Some("OPENAI_API_KEY".to_string()),
                    base_url: Some("https://api.openai.com/v1".to_string()),
                    base_url_env: Some("OPENAI_API_BASE".to_string()),
                    // WHY: gpt-4.1-nano is the recommended default. gpt-4o-mini deprecated.
                    default_llm_model: Some("gpt-4.1-nano".to_string()),
                    default_embedding_model: Some("text-embedding-3-small".to_string()),
                    priority: 10,
                    models: vec![
                        ModelCard {
                            name: "gpt-4o".to_string(),
                            display_name: "GPT-4 Omni".to_string(),
                            model_type: ModelType::Llm,
                            capabilities: ModelCapabilities {
                                context_length: 128000,
                                max_output_tokens: 16384,
                                supports_vision: true,
                                supports_function_calling: true,
                                supports_json_mode: true,
                                supports_streaming: true,
                                ..Default::default()
                            },
                            cost: ModelCost {
                                input_per_1k: 0.0025,
                                output_per_1k: 0.01,
                                ..Default::default()
                            },
                            description: "Most capable GPT-4 model with vision support".to_string(),
                            ..Default::default()
                        },
                        ModelCard {
                            name: "gpt-4o-mini".to_string(),
                            display_name: "GPT-4 Omni Mini".to_string(),
                            model_type: ModelType::Llm,
                            capabilities: ModelCapabilities {
                                context_length: 128000,
                                max_output_tokens: 16384,
                                supports_vision: true,
                                supports_function_calling: true,
                                supports_json_mode: true,
                                supports_streaming: true,
                                ..Default::default()
                            },
                            cost: ModelCost {
                                input_per_1k: 0.00015,
                                output_per_1k: 0.0006,
                                ..Default::default()
                            },
                            description: "Cost-effective GPT-4 variant".to_string(),
                            ..Default::default()
                        },
                        // WHY: gpt-4.1-nano is the recommended default (2025-02).
                        // It replaces gpt-4o-mini which has quota issues.
                        ModelCard {
                            name: "gpt-4.1-nano".to_string(),
                            display_name: "GPT-5 Nano".to_string(),
                            model_type: ModelType::Llm,
                            capabilities: ModelCapabilities {
                                context_length: 128000,
                                max_output_tokens: 16384,
                                supports_vision: true,
                                supports_function_calling: true,
                                supports_json_mode: true,
                                supports_streaming: true,
                                ..Default::default()
                            },
                            cost: ModelCost {
                                input_per_1k: 0.00015, // Similar to gpt-4o-mini
                                output_per_1k: 0.0006,
                                ..Default::default()
                            },
                            description: "Recommended cost-effective model for entity extraction"
                                .to_string(),
                            ..Default::default()
                        },
                        ModelCard {
                            name: "text-embedding-3-small".to_string(),
                            display_name: "Embedding 3 Small".to_string(),
                            model_type: ModelType::Embedding,
                            capabilities: ModelCapabilities {
                                embedding_dimension: 1536,
                                max_embedding_tokens: 8191,
                                ..Default::default()
                            },
                            cost: ModelCost {
                                embedding_per_1k: 0.00002,
                                ..Default::default()
                            },
                            description: "Efficient embedding model".to_string(),
                            ..Default::default()
                        },
                        ModelCard {
                            name: "text-embedding-3-large".to_string(),
                            display_name: "Embedding 3 Large".to_string(),
                            model_type: ModelType::Embedding,
                            capabilities: ModelCapabilities {
                                embedding_dimension: 3072,
                                max_embedding_tokens: 8191,
                                ..Default::default()
                            },
                            cost: ModelCost {
                                embedding_per_1k: 0.00013,
                                ..Default::default()
                            },
                            description: "High-quality embedding model".to_string(),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
                // Ollama provider
                ProviderConfig {
                    name: "ollama".to_string(),
                    display_name: "Ollama (Local)".to_string(),
                    provider_type: ProviderType::Ollama,
                    base_url: Some("http://localhost:11434".to_string()),
                    base_url_env: Some("OLLAMA_HOST".to_string()),
                    default_llm_model: Some("gemma3:12b".to_string()),
                    default_embedding_model: Some("nomic-embed-text".to_string()),
                    priority: 20,
                    models: vec![
                        ModelCard {
                            name: "gemma3:12b".to_string(),
                            display_name: "Gemma 3 12B".to_string(),
                            model_type: ModelType::Llm,
                            capabilities: ModelCapabilities {
                                context_length: 8192,
                                max_output_tokens: 4096,
                                supports_streaming: true,
                                ..Default::default()
                            },
                            cost: ModelCost::default(), // Free for local
                            description: "Google's Gemma 3 12B parameter model".to_string(),
                            ..Default::default()
                        },
                        ModelCard {
                            name: "llama3.3:70b".to_string(),
                            display_name: "Llama 3.3 70B".to_string(),
                            model_type: ModelType::Llm,
                            capabilities: ModelCapabilities {
                                context_length: 131072,
                                max_output_tokens: 8192,
                                supports_function_calling: true,
                                supports_streaming: true,
                                ..Default::default()
                            },
                            cost: ModelCost::default(),
                            description: "Meta's Llama 3.3 70B with extended context".to_string(),
                            ..Default::default()
                        },
                        ModelCard {
                            name: "nomic-embed-text".to_string(),
                            display_name: "Nomic Embed Text".to_string(),
                            model_type: ModelType::Embedding,
                            capabilities: ModelCapabilities {
                                embedding_dimension: 768,
                                max_embedding_tokens: 8192,
                                ..Default::default()
                            },
                            cost: ModelCost::default(),
                            description: "High-quality local embedding model".to_string(),
                            ..Default::default()
                        },
                        ModelCard {
                            name: "mxbai-embed-large".to_string(),
                            display_name: "MxBai Embed Large".to_string(),
                            model_type: ModelType::Embedding,
                            capabilities: ModelCapabilities {
                                embedding_dimension: 1024,
                                max_embedding_tokens: 512,
                                ..Default::default()
                            },
                            cost: ModelCost::default(),
                            description: "Large embedding model with 1024 dimensions".to_string(),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
                // LM Studio provider
                ProviderConfig {
                    name: "lmstudio".to_string(),
                    display_name: "LM Studio (Local)".to_string(),
                    provider_type: ProviderType::LMStudio,
                    base_url: Some("http://localhost:1234/v1".to_string()),
                    base_url_env: Some("LMSTUDIO_HOST".to_string()),
                    default_llm_model: Some("local-model".to_string()),
                    default_embedding_model: Some("nomic-embed-text-v1.5".to_string()),
                    priority: 30,
                    models: vec![
                        ModelCard {
                            name: "local-model".to_string(),
                            display_name: "Local LM Studio Model".to_string(),
                            model_type: ModelType::Llm,
                            capabilities: ModelCapabilities {
                                context_length: 4096,
                                max_output_tokens: 2048,
                                supports_streaming: true,
                                ..Default::default()
                            },
                            cost: ModelCost::default(),
                            description: "Currently loaded model in LM Studio".to_string(),
                            ..Default::default()
                        },
                        ModelCard {
                            name: "nomic-embed-text-v1.5".to_string(),
                            display_name: "Nomic Embed Text v1.5".to_string(),
                            model_type: ModelType::Embedding,
                            capabilities: ModelCapabilities {
                                embedding_dimension: 768,
                                max_embedding_tokens: 8192,
                                ..Default::default()
                            },
                            cost: ModelCost::default(),
                            description: "Nomic embedding model for LM Studio".to_string(),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
                // Mock provider for testing
                ProviderConfig {
                    name: "mock".to_string(),
                    display_name: "Mock (Testing)".to_string(),
                    provider_type: ProviderType::Mock,
                    default_llm_model: Some("mock-model".to_string()),
                    default_embedding_model: Some("mock-embedding".to_string()),
                    priority: 1000,
                    models: vec![
                        ModelCard {
                            name: "mock-model".to_string(),
                            display_name: "Mock LLM".to_string(),
                            model_type: ModelType::Llm,
                            capabilities: ModelCapabilities {
                                context_length: 4096,
                                max_output_tokens: 2048,
                                supports_streaming: true,
                                ..Default::default()
                            },
                            cost: ModelCost::default(),
                            description: "Mock model for testing".to_string(),
                            ..Default::default()
                        },
                        ModelCard {
                            name: "mock-embedding".to_string(),
                            display_name: "Mock Embedding".to_string(),
                            model_type: ModelType::Embedding,
                            capabilities: ModelCapabilities {
                                embedding_dimension: 1536,
                                max_embedding_tokens: 512,
                                ..Default::default()
                            },
                            cost: ModelCost::default(),
                            description: "Mock embedding for testing".to_string(),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
            ],
        }
    }

    /// Get a provider by name.
    pub fn get_provider(&self, name: &str) -> Option<&ProviderConfig> {
        self.providers.iter().find(|p| p.name == name)
    }

    /// Get a model by provider and model name.
    pub fn get_model(&self, provider: &str, model: &str) -> Option<&ModelCard> {
        self.get_provider(provider)
            .and_then(|p| p.models.iter().find(|m| m.name == model))
    }

    /// Get all LLM models across all providers.
    pub fn all_llm_models(&self) -> Vec<(&ProviderConfig, &ModelCard)> {
        self.providers
            .iter()
            .filter(|p| p.enabled)
            .flat_map(|p| {
                p.models
                    .iter()
                    .filter(|m| matches!(m.model_type, ModelType::Llm | ModelType::Multimodal))
                    .map(move |m| (p, m))
            })
            .collect()
    }

    /// Get all embedding models across all providers.
    ///
    /// # WHY: Exclude Multimodal from embedding list
    ///
    /// In EdgeQuake, "multimodal" refers to vision-capable LLMs (text + image input),
    /// NOT models that can do both embedding AND text generation.
    /// Embedding models are strictly for vector generation (model_type = "embedding").
    /// Vision LLMs should only appear in the LLM selector, not the embedding selector.
    pub fn all_embedding_models(&self) -> Vec<(&ProviderConfig, &ModelCard)> {
        self.providers
            .iter()
            .filter(|p| p.enabled)
            .flat_map(|p| {
                p.models
                    .iter()
                    // WHY: Only include pure embedding models, NOT multimodal (vision LLMs)
                    .filter(|m| matches!(m.model_type, ModelType::Embedding))
                    .map(move |m| (p, m))
            })
            .collect()
    }

    /// Get the default LLM provider and model.
    pub fn default_llm(&self) -> Option<(&ProviderConfig, &ModelCard)> {
        self.get_model(&self.defaults.llm_provider, &self.defaults.llm_model)
            .and_then(|m| {
                self.get_provider(&self.defaults.llm_provider)
                    .map(|p| (p, m))
            })
    }

    /// Get the default embedding provider and model.
    pub fn default_embedding(&self) -> Option<(&ProviderConfig, &ModelCard)> {
        self.get_model(
            &self.defaults.embedding_provider,
            &self.defaults.embedding_model,
        )
        .and_then(|m| {
            self.get_provider(&self.defaults.embedding_provider)
                .map(|p| (p, m))
        })
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), ModelConfigError> {
        // Check that default providers exist
        if self.get_provider(&self.defaults.llm_provider).is_none() {
            return Err(ModelConfigError::ValidationError(format!(
                "Default LLM provider '{}' not found in providers list",
                self.defaults.llm_provider
            )));
        }

        if self
            .get_provider(&self.defaults.embedding_provider)
            .is_none()
        {
            return Err(ModelConfigError::ValidationError(format!(
                "Default embedding provider '{}' not found in providers list",
                self.defaults.embedding_provider
            )));
        }

        // Check that default models exist
        if self
            .get_model(&self.defaults.llm_provider, &self.defaults.llm_model)
            .is_none()
        {
            return Err(ModelConfigError::ValidationError(format!(
                "Default LLM model '{}' not found in provider '{}'",
                self.defaults.llm_model, self.defaults.llm_provider
            )));
        }

        if self
            .get_model(
                &self.defaults.embedding_provider,
                &self.defaults.embedding_model,
            )
            .is_none()
        {
            return Err(ModelConfigError::ValidationError(format!(
                "Default embedding model '{}' not found in provider '{}'",
                self.defaults.embedding_model, self.defaults.embedding_provider
            )));
        }

        // Check for duplicate provider names
        let mut seen_providers = std::collections::HashSet::new();
        for provider in &self.providers {
            if !seen_providers.insert(&provider.name) {
                return Err(ModelConfigError::ValidationError(format!(
                    "Duplicate provider name: '{}'",
                    provider.name
                )));
            }

            // Check for duplicate model names within a provider
            let mut seen_models = std::collections::HashSet::new();
            for model in &provider.models {
                if !seen_models.insert(&model.name) {
                    return Err(ModelConfigError::ValidationError(format!(
                        "Duplicate model name '{}' in provider '{}'",
                        model.name, provider.name
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_defaults() {
        let config = ModelsConfig::builtin_defaults();
        assert!(config.validate().is_ok());
        assert!(!config.providers.is_empty());
    }

    #[test]
    fn test_get_provider() {
        let config = ModelsConfig::builtin_defaults();
        assert!(config.get_provider("openai").is_some());
        assert!(config.get_provider("ollama").is_some());
        assert!(config.get_provider("nonexistent").is_none());
    }

    #[test]
    fn test_get_model() {
        let config = ModelsConfig::builtin_defaults();
        assert!(config.get_model("openai", "gpt-4o").is_some());
        assert!(config.get_model("ollama", "nomic-embed-text").is_some());
        assert!(config.get_model("openai", "nonexistent").is_none());
    }

    #[test]
    fn test_all_llm_models() {
        let config = ModelsConfig::builtin_defaults();
        let llm_models = config.all_llm_models();
        assert!(!llm_models.is_empty());
        assert!(llm_models.iter().any(|(_, m)| m.name == "gpt-4o"));
    }

    #[test]
    fn test_all_embedding_models() {
        let config = ModelsConfig::builtin_defaults();
        let embedding_models = config.all_embedding_models();
        assert!(!embedding_models.is_empty());
        assert!(embedding_models
            .iter()
            .any(|(_, m)| m.name == "text-embedding-3-small"));
    }

    #[test]
    fn test_toml_roundtrip() {
        let config = ModelsConfig::builtin_defaults();
        let toml_str = config.to_toml().expect("Failed to serialize");
        let parsed: ModelsConfig = ModelsConfig::from_toml(&toml_str).expect("Failed to parse");
        assert_eq!(config.providers.len(), parsed.providers.len());
    }

    #[test]
    fn test_model_capabilities() {
        let config = ModelsConfig::builtin_defaults();
        let gpt4o = config
            .get_model("openai", "gpt-4o")
            .expect("gpt-4o should exist");
        assert!(gpt4o.capabilities.supports_vision);
        assert!(gpt4o.capabilities.supports_function_calling);
        assert_eq!(gpt4o.capabilities.context_length, 128000);
    }

    #[test]
    fn test_embedding_dimensions() {
        let config = ModelsConfig::builtin_defaults();

        let openai_embed = config
            .get_model("openai", "text-embedding-3-small")
            .unwrap();
        assert_eq!(openai_embed.capabilities.embedding_dimension, 1536);

        let ollama_embed = config.get_model("ollama", "nomic-embed-text").unwrap();
        assert_eq!(ollama_embed.capabilities.embedding_dimension, 768);
    }

    #[test]
    fn test_validation_duplicate_provider() {
        let mut config = ModelsConfig::builtin_defaults();
        config.providers.push(config.providers[0].clone());
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_parse_models_toml_file() {
        // Read the actual models.toml file from the project root
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let toml_path = std::path::Path::new(&manifest_dir)
            .parent() // crates/
            .unwrap()
            .parent() // edgequake/
            .unwrap()
            .join("models.toml");

        if toml_path.exists() {
            let content = std::fs::read_to_string(&toml_path).expect("Failed to read models.toml");
            let config = ModelsConfig::from_toml(&content).expect("Failed to parse models.toml");

            // Validate the parsed config
            assert!(config.validate().is_ok(), "models.toml failed validation");

            // Check we have expected providers
            assert!(
                config.get_provider("openai").is_some(),
                "OpenAI provider should exist"
            );
            assert!(
                config.get_provider("ollama").is_some(),
                "Ollama provider should exist"
            );
            assert!(
                config.get_provider("lmstudio").is_some(),
                "LM Studio provider should exist"
            );
            assert!(
                config.get_provider("mock").is_some(),
                "Mock provider should exist"
            );

            // Check default selections are set
            assert!(!config.defaults.llm_provider.is_empty());
            assert!(!config.defaults.llm_model.is_empty());
            assert!(!config.defaults.embedding_provider.is_empty());
            assert!(!config.defaults.embedding_model.is_empty());

            // Check we have LLM and embedding models
            let llm_models = config.all_llm_models();
            let embedding_models = config.all_embedding_models();
            assert!(!llm_models.is_empty(), "Should have LLM models");
            assert!(!embedding_models.is_empty(), "Should have embedding models");
        }
    }

    #[test]
    fn test_provider_priorities() {
        let config = ModelsConfig::builtin_defaults();
        let mut priorities: Vec<(String, u32)> = config
            .providers
            .iter()
            .map(|p| (p.name.clone(), p.priority))
            .collect();
        priorities.sort_by_key(|(_, p)| *p);

        // Lower priority means higher preference
        // OpenAI should have lower priority number than Mock
        let openai_prio = config.get_provider("openai").unwrap().priority;
        let mock_prio = config.get_provider("mock").unwrap().priority;
        assert!(
            openai_prio < mock_prio,
            "OpenAI should have higher priority than mock"
        );
    }
}
