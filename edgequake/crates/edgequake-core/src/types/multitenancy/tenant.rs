//! Tenant type and subscription plans.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::workspace::Workspace;

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
    /// URL-safe slug (unique).
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

    // === Model Configuration (SPEC-032) ===
    /// Default LLM model for new workspaces (e.g., "gemma3:12b", "gpt-4o-mini").
    pub default_llm_model: String,
    /// Default LLM provider for new workspaces (e.g., "ollama", "openai").
    pub default_llm_provider: String,
    /// Default embedding model for new workspaces (e.g., "text-embedding-3-small").
    pub default_embedding_model: String,
    /// Default embedding provider for new workspaces (e.g., "openai", "ollama").
    pub default_embedding_provider: String,
    /// Default embedding dimension for new workspaces (e.g., 1536, 768).
    pub default_embedding_dimension: usize,

    // === Vision LLM Configuration (SPEC-040) ===
    /// Default Vision LLM provider for PDF-to-Markdown extraction.
    /// Workspaces inherit this if not explicitly configured.
    /// Falls back automatically when workspace has no vision LLM set.
    pub default_vision_llm_provider: Option<String>,
    /// Default Vision LLM model for PDF-to-Markdown extraction.
    /// Workspaces inherit this if not explicitly configured.
    /// Falls back automatically when workspace has no vision LLM set.
    pub default_vision_llm_model: Option<String>,
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
            default_vision_llm_provider: None,
            default_vision_llm_model: None,
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

    /// Set the default Vision LLM configuration for new workspaces.
    ///
    /// Used as fallback when a workspace has no vision LLM configured.
    /// Applied automatically during PDF-to-Markdown extraction (SPEC-041).
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_core::Tenant;
    ///
    /// let tenant = Tenant::new("Acme Corp", "acme")
    ///     .with_vision_config("gpt-4o", "openai");
    /// assert_eq!(tenant.default_vision_llm_model, Some("gpt-4o".to_string()));
    /// assert_eq!(tenant.default_vision_llm_provider, Some("openai".to_string()));
    /// ```
    pub fn with_vision_config(
        mut self,
        model: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        self.default_vision_llm_model = Some(model.into());
        self.default_vision_llm_provider = Some(provider.into());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    // --- TenantPlan ---

    #[test]
    fn test_tenant_plan_default_is_free() {
        assert_eq!(TenantPlan::default(), TenantPlan::Free);
    }

    #[test]
    fn test_tenant_plan_max_workspaces() {
        assert_eq!(TenantPlan::Free.default_max_workspaces(), 10);
        assert_eq!(TenantPlan::Basic.default_max_workspaces(), 100);
        assert_eq!(TenantPlan::Pro.default_max_workspaces(), 500);
        assert_eq!(TenantPlan::Enterprise.default_max_workspaces(), 500);
    }

    #[test]
    fn test_tenant_plan_max_users() {
        assert_eq!(TenantPlan::Free.default_max_users(), 3);
        assert_eq!(TenantPlan::Basic.default_max_users(), 10);
        assert_eq!(TenantPlan::Pro.default_max_users(), 50);
        assert_eq!(TenantPlan::Enterprise.default_max_users(), 500);
    }

    #[test]
    fn test_tenant_plan_max_documents() {
        assert_eq!(TenantPlan::Free.default_max_documents(), 100);
        assert_eq!(TenantPlan::Basic.default_max_documents(), 1000);
        assert_eq!(TenantPlan::Pro.default_max_documents(), 10000);
        assert_eq!(TenantPlan::Enterprise.default_max_documents(), 100000);
    }

    #[test]
    fn test_tenant_plan_display_roundtrip() {
        for (plan, label) in [
            (TenantPlan::Free, "free"),
            (TenantPlan::Basic, "basic"),
            (TenantPlan::Pro, "pro"),
            (TenantPlan::Enterprise, "enterprise"),
        ] {
            assert_eq!(plan.to_string(), label);
            assert_eq!(TenantPlan::from_str(label).unwrap(), plan);
        }
    }

    #[test]
    fn test_tenant_plan_from_str_case_insensitive() {
        assert_eq!(TenantPlan::from_str("FREE").unwrap(), TenantPlan::Free);
        assert_eq!(TenantPlan::from_str("Pro").unwrap(), TenantPlan::Pro);
    }

    #[test]
    fn test_tenant_plan_from_str_unknown() {
        let err = TenantPlan::from_str("premium").unwrap_err();
        assert!(err.contains("Unknown plan"));
    }

    // --- Tenant ---

    #[test]
    fn test_tenant_new_defaults() {
        let t = Tenant::new("Acme Corp", "acme");
        assert_eq!(t.name, "Acme Corp");
        assert_eq!(t.slug, "acme");
        assert_eq!(t.plan, TenantPlan::Free);
        assert_eq!(t.max_workspaces, 5);
        assert_eq!(t.max_users, 10);
        assert!(t.is_active);
        assert!(t.description.is_none());
        assert!(t.metadata.is_empty());
        assert!(t.default_vision_llm_provider.is_none());
        assert!(t.default_vision_llm_model.is_none());
    }

    #[test]
    fn test_tenant_with_plan_updates_limits() {
        let t = Tenant::new("X", "x").with_plan(TenantPlan::Pro);
        assert_eq!(t.plan, TenantPlan::Pro);
        // WHY: with_plan overrides max_workspaces/max_users from plan defaults
        assert_eq!(t.max_workspaces, 500);
        assert_eq!(t.max_users, 50);
    }

    #[test]
    fn test_tenant_with_description() {
        let t = Tenant::new("X", "x").with_description("A test tenant");
        assert_eq!(t.description.as_deref(), Some("A test tenant"));
    }

    #[test]
    fn test_tenant_with_llm_config() {
        let t = Tenant::new("X", "x").with_llm_config("gemma3:12b", "ollama");
        assert_eq!(t.default_llm_model, "gemma3:12b");
        assert_eq!(t.default_llm_provider, "ollama");
    }

    #[test]
    fn test_tenant_with_embedding_config() {
        let t = Tenant::new("X", "x")
            .with_embedding_config("text-embedding-3-small", "openai", 1536);
        assert_eq!(t.default_embedding_model, "text-embedding-3-small");
        assert_eq!(t.default_embedding_provider, "openai");
        assert_eq!(t.default_embedding_dimension, 1536);
    }

    #[test]
    fn test_tenant_with_vision_config() {
        let t = Tenant::new("X", "x").with_vision_config("gpt-4o", "openai");
        assert_eq!(t.default_vision_llm_model.as_deref(), Some("gpt-4o"));
        assert_eq!(t.default_vision_llm_provider.as_deref(), Some("openai"));
    }

    #[test]
    fn test_tenant_builder_chain() {
        let t = Tenant::new("Big Co", "big-co")
            .with_plan(TenantPlan::Enterprise)
            .with_description("Enterprise tenant")
            .with_llm_config("gpt-5-nano", "openai")
            .with_embedding_config("nomic-embed", "ollama", 768)
            .with_vision_config("gpt-4o", "openai");
        assert_eq!(t.plan, TenantPlan::Enterprise);
        assert_eq!(t.max_workspaces, 500);
        assert_eq!(t.max_users, 500);
        assert_eq!(t.default_llm_model, "gpt-5-nano");
        assert_eq!(t.default_embedding_dimension, 768);
        assert!(t.default_vision_llm_model.is_some());
    }
}
