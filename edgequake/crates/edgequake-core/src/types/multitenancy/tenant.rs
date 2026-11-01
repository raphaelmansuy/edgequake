//! Tenant type and subscription plans.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::workspace::Workspace;
use crate::hex_color::canonicalize_hex_color;

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

    /// SPEC-181: when the organization was archived. Archived organizations no longer grant
    /// access to any non-platform caller and stop counting toward an operator's active count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<chrono::DateTime<chrono::Utc>>,

    /// SPEC-181: who archived it, for the audit trail shown in the organizations list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_by: Option<Uuid>,

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

    /// SPEC-109: default reasoning effort seeded into new workspaces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_reasoning_effort: Option<String>,

    /// SPEC-123: default PDF parser choice for workspaces that inherit (`none`).
    /// `None` falls through to env → Vision. Values: vision | edgeparse | auto.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdf_parser_backend: Option<edgequake_pdf::PdfParserBackend>,

    /// Tenant brand color (`#rrggbb`, canonical lowercase). `None` = EdgeQuake default.
    /// Drives the primary accent across the WebUI for every member of this tenant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_color: Option<String>,
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
        // SPEC-181: the quota defaults are the Free plan's, stated once. They used to be
        // hard-coded 5/10 here while `TenantPlan::Free` said 10/3, so a tenant created
        // without an explicit plan got a limit the plan it claimed to be did not have.
        let plan = TenantPlan::Free;

        Self {
            tenant_id: Uuid::new_v4(),
            name: name.into(),
            slug: slug.into(),
            description: None,
            plan,
            max_workspaces: plan.default_max_workspaces(),
            max_users: plan.default_max_users(),
            is_active: true,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
            archived_at: None,
            archived_by: None,
            default_llm_model,
            default_llm_provider,
            default_embedding_model,
            default_embedding_provider,
            default_embedding_dimension,
            default_vision_llm_provider: None,
            default_vision_llm_model: None,
            default_reasoning_effort: None,
            pdf_parser_backend: None,
            primary_color: None,
        }
    }

    /// Set the tenant plan.
    pub fn with_plan(mut self, plan: TenantPlan) -> Self {
        self.plan = plan;
        self.max_workspaces = plan.default_max_workspaces();
        self.max_users = plan.default_max_users();
        self
    }

    /// SPEC-181: override the plan's quota (an operator-raised limit).
    ///
    /// Kept separate from [`Self::with_plan`] so a plan change and a quota grant stay distinct
    /// ideas: the plan picks the default, the operator may raise it, and `max_users == 0` means
    /// unlimited.
    pub fn with_quota(mut self, max_workspaces: usize, max_users: usize) -> Self {
        self.max_workspaces = max_workspaces;
        self.max_users = max_users;
        self
    }

    /// SPEC-181: `true` when the tenant carries unlimited seats (`max_users == 0`).
    pub fn has_unlimited_seats(&self) -> bool {
        self.max_users == 0
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

    /// Heal persisted Mock default LLM/embedding/vision fields in place.
    pub fn sanitize_mock_providers(&mut self) {
        if super::workspace::is_mock_provider_id(&self.default_llm_provider) {
            self.default_llm_provider =
                Workspace::coerce_non_mock_provider(self.default_llm_provider.clone());
            self.default_llm_model =
                Workspace::default_model_for_provider(&self.default_llm_provider);
        }
        if super::workspace::is_mock_provider_id(&self.default_embedding_provider) {
            self.default_embedding_provider =
                Workspace::coerce_non_mock_provider(self.default_embedding_provider.clone());
            self.default_embedding_model =
                Workspace::default_embedding_model_for_provider(&self.default_embedding_provider);
            self.default_embedding_dimension =
                Workspace::detect_dimension_from_model(&self.default_embedding_model);
        }
        if let Some(ref vision_provider) = self.default_vision_llm_provider {
            if super::workspace::is_mock_provider_id(vision_provider) {
                let replacement = Workspace::coerce_non_mock_provider(vision_provider.clone());
                self.default_vision_llm_model =
                    Some(Workspace::default_model_for_provider(&replacement));
                self.default_vision_llm_provider = Some(replacement);
            }
        }
    }
}

/// Parse a user-supplied tenant brand color.
///
/// - `""` or `"none"` (case-insensitive) → `Ok(None)`: clear to the EdgeQuake default.
/// - `#RGB` / `#RRGGBB` → `Ok(Some("#rrggbb"))` (canonical lowercase).
/// - anything else → `Err` (the API maps this to 400).
pub fn parse_tenant_primary_color(raw: &str) -> Result<Option<String>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    canonicalize_hex_color(trimmed).map(Some).ok_or_else(|| {
        format!("Invalid primary_color '{raw}'. Expected #RRGGBB or #RGB, or 'none' to clear")
    })
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
    /// Wire id (`free`, `basic`, `pro`, `enterprise`). This is the `subscription_plans.id`
    /// value and the `tenants.plan` column value, so it must stay lowercase and stable.
    pub fn as_str(&self) -> &'static str {
        match self {
            TenantPlan::Free => "free",
            TenantPlan::Basic => "basic",
            TenantPlan::Pro => "pro",
            TenantPlan::Enterprise => "enterprise",
        }
    }

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
    ///
    /// SPEC-181: `0` means unlimited, matching `subscription_plans.max_users`. Enterprise used
    /// to return `500`, which contradicted the catalog row for the same plan.
    pub fn default_max_users(&self) -> usize {
        match self {
            TenantPlan::Free => 3,
            TenantPlan::Basic => 10,
            TenantPlan::Pro => 50,
            TenantPlan::Enterprise => 0,
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
mod primary_color_tests {
    use super::parse_tenant_primary_color;

    #[test]
    fn empty_and_none_clear_the_color() {
        assert_eq!(parse_tenant_primary_color("").unwrap(), None);
        assert_eq!(parse_tenant_primary_color("   ").unwrap(), None);
        assert_eq!(parse_tenant_primary_color("none").unwrap(), None);
        assert_eq!(parse_tenant_primary_color("NONE").unwrap(), None);
    }

    #[test]
    fn valid_hex_is_canonicalized() {
        assert_eq!(
            parse_tenant_primary_color("#4B5BD6").unwrap().as_deref(),
            Some("#4b5bd6")
        );
        assert_eq!(
            parse_tenant_primary_color(" #0F0 ").unwrap().as_deref(),
            Some("#00ff00")
        );
    }

    #[test]
    fn invalid_values_are_rejected_with_context() {
        for bad in ["blue", "#12", "#gggggg", "rgb(1,2,3)"] {
            let err = parse_tenant_primary_color(bad).unwrap_err();
            assert!(err.contains("Invalid primary_color"), "{bad}: {err}");
        }
    }
}
