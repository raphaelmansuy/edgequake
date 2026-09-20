//! Immutable data-binding descriptors for projection and cleanup.

use crate::error::{AccessError, AccessResult};
use crate::scope::AccessScope;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Axis role recorded on a data binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingRole {
    Graph,
    GraphProjection,
    Vector,
    VectorProjection,
    Embedding,
}

impl BindingRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Graph => "graph",
            Self::GraphProjection => "graph_projection",
            Self::Vector => "vector",
            Self::VectorProjection => "vector_projection",
            Self::Embedding => "embedding",
        }
    }

    pub fn parse(value: &str) -> AccessResult<Self> {
        match value {
            "graph" => Ok(Self::Graph),
            "graph_projection" => Ok(Self::GraphProjection),
            "vector" => Ok(Self::Vector),
            "vector_projection" => Ok(Self::VectorProjection),
            "embedding" => Ok(Self::Embedding),
            other => Err(AccessError::InvalidInput(format!(
                "unknown binding role '{other}'"
            ))),
        }
    }

    pub fn is_graph(self) -> bool {
        matches!(self, Self::Graph | Self::GraphProjection)
    }

    pub fn is_vector(self) -> bool {
        matches!(
            self,
            Self::Vector | Self::VectorProjection | Self::Embedding
        )
    }
}

/// Lifecycle state of a data binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingState {
    Active,
    Draining,
    Retired,
}

impl BindingState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Draining => "draining",
            Self::Retired => "retired",
        }
    }

    pub fn parse(value: &str) -> AccessResult<Self> {
        match value {
            "active" => Ok(Self::Active),
            "draining" => Ok(Self::Draining),
            "retired" => Ok(Self::Retired),
            other => Err(AccessError::InvalidInput(format!(
                "unknown binding state '{other}'"
            ))),
        }
    }

    pub fn accepts_cleanup(self) -> bool {
        matches!(self, Self::Active | Self::Draining)
    }
}

/// Immutable provider binding selected for a scope/role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataBindingDescriptor {
    pub binding_id: Uuid,
    pub scope: AccessScope,
    pub role: BindingRole,
    pub provider: String,
    pub config_ref: String,
    pub layout: String,
    pub physical_index: String,
    pub model_descriptor: Option<String>,
    pub generation: u64,
    pub state: BindingState,
}

impl DataBindingDescriptor {
    pub fn require_graph(&self) -> AccessResult<()> {
        if self.role.is_graph() {
            Ok(())
        } else {
            Err(AccessError::InvalidInput(format!(
                "binding {} is not a graph role",
                self.binding_id
            )))
        }
    }

    pub fn require_vector(&self) -> AccessResult<()> {
        if self.role.is_vector() {
            Ok(())
        } else {
            Err(AccessError::InvalidInput(format!(
                "binding {} is not a vector role",
                self.binding_id
            )))
        }
    }

    pub fn require_provider(&self, expected: &str) -> AccessResult<()> {
        if self.provider == expected {
            Ok(())
        } else {
            Err(AccessError::InvalidInput(format!(
                "binding {} provider '{}' does not match expected '{expected}'",
                self.binding_id, self.provider
            )))
        }
    }
}

/// Roles every P0 (PostgreSQL + AGE + colocated pgvector) scope must have.
pub const P0_REQUIRED_ROLES: &[BindingRole] = &[BindingRole::Graph, BindingRole::Vector];

/// Provision or return the immutable active bindings for a scope.
#[async_trait]
pub trait BindingRegistry: Send + Sync {
    async fn ensure_scope_bindings(
        &self,
        scope: &AccessScope,
        roles: &[BindingRole],
    ) -> AccessResult<Vec<DataBindingDescriptor>>;

    async fn get_binding(&self, binding_id: Uuid) -> AccessResult<Option<DataBindingDescriptor>>;

    async fn list_active(&self, scope: &AccessScope) -> AccessResult<Vec<DataBindingDescriptor>>;
}

/// Fail closed when required active roles are missing.
pub fn require_active_roles(
    bindings: &[DataBindingDescriptor],
    required: &[BindingRole],
) -> AccessResult<()> {
    for role in required {
        let found = bindings
            .iter()
            .any(|binding| binding.role == *role && binding.state == BindingState::Active);
        if !found {
            return Err(AccessError::Unavailable(format!(
                "required active binding role '{}' is missing for the scope",
                role.as_str()
            )));
        }
    }
    if bindings.is_empty() {
        return Err(AccessError::Unavailable(
            "scope has zero data bindings; refuse durable commit without deliveries".into(),
        ));
    }
    Ok(())
}
