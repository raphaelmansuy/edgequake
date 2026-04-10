//! Database row types and conversions for the workspace service.
//!
//! WHY separate module: These types change when the PostgreSQL schema changes,
//! which is a different trigger than workspace service logic changes (SRP).

#[cfg(feature = "postgres")]
use std::collections::HashMap;
#[cfg(feature = "postgres")]
use uuid::Uuid;

#[cfg(feature = "postgres")]
use crate::types::{Membership, Tenant, Workspace};

// Pure parsing helpers live in workspace_utils.rs (always compiled, testable)
use crate::workspace_utils::{parse_plan, parse_role};

// ============ Database Row Types ============

/// Tenant row from PostgreSQL.
/// The actual schema uses metadata JSONB for plan, max_workspaces, max_users, description.
#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
pub(crate) struct TenantRow {
    pub tenant_id: Uuid,
    pub name: String,
    pub slug: Option<String>,
    pub is_active: bool,
    pub metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "postgres")]
impl TenantRow {
    pub fn into_tenant(self) -> Tenant {
        // Extract values from metadata JSONB
        let plan_str = self
            .metadata
            .get("plan")
            .and_then(|v| v.as_str())
            .unwrap_or("free");
        let max_workspaces = self
            .metadata
            .get("max_workspaces")
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as usize;
        let max_users = self
            .metadata
            .get("max_users")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;
        let description = self
            .metadata
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // SPEC-032: Extract default LLM config from metadata.
        // WHY: Use env-aware defaults (same as Workspace::default_llm_config)
        // so Docker deployments with EDGEQUAKE_LLM_PROVIDER=openai propagate
        // correctly to new workspaces created under this tenant.
        let (env_llm_model, env_llm_provider) = Workspace::default_llm_config();
        let default_llm_model = self
            .metadata
            .get("default_llm_model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or(env_llm_model);
        let default_llm_provider = self
            .metadata
            .get("default_llm_provider")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or(env_llm_provider);

        // SPEC-032: Extract default embedding config from metadata.
        let (env_emb_model, env_emb_provider, env_emb_dim) = Workspace::default_embedding_config();
        let default_embedding_model = self
            .metadata
            .get("default_embedding_model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or(env_emb_model);
        let default_embedding_provider = self
            .metadata
            .get("default_embedding_provider")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or(env_emb_provider);
        let default_embedding_dimension = self
            .metadata
            .get("default_embedding_dimension")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(env_emb_dim);

        // SPEC-041: Extract default vision LLM config from metadata
        let default_vision_llm_provider = self
            .metadata
            .get("default_vision_llm_provider")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let default_vision_llm_model = self
            .metadata
            .get("default_vision_llm_model")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        Tenant {
            tenant_id: self.tenant_id,
            name: self.name,
            slug: self.slug.unwrap_or_default(),
            description,
            plan: parse_plan(plan_str),
            is_active: self.is_active,
            max_workspaces,
            max_users,
            created_at: self.created_at,
            updated_at: self.updated_at,
            metadata: HashMap::new(),
            default_llm_model,
            default_llm_provider,
            default_embedding_model,
            default_embedding_provider,
            default_embedding_dimension,
            default_vision_llm_provider,
            default_vision_llm_model,
        }
    }
}

/// Workspace row from PostgreSQL.
#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
pub(crate) struct WorkspaceRow {
    pub workspace_id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
    pub metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "postgres")]
impl WorkspaceRow {
    pub fn into_workspace(self) -> Workspace {
        // Convert metadata from serde_json::Value to HashMap
        let metadata: HashMap<String, serde_json::Value> =
            if let serde_json::Value::Object(map) = self.metadata {
                map.into_iter().collect()
            } else {
                HashMap::new()
            };

        // SPEC-032: Extract LLM config from metadata.
        // WHY: When the workspace has no LLM config in metadata (empty `{}`),
        // we must fall back to env-aware defaults (Workspace::default_llm_config)
        // instead of hardcoded Ollama constants. This ensures Docker/Portainer
        // deployments that set EDGEQUAKE_LLM_PROVIDER=openai get OpenAI for
        // entity extraction, not a broken Ollama fallback.
        let (env_llm_model, env_llm_provider) = Workspace::default_llm_config();
        let llm_model = metadata
            .get("llm_model")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty()) // WHY: empty string stored from Docker ${VAR:-} must not override env default
            .map(|s| s.to_string())
            .unwrap_or(env_llm_model);
        let llm_provider = metadata
            .get("llm_provider")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty()) // WHY: same empty-string guard as llm_model
            .map(|s| s.to_string())
            .unwrap_or(env_llm_provider);

        // SPEC-032: Extract embedding config from metadata.
        // Same env-aware fallback as LLM config above.
        let (env_emb_model, env_emb_provider, env_emb_dim) = Workspace::default_embedding_config();
        let embedding_model = metadata
            .get("embedding_model")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty()) // WHY: empty string from Docker ${VAR:-} must not override env default
            .map(|s| s.to_string())
            .unwrap_or(env_emb_model);
        let embedding_provider = metadata
            .get("embedding_provider")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty()) // WHY: same empty-string guard — prevents "Unknown embedding provider: ''"
            .map(|s| s.to_string())
            .unwrap_or(env_emb_provider);
        let embedding_dimension = metadata
            .get("embedding_dimension")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(env_emb_dim);

        // SPEC-040: Extract vision LLM config from metadata
        let vision_llm_provider = metadata
            .get("vision_llm_provider")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let vision_llm_model = metadata
            .get("vision_llm_model")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        Workspace {
            workspace_id: self.workspace_id,
            tenant_id: self.tenant_id,
            name: self.name,
            slug: self.slug.unwrap_or_default(),
            description: self.description,
            is_active: self.is_active,
            created_at: self.created_at,
            updated_at: self.updated_at,
            metadata,
            llm_model,
            llm_provider,
            embedding_model,
            embedding_provider,
            embedding_dimension,
            vision_llm_provider,
            vision_llm_model,
        }
    }
}

/// Membership row from PostgreSQL.
#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
pub(crate) struct MembershipRow {
    pub membership_id: Uuid,
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub user_id: Uuid,
    pub role: String,
    pub is_active: bool,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "postgres")]
impl MembershipRow {
    pub fn into_membership(self) -> Membership {
        Membership {
            membership_id: self.membership_id,
            tenant_id: self.tenant_id,
            workspace_id: self.workspace_id,
            user_id: self.user_id,
            role: parse_role(&self.role),
            is_active: self.is_active,
            joined_at: self.joined_at,
            metadata: HashMap::new(),
        }
    }
}
// Tests for pure parsing functions live in workspace_utils.rs (always compiled)
