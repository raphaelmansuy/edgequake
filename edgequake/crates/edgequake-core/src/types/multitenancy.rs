//! Multi-tenant workspace and isolation types.
//!
//! This module provides the domain types for multi-tenant isolation:
//! - `Tenant` - A top-level organization/customer
//! - `Workspace` - A document workspace within a tenant (knowledge base)
//! - `Membership` - User access to tenants/workspaces
//! - `TenantContext` - Current request context for RLS

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A tenant represents an organization or customer in the multi-tenant system.
///
/// ## Model Configuration (SPEC-032)
///
/// Each tenant has default LLM and embedding model configuration that serves as:
/// - **Defaults for new workspaces**: When a workspace is created without explicit model config,
///   it inherits the tenant's model configuration.
/// - **Organization-wide policy**: Tenant admins can set preferred providers and models
///   for all workspaces in their organization.
///
/// Workspaces can override these defaults with their own model configuration.
///
/// ## Model ID Format
///
/// Models are identified by `provider/model_name` format:
/// - `"ollama/gemma3:12b"` - Ollama with Gemma 3 12B
/// - `"openai/gpt-4o-mini"` - OpenAI GPT-4o Mini
/// - `"lmstudio/gemma-3n-e4b-it"` - LM Studio local model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    /// Unique tenant identifier.
    pub tenant_id: Uuid,
    /// Human-readable name.
    pub name: String,
    /// URL-safe slug for routing.
    pub slug: String,
    /// Optional description.
    pub description: Option<String>,
    /// Subscription plan.
    pub plan: TenantPlan,
    /// Maximum number of workspaces allowed.
    pub max_workspaces: usize,
    /// Maximum number of users allowed.
    pub max_users: usize,
    /// Whether the tenant is active.
    pub is_active: bool,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Custom metadata.
    pub metadata: HashMap<String, serde_json::Value>,

    // === Default LLM Configuration (SPEC-032) ===
    /// Default LLM model name for new workspaces (e.g., "gemma3:12b", "gpt-4o-mini").
    /// Used for knowledge graph generation, summarization, entity extraction.
    /// Workspaces inherit this if not explicitly configured.
    pub default_llm_model: String,

    /// Default LLM provider for new workspaces (e.g., "ollama", "openai", "lmstudio").
    /// Workspaces inherit this if not explicitly configured.
    pub default_llm_provider: String,

    // === Default Embedding Configuration (SPEC-032) ===
    /// Default embedding model name for new workspaces (e.g., "text-embedding-3-small").
    /// Workspaces inherit this if not explicitly configured.
    pub default_embedding_model: String,

    /// Default embedding provider for new workspaces (e.g., "openai", "ollama", "lmstudio").
    /// Workspaces inherit this if not explicitly configured.
    pub default_embedding_provider: String,

    /// Default embedding dimension for new workspaces (e.g., 1536 for OpenAI, 768 for Ollama).
    /// Workspaces inherit this if not explicitly configured.
    pub default_embedding_dimension: usize,
}

impl Tenant {
    /// Create a new tenant with defaults.
    ///
    /// Uses server defaults from environment variables for model configuration:
    /// - `EDGEQUAKE_DEFAULT_LLM_MODEL`
    /// - `EDGEQUAKE_DEFAULT_LLM_PROVIDER`
    /// - `EDGEQUAKE_DEFAULT_EMBEDDING_MODEL`
    /// - `EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER`
    /// - `EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION`
    pub fn new(name: impl Into<String>, slug: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        let (default_llm_model, default_llm_provider) = Workspace::default_llm_config();
        let (default_embedding_model, default_embedding_provider, default_embedding_dimension) =
            Workspace::default_embedding_config();

        Self {
            tenant_id: Uuid::new_v4(),
            name: name.into(),
            slug: slug.into(),
            description: None,
            plan: TenantPlan::Free,
            max_workspaces: 5,
            max_users: 10,
            is_active: true,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
            default_llm_model,
            default_llm_provider,
            default_embedding_model,
            default_embedding_provider,
            default_embedding_dimension,
        }
    }

    /// Set the tenant plan.
    pub fn with_plan(mut self, plan: TenantPlan) -> Self {
        self.plan = plan;
        self.max_workspaces = plan.default_max_workspaces();
        self.max_users = plan.default_max_users();
        self
    }

    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set the default LLM configuration for new workspaces.
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_core::Tenant;
    ///
    /// let tenant = Tenant::new("Acme Corp", "acme")
    ///     .with_llm_config("gemma3:12b", "ollama");
    /// assert_eq!(tenant.default_llm_model, "gemma3:12b");
    /// assert_eq!(tenant.default_llm_provider, "ollama");
    /// ```
    pub fn with_llm_config(
        mut self,
        model: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        self.default_llm_model = model.into();
        self.default_llm_provider = provider.into();
        self
    }

    /// Set the default embedding configuration for new workspaces.
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_core::Tenant;
    ///
    /// let tenant = Tenant::new("Acme Corp", "acme")
    ///     .with_embedding_config("text-embedding-3-small", "openai", 1536);
    /// assert_eq!(tenant.default_embedding_model, "text-embedding-3-small");
    /// assert_eq!(tenant.default_embedding_provider, "openai");
    /// assert_eq!(tenant.default_embedding_dimension, 1536);
    /// ```
    pub fn with_embedding_config(
        mut self,
        model: impl Into<String>,
        provider: impl Into<String>,
        dimension: usize,
    ) -> Self {
        self.default_embedding_model = model.into();
        self.default_embedding_provider = provider.into();
        self.default_embedding_dimension = dimension;
        self
    }
}

/// Tenant subscription plans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TenantPlan {
    /// Free tier with limited resources.
    #[default]
    Free,
    /// Basic paid tier.
    Basic,
    /// Professional tier.
    Pro,
    /// Enterprise tier with custom limits.
    Enterprise,
}

impl TenantPlan {
    /// Get the default max workspaces for this plan.
    ///
    /// SPEC-028: Updated to support 500 workspaces by default for Pro/Enterprise.
    /// WHY: Enable large-scale knowledge base organization without artificial limits.
    pub fn default_max_workspaces(&self) -> usize {
        match self {
            TenantPlan::Free => 10,        // Reasonable for trials
            TenantPlan::Basic => 100,      // Small teams
            TenantPlan::Pro => 500,        // SPEC-028: 500 workspaces target
            TenantPlan::Enterprise => 500, // SPEC-028: 500 workspaces target
        }
    }

    /// Get the default max users for this plan.
    pub fn default_max_users(&self) -> usize {
        match self {
            TenantPlan::Free => 3,
            TenantPlan::Basic => 10,
            TenantPlan::Pro => 50,
            TenantPlan::Enterprise => 500,
        }
    }

    /// Get the default max documents per workspace.
    pub fn default_max_documents(&self) -> usize {
        match self {
            TenantPlan::Free => 100,
            TenantPlan::Basic => 1000,
            TenantPlan::Pro => 10000,
            TenantPlan::Enterprise => 100000,
        }
    }
}

impl std::fmt::Display for TenantPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TenantPlan::Free => write!(f, "free"),
            TenantPlan::Basic => write!(f, "basic"),
            TenantPlan::Pro => write!(f, "pro"),
            TenantPlan::Enterprise => write!(f, "enterprise"),
        }
    }
}

impl std::str::FromStr for TenantPlan {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "free" => Ok(TenantPlan::Free),
            "basic" => Ok(TenantPlan::Basic),
            "pro" => Ok(TenantPlan::Pro),
            "enterprise" => Ok(TenantPlan::Enterprise),
            _ => Err(format!("Unknown plan: {}", s)),
        }
    }
}

/// A workspace (knowledge base) within a tenant.
///
/// ## Model Configuration (SPEC-032)
///
/// Each workspace has its own model configuration that determines:
/// - **Embedding**: Which model to use for vector embeddings (must match stored vectors)
/// - **LLM**: Which model to use for knowledge graph generation, summarization, etc.
///
/// This enables mixing different providers per workspace:
/// - Workspace A: OpenAI gpt-4o + text-embedding-3-small (1536 dims)
/// - Workspace B: Ollama gemma3:12b + embeddinggemma:latest (768 dims)
///
/// ## Model ID Format
///
/// Models are identified by `provider/model_name` format:
/// - `"ollama/gemma3:12b"` - Ollama with Gemma 3 12B
/// - `"openai/gpt-4o-mini"` - OpenAI GPT-4o Mini
/// - `"lmstudio/gemma-3n-e4b-it"` - LM Studio local model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Unique workspace identifier.
    pub workspace_id: Uuid,
    /// Owning tenant ID.
    pub tenant_id: Uuid,
    /// Human-readable name.
    pub name: String,
    /// URL-safe slug (unique within tenant).
    pub slug: String,
    /// Optional description.
    pub description: Option<String>,
    /// Whether the workspace is active.
    pub is_active: bool,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Custom metadata including quotas.
    pub metadata: HashMap<String, serde_json::Value>,

    // === LLM Configuration (SPEC-032) ===
    /// LLM model name (e.g., "gemma3:12b", "gpt-4o-mini").
    /// Used for knowledge graph generation, summarization, entity extraction.
    /// Note: Query-time LLM can be different (user's choice in UI).
    pub llm_model: String,

    /// LLM provider (e.g., "ollama", "openai", "lmstudio").
    /// Determines which API to call for LLM completions during ingestion.
    pub llm_provider: String,

    // === Embedding Configuration (SPEC-032) ===
    /// Embedding model name (e.g., "text-embedding-3-small", "embeddinggemma:latest").
    /// Used for both document ingestion and query embedding generation.
    /// MUST be consistent: query embeddings must use same model as stored vectors.
    pub embedding_model: String,

    /// Embedding provider (e.g., "openai", "ollama", "lmstudio").
    /// Determines which API to call for embedding generation.
    pub embedding_provider: String,

    /// Embedding dimension (e.g., 1536 for OpenAI, 768 for Ollama).
    /// Must match the stored vector dimensions in this workspace.
    pub embedding_dimension: usize,
}

// ============================================================================
// Model Configuration Constants (SPEC-032)
// ============================================================================
// These defaults MUST match models.toml [defaults] section.
// Ollama is used by default for both LLM and embedding to enable
// development without requiring API keys.
//
// To use OpenAI or other providers, set environment variables:
//   - EDGEQUAKE_DEFAULT_LLM_PROVIDER=openai
//   - EDGEQUAKE_DEFAULT_LLM_MODEL=gpt-4o-mini
//   - EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER=openai
//   - EDGEQUAKE_DEFAULT_EMBEDDING_MODEL=text-embedding-3-small
//   - EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION=1536

/// Default LLM model (Ollama gemma3:12b - 128K context, vision support).
pub const DEFAULT_LLM_MODEL: &str = "gemma3:12b";

/// Default LLM provider.
pub const DEFAULT_LLM_PROVIDER: &str = "ollama";

/// Default embedding model (Ollama embeddinggemma - 768 dimensions, 2K context).
/// Synced with models.toml [defaults] section.
pub const DEFAULT_EMBEDDING_MODEL: &str = "embeddinggemma";

/// Default embedding provider.
/// Synced with models.toml [defaults] section.
pub const DEFAULT_EMBEDDING_PROVIDER: &str = "ollama";

/// Default embedding dimension (Ollama embeddinggemma).
/// Synced with models.toml [defaults] section.
pub const DEFAULT_EMBEDDING_DIMENSION: usize = 768;

impl Workspace {
    /// Create a new workspace with default model configuration.
    ///
    /// Uses server defaults from environment variables if set:
    /// - `EDGEQUAKE_DEFAULT_LLM_MODEL`
    /// - `EDGEQUAKE_DEFAULT_LLM_PROVIDER`
    /// - `EDGEQUAKE_DEFAULT_EMBEDDING_MODEL`
    /// - `EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER`
    /// - `EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION`
    pub fn new(tenant_id: Uuid, name: impl Into<String>, slug: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        let (llm_model, llm_provider) = Self::default_llm_config();
        let (embedding_model, embedding_provider, embedding_dimension) =
            Self::default_embedding_config();

        Self {
            workspace_id: Uuid::new_v4(),
            tenant_id,
            name: name.into(),
            slug: slug.into(),
            description: None,
            is_active: true,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
            llm_model,
            llm_provider,
            embedding_model,
            embedding_provider,
            embedding_dimension,
        }
    }

    /// Get default LLM configuration from environment.
    ///
    /// Returns (model, provider) tuple.
    pub fn default_llm_config() -> (String, String) {
        let model = std::env::var("EDGEQUAKE_DEFAULT_LLM_MODEL")
            .unwrap_or_else(|_| DEFAULT_LLM_MODEL.to_string());

        let provider = std::env::var("EDGEQUAKE_DEFAULT_LLM_PROVIDER")
            .unwrap_or_else(|_| DEFAULT_LLM_PROVIDER.to_string());

        (model, provider)
    }

    /// Get default embedding configuration from environment.
    ///
    /// Returns (model, provider, dimension) tuple.
    pub fn default_embedding_config() -> (String, String, usize) {
        let model = std::env::var("EDGEQUAKE_DEFAULT_EMBEDDING_MODEL")
            .unwrap_or_else(|_| DEFAULT_EMBEDDING_MODEL.to_string());

        let provider = std::env::var("EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER")
            .unwrap_or_else(|_| Self::detect_provider_from_model(&model));

        let dimension = std::env::var("EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION")
            .and_then(|s| s.parse().map_err(|_| std::env::VarError::NotPresent))
            .unwrap_or_else(|_| Self::detect_dimension_from_model(&model));

        (model, provider, dimension)
    }

    /// Auto-detect provider from model name conventions.
    ///
    /// # Examples
    ///
    /// - "text-embedding-3-small" → "openai"
    /// - "gemma3:12b" → "ollama" (colon indicates Ollama tag format)
    /// - "gemma2-9b-it" → "lmstudio"
    pub fn detect_provider_from_model(model: &str) -> String {
        if model.starts_with("text-embedding") || model.starts_with("ada") {
            "openai".to_string()
        } else if model.contains(':') {
            // Ollama uses "model:tag" format
            "ollama".to_string()
        } else if model.starts_with("gemma") || model.starts_with("llama") {
            "lmstudio".to_string()
        } else {
            // Default fallback to openai
            "openai".to_string()
        }
    }

    /// Auto-detect embedding dimension from known model names.
    ///
    /// # Known Models
    ///
    /// | Model | Dimension |
    /// |-------|-----------|
    /// | text-embedding-3-small | 1536 |
    /// | text-embedding-3-large | 3072 |
    /// | text-embedding-ada-002 | 1536 |
    /// | embeddinggemma:latest | 768 |
    /// | nomic-embed-text | 768 |
    /// | mxbai-embed-large | 1024 |
    pub fn detect_dimension_from_model(model: &str) -> usize {
        match model {
            "text-embedding-3-small" | "text-embedding-ada-002" => 1536,
            "text-embedding-3-large" => 3072,
            "embeddinggemma:latest" | "nomic-embed-text" | "nomic-embed-text:latest" => 768,
            "mxbai-embed-large" | "mxbai-embed-large:latest" => 1024,
            _ if model.contains("768") => 768,
            _ if model.contains("1024") => 1024,
            _ if model.contains("3072") => 3072,
            _ => DEFAULT_EMBEDDING_DIMENSION, // Safe default
        }
    }

    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set max documents quota.
    pub fn with_max_documents(mut self, max: usize) -> Self {
        self.metadata
            .insert("max_documents".to_string(), serde_json::json!(max));
        self
    }

    /// Get max documents quota.
    pub fn max_documents(&self) -> Option<usize> {
        self.metadata
            .get("max_documents")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
    }

    // === Embedding Configuration Builder Methods (SPEC-032) ===

    /// Set the embedding model and auto-detect provider/dimension.
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_core::Workspace;
    /// use uuid::Uuid;
    ///
    /// let workspace = Workspace::new(Uuid::new_v4(), "My Workspace", "my-workspace")
    ///     .with_embedding_model("embeddinggemma:latest");
    ///
    /// assert_eq!(workspace.embedding_model, "embeddinggemma:latest");
    /// assert_eq!(workspace.embedding_provider, "ollama");
    /// assert_eq!(workspace.embedding_dimension, 768);
    /// ```
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        let model = model.into();
        self.embedding_provider = Self::detect_provider_from_model(&model);
        self.embedding_dimension = Self::detect_dimension_from_model(&model);
        self.embedding_model = model;
        self
    }

    /// Set the embedding provider explicitly.
    pub fn with_embedding_provider(mut self, provider: impl Into<String>) -> Self {
        self.embedding_provider = provider.into();
        self
    }

    /// Set the embedding dimension explicitly.
    ///
    /// Use this when auto-detection doesn't work for custom models.
    pub fn with_embedding_dimension(mut self, dimension: usize) -> Self {
        self.embedding_dimension = dimension;
        self
    }

    /// Set complete embedding configuration.
    ///
    /// # Arguments
    ///
    /// * `model` - Embedding model name
    /// * `provider` - Provider name (openai, ollama, lmstudio)
    /// * `dimension` - Vector dimension
    pub fn with_embedding_config(
        mut self,
        model: impl Into<String>,
        provider: impl Into<String>,
        dimension: usize,
    ) -> Self {
        self.embedding_model = model.into();
        self.embedding_provider = provider.into();
        self.embedding_dimension = dimension;
        self
    }

    // === LLM Configuration Builder Methods (SPEC-032) ===

    /// Set the LLM model and auto-detect provider.
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_core::Workspace;
    /// use uuid::Uuid;
    ///
    /// let workspace = Workspace::new(Uuid::new_v4(), "My Workspace", "my-workspace")
    ///     .with_llm_model("gemma3:12b");
    ///
    /// assert_eq!(workspace.llm_model, "gemma3:12b");
    /// assert_eq!(workspace.llm_provider, "ollama");
    /// ```
    pub fn with_llm_model(mut self, model: impl Into<String>) -> Self {
        let model = model.into();
        self.llm_provider = Self::detect_provider_from_model(&model);
        self.llm_model = model;
        self
    }

    /// Set the LLM provider explicitly.
    pub fn with_llm_provider(mut self, provider: impl Into<String>) -> Self {
        self.llm_provider = provider.into();
        self
    }

    /// Set complete LLM configuration.
    ///
    /// # Arguments
    ///
    /// * `model` - LLM model name
    /// * `provider` - Provider name (openai, ollama, lmstudio)
    pub fn with_llm_config(
        mut self,
        model: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        self.llm_model = model.into();
        self.llm_provider = provider.into();
        self
    }

    // === Full Model ID Methods (SPEC-032) ===

    /// Get fully qualified LLM model ID in `provider/model` format.
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_core::Workspace;
    /// use uuid::Uuid;
    ///
    /// let workspace = Workspace::new(Uuid::new_v4(), "Test", "test")
    ///     .with_llm_config("gemma3:12b", "ollama");
    ///
    /// assert_eq!(workspace.llm_full_id(), "ollama/gemma3:12b");
    /// ```
    pub fn llm_full_id(&self) -> String {
        format!("{}/{}", self.llm_provider, self.llm_model)
    }

    /// Get fully qualified embedding model ID in `provider/model` format.
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_core::Workspace;
    /// use uuid::Uuid;
    ///
    /// let workspace = Workspace::new(Uuid::new_v4(), "Test", "test")
    ///     .with_embedding_config("text-embedding-3-small", "openai", 1536);
    ///
    /// assert_eq!(workspace.embedding_full_id(), "openai/text-embedding-3-small");
    /// ```
    pub fn embedding_full_id(&self) -> String {
        format!("{}/{}", self.embedding_provider, self.embedding_model)
    }

    /// Parse a full model ID into (provider, model) tuple.
    ///
    /// # Arguments
    ///
    /// * `full_id` - Model ID in `provider/model` format (e.g., "ollama/gemma3:12b")
    ///
    /// # Returns
    ///
    /// `Some((provider, model))` if valid format, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_core::Workspace;
    ///
    /// assert_eq!(
    ///     Workspace::parse_model_id("ollama/gemma3:12b"),
    ///     Some(("ollama".to_string(), "gemma3:12b".to_string()))
    /// );
    ///
    /// assert_eq!(Workspace::parse_model_id("invalid"), None);
    /// ```
    pub fn parse_model_id(full_id: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = full_id.splitn(2, '/').collect();
        if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }
}

/// A user's membership in a tenant/workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Membership {
    /// Unique membership identifier.
    pub membership_id: Uuid,
    /// User ID (from auth system).
    pub user_id: Uuid,
    /// Tenant ID.
    pub tenant_id: Uuid,
    /// Optional workspace ID (None = all workspaces in tenant).
    pub workspace_id: Option<Uuid>,
    /// Role within the tenant/workspace.
    pub role: MembershipRole,
    /// Whether the membership is active.
    pub is_active: bool,
    /// When the user joined.
    pub joined_at: chrono::DateTime<chrono::Utc>,
    /// Custom metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Membership {
    /// Create a new membership.
    pub fn new(user_id: Uuid, tenant_id: Uuid, role: MembershipRole) -> Self {
        Self {
            membership_id: Uuid::new_v4(),
            user_id,
            tenant_id,
            workspace_id: None,
            role,
            is_active: true,
            joined_at: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Scope membership to a specific workspace.
    pub fn for_workspace(mut self, workspace_id: Uuid) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }

    /// Check if user has at least the given role.
    pub fn has_role(&self, required: MembershipRole) -> bool {
        self.role.level() >= required.level()
    }

    /// Check if user can access a specific workspace.
    pub fn can_access_workspace(&self, workspace_id: &Uuid) -> bool {
        if !self.is_active {
            return false;
        }
        // None = access to all workspaces
        self.workspace_id.is_none() || self.workspace_id.as_ref() == Some(workspace_id)
    }
}

/// Roles for tenant/workspace membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MembershipRole {
    /// Read-only access.
    Readonly,
    /// Standard member (read/write).
    #[default]
    Member,
    /// Administrator (can manage users).
    Admin,
    /// Owner (full control, can delete tenant).
    Owner,
}

impl MembershipRole {
    /// Get the permission level (higher = more permissions).
    pub fn level(&self) -> u8 {
        match self {
            MembershipRole::Readonly => 1,
            MembershipRole::Member => 2,
            MembershipRole::Admin => 3,
            MembershipRole::Owner => 4,
        }
    }

    /// Check if this role can write data.
    pub fn can_write(&self) -> bool {
        matches!(
            self,
            MembershipRole::Member | MembershipRole::Admin | MembershipRole::Owner
        )
    }

    /// Check if this role can manage users.
    pub fn can_manage_users(&self) -> bool {
        matches!(self, MembershipRole::Admin | MembershipRole::Owner)
    }

    /// Check if this role can delete the tenant.
    pub fn can_delete_tenant(&self) -> bool {
        matches!(self, MembershipRole::Owner)
    }
}

impl std::fmt::Display for MembershipRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MembershipRole::Readonly => write!(f, "readonly"),
            MembershipRole::Member => write!(f, "member"),
            MembershipRole::Admin => write!(f, "admin"),
            MembershipRole::Owner => write!(f, "owner"),
        }
    }
}

impl std::str::FromStr for MembershipRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "readonly" => Ok(MembershipRole::Readonly),
            "member" => Ok(MembershipRole::Member),
            "admin" => Ok(MembershipRole::Admin),
            "owner" => Ok(MembershipRole::Owner),
            _ => Err(format!("Unknown role: {}", s)),
        }
    }
}

/// Context for the current tenant/workspace scope.
///
/// This is used to set PostgreSQL session variables for RLS enforcement.
#[derive(Debug, Clone, Default)]
pub struct TenantContext {
    /// Current tenant ID.
    pub tenant_id: Option<Uuid>,
    /// Current workspace ID.
    pub workspace_id: Option<Uuid>,
    /// Current user ID.
    pub user_id: Option<Uuid>,
    /// User's role in the current context.
    pub role: Option<MembershipRole>,
}

impl TenantContext {
    /// Create a new tenant context.
    pub fn new(tenant_id: Uuid) -> Self {
        Self {
            tenant_id: Some(tenant_id),
            workspace_id: None,
            user_id: None,
            role: None,
        }
    }

    /// Set the workspace scope.
    pub fn with_workspace(mut self, workspace_id: Uuid) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }

    /// Set the user context.
    pub fn with_user(mut self, user_id: Uuid, role: MembershipRole) -> Self {
        self.user_id = Some(user_id);
        self.role = Some(role);
        self
    }

    /// Check if the context is valid (has at least tenant_id).
    pub fn is_valid(&self) -> bool {
        self.tenant_id.is_some()
    }

    /// Check if user can write in this context.
    pub fn can_write(&self) -> bool {
        self.role.map(|r| r.can_write()).unwrap_or(false)
    }
}

/// Request to create a new workspace.
///
/// ## Model Configuration (SPEC-032)
///
/// If `embedding_model` is not provided, the workspace will use server defaults:
/// - `EDGEQUAKE_DEFAULT_EMBEDDING_MODEL` or "text-embedding-3-small"
/// - Provider and dimension auto-detected from model name
///
/// If `llm_model` is not provided, the workspace will use server defaults:
/// - `EDGEQUAKE_DEFAULT_LLM_MODEL` or "gemma3:12b" (Ollama)
/// - Provider auto-detected from model name
///
/// ## Model ID Format
///
/// Models can be specified as:
/// - Simple name: "gemma3:12b" (provider auto-detected)
/// - Full ID: "ollama/gemma3:12b" (provider parsed from full ID)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateWorkspaceRequest {
    /// Human-readable name.
    pub name: String,
    /// Optional slug (generated from name if not provided).
    pub slug: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Optional max documents quota.
    pub max_documents: Option<usize>,

    // === LLM Configuration (SPEC-032) ===
    /// LLM model name (e.g., "gemma3:12b", "gpt-4o-mini").
    /// If None, uses server default from EDGEQUAKE_DEFAULT_LLM_MODEL.
    /// Can be a full ID like "ollama/gemma3:12b" for explicit provider.
    pub llm_model: Option<String>,

    /// LLM provider (e.g., "ollama", "openai", "lmstudio").
    /// If None, auto-detected from llm_model.
    pub llm_provider: Option<String>,

    // === Embedding Configuration (SPEC-032) ===
    /// Embedding model name (e.g., "text-embedding-3-small", "embeddinggemma:latest").
    /// If None, uses server default from EDGEQUAKE_DEFAULT_EMBEDDING_MODEL.
    /// Can be a full ID like "openai/text-embedding-3-small" for explicit provider.
    pub embedding_model: Option<String>,

    /// Embedding provider (e.g., "openai", "ollama", "lmstudio").
    /// If None, auto-detected from embedding_model.
    pub embedding_provider: Option<String>,

    /// Embedding dimension override.
    /// If None, auto-detected from embedding_model.
    pub embedding_dimension: Option<usize>,
}

impl CreateWorkspaceRequest {
    /// Create a new request with just a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    // === LLM Configuration Builder Methods (SPEC-032) ===

    /// Set the LLM model.
    ///
    /// # Arguments
    ///
    /// * `model` - Model name or full ID (e.g., "gemma3:12b" or "ollama/gemma3:12b")
    pub fn with_llm_model(mut self, model: impl Into<String>) -> Self {
        self.llm_model = Some(model.into());
        self
    }

    /// Set the LLM provider.
    pub fn with_llm_provider(mut self, provider: impl Into<String>) -> Self {
        self.llm_provider = Some(provider.into());
        self
    }

    /// Set complete LLM configuration from a full model ID.
    ///
    /// # Arguments
    ///
    /// * `full_id` - Full model ID in `provider/model` format (e.g., "ollama/gemma3:12b")
    ///
    /// If the format is invalid, sets the entire string as the model name.
    pub fn with_llm_full_id(mut self, full_id: impl Into<String>) -> Self {
        let full_id = full_id.into();
        if let Some((provider, model)) = Workspace::parse_model_id(&full_id) {
            self.llm_provider = Some(provider);
            self.llm_model = Some(model);
        } else {
            self.llm_model = Some(full_id);
        }
        self
    }

    // === Embedding Configuration Builder Methods (SPEC-032) ===

    /// Set the embedding model.
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.embedding_model = Some(model.into());
        self
    }

    /// Set the embedding provider.
    pub fn with_embedding_provider(mut self, provider: impl Into<String>) -> Self {
        self.embedding_provider = Some(provider.into());
        self
    }

    /// Set the embedding dimension.
    pub fn with_embedding_dimension(mut self, dimension: usize) -> Self {
        self.embedding_dimension = Some(dimension);
        self
    }

    /// Set complete embedding configuration from a full model ID.
    ///
    /// # Arguments
    ///
    /// * `full_id` - Full model ID in `provider/model` format (e.g., "openai/text-embedding-3-small")
    ///
    /// If the format is invalid, sets the entire string as the model name.
    pub fn with_embedding_full_id(mut self, full_id: impl Into<String>) -> Self {
        let full_id = full_id.into();
        if let Some((provider, model)) = Workspace::parse_model_id(&full_id) {
            self.embedding_provider = Some(provider);
            self.embedding_model = Some(model);
        } else {
            self.embedding_model = Some(full_id);
        }
        self
    }

    /// Set complete LLM configuration with model and provider.
    ///
    /// # Arguments
    ///
    /// * `model` - LLM model name (e.g., "gemma3:12b", "gpt-4o-mini")
    /// * `provider` - Provider name (e.g., "ollama", "openai", "lmstudio")
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_core::CreateWorkspaceRequest;
    ///
    /// let req = CreateWorkspaceRequest::new("My Workspace")
    ///     .with_llm_config("gemma3:12b", "ollama");
    /// assert_eq!(req.llm_model, Some("gemma3:12b".to_string()));
    /// assert_eq!(req.llm_provider, Some("ollama".to_string()));
    /// ```
    pub fn with_llm_config(
        mut self,
        model: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        self.llm_model = Some(model.into());
        self.llm_provider = Some(provider.into());
        self
    }

    /// Set complete embedding configuration with model, provider, and dimension.
    ///
    /// # Arguments
    ///
    /// * `model` - Embedding model name (e.g., "text-embedding-3-small")
    /// * `provider` - Provider name (e.g., "openai", "ollama", "lmstudio")
    /// * `dimension` - Vector dimension (e.g., 1536, 768, 3072)
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_core::CreateWorkspaceRequest;
    ///
    /// let req = CreateWorkspaceRequest::new("My Workspace")
    ///     .with_embedding_config("text-embedding-3-small", "openai", 1536);
    /// assert_eq!(req.embedding_model, Some("text-embedding-3-small".to_string()));
    /// assert_eq!(req.embedding_provider, Some("openai".to_string()));
    /// assert_eq!(req.embedding_dimension, Some(1536));
    /// ```
    pub fn with_embedding_config(
        mut self,
        model: impl Into<String>,
        provider: impl Into<String>,
        dimension: usize,
    ) -> Self {
        self.embedding_model = Some(model.into());
        self.embedding_provider = Some(provider.into());
        self.embedding_dimension = Some(dimension);
        self
    }
}

/// Request to update a workspace.
///
/// ## Model Configuration (SPEC-032)
///
/// - LLM model/provider changes take effect immediately for new ingestions
/// - Embedding model changes require vector rebuild (use rebuild-embeddings endpoint)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateWorkspaceRequest {
    /// New name (optional).
    pub name: Option<String>,
    /// New description (optional).
    pub description: Option<String>,
    /// Whether the workspace is active.
    pub is_active: Option<bool>,
    /// Max documents quota.
    pub max_documents: Option<usize>,
    /// New LLM model for entity extraction (optional).
    /// Takes effect immediately for new document ingestions.
    pub llm_model: Option<String>,
    /// New LLM provider (optional).
    pub llm_provider: Option<String>,
    /// New embedding model (optional).
    /// WARNING: Requires vector rebuild - use rebuild-embeddings endpoint.
    pub embedding_model: Option<String>,
    /// New embedding provider (optional).
    pub embedding_provider: Option<String>,
    /// New embedding dimension (optional).
    pub embedding_dimension: Option<usize>,
}

/// Statistics for a workspace.
///
/// WHY embedding_count: Mission requirement - "Ensure metric likes number of
/// Entities, Relationships, Embeddings per document" are tracked.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceStats {
    /// Workspace ID.
    pub workspace_id: Uuid,
    /// Total documents.
    pub document_count: usize,
    /// Total entities (graph nodes).
    pub entity_count: usize,
    /// Total relationships (graph edges).
    pub relationship_count: usize,
    /// Total chunks (text segments).
    pub chunk_count: usize,
    /// Total embeddings (vector representations).
    pub embedding_count: usize,
    /// Storage used in bytes.
    pub storage_bytes: usize,
}

/// Trigger type for metrics recording.
///
/// WHY enum: Type-safe representation of when metrics were recorded.
/// OODA-20: Aligns with migration 016 trigger_type column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricsTriggerType {
    /// Recorded automatically after document add/delete events.
    Event,
    /// Recorded by background scheduled task (hourly).
    Scheduled,
    /// Recorded by admin request.
    Manual,
}

impl MetricsTriggerType {
    /// Convert to database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::Scheduled => "scheduled",
            Self::Manual => "manual",
        }
    }

    /// Parse from database string representation.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "event" => Some(Self::Event),
            "scheduled" => Some(Self::Scheduled),
            "manual" => Some(Self::Manual),
            _ => None,
        }
    }
}

impl std::fmt::Display for MetricsTriggerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A recorded metrics snapshot for time-series analysis.
///
/// WHY snapshot: Enables trend analysis, debugging, and historical reporting.
/// OODA-20: Corresponds to workspace_metrics_history table from migration 016.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Unique snapshot ID.
    pub id: Uuid,
    /// Workspace this snapshot belongs to.
    pub workspace_id: Uuid,
    /// When the snapshot was recorded.
    pub recorded_at: chrono::DateTime<chrono::Utc>,
    /// What triggered the recording.
    pub trigger_type: MetricsTriggerType,
    /// Number of documents at recording time.
    pub document_count: i64,
    /// Number of chunks at recording time.
    pub chunk_count: i64,
    /// Number of entities at recording time.
    pub entity_count: i64,
    /// Number of relationships at recording time.
    pub relationship_count: i64,
    /// Number of embeddings at recording time.
    pub embedding_count: i64,
    /// Storage bytes at recording time.
    pub storage_bytes: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_creation() {
        let tenant = Tenant::new("Acme Corp", "acme-corp")
            .with_plan(TenantPlan::Pro)
            .with_description("Main production tenant");

        assert_eq!(tenant.name, "Acme Corp");
        assert_eq!(tenant.slug, "acme-corp");
        assert_eq!(tenant.plan, TenantPlan::Pro);
        // SPEC-028: Pro plan now allows 500 workspaces
        assert_eq!(tenant.max_workspaces, 500);
        assert!(tenant.is_active);
    }

    #[test]
    fn test_workspace_creation() {
        let tenant_id = Uuid::new_v4();
        let workspace = Workspace::new(tenant_id, "Knowledge Base", "kb-1")
            .with_description("Primary KB")
            .with_max_documents(5000);

        assert_eq!(workspace.tenant_id, tenant_id);
        assert_eq!(workspace.name, "Knowledge Base");
        assert_eq!(workspace.max_documents(), Some(5000));
    }

    #[test]
    fn test_membership_roles() {
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        let owner = Membership::new(user_id, tenant_id, MembershipRole::Owner);
        let member =
            Membership::new(user_id, tenant_id, MembershipRole::Member).for_workspace(workspace_id);

        assert!(owner.has_role(MembershipRole::Admin));
        assert!(owner.can_access_workspace(&workspace_id));
        assert!(member.can_access_workspace(&workspace_id));
        assert!(!member.can_access_workspace(&Uuid::new_v4()));
    }

    #[test]
    fn test_tenant_context() {
        let ctx = TenantContext::new(Uuid::new_v4())
            .with_workspace(Uuid::new_v4())
            .with_user(Uuid::new_v4(), MembershipRole::Member);

        assert!(ctx.is_valid());
        assert!(ctx.can_write());
    }
}
