//! Projection work-item types shared by the ledger and worker.

use edgequake_storage_contracts::{
    BindingRole, BindingState, DataBindingDescriptor, ProjectionDelivery, ProjectionEvent,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// First schema understood by the SPEC-149 worker.
pub const PROJECTION_SCHEMA_V1: u32 = 1;

/// Provider axis selected by an immutable data binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectionTarget {
    Graph,
    Vector,
}

impl ProjectionTarget {
    pub fn from_binding_role(role: &str) -> Option<Self> {
        BindingRole::parse(role).ok().and_then(|role| {
            if role.is_graph() {
                Some(Self::Graph)
            } else if role.is_vector() {
                Some(Self::Vector)
            } else {
                None
            }
        })
    }

    pub fn from_role(role: BindingRole) -> Option<Self> {
        if role.is_graph() {
            Some(Self::Graph)
        } else if role.is_vector() {
            Some(Self::Vector)
        } else {
            None
        }
    }
}

/// A claimed event and one independently-progressing target delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionWorkItem {
    pub event: ProjectionEvent,
    pub delivery: ProjectionDelivery,
    pub binding: DataBindingDescriptor,
    /// Role-specific digest the applier must reproduce from the event manifest.
    pub expected_completion_proof: [u8; 32],
}

impl ProjectionWorkItem {
    pub fn target(&self) -> Option<ProjectionTarget> {
        ProjectionTarget::from_role(self.binding.role)
    }

    pub fn binding_id(&self) -> Uuid {
        self.binding.binding_id
    }

    pub fn binding_role(&self) -> &str {
        self.binding.role.as_str()
    }

    pub fn binding_generation(&self) -> u64 {
        self.binding.generation
    }

    pub fn require_active_or_draining(
        &self,
    ) -> Result<(), edgequake_storage_contracts::AccessError> {
        if self.binding.state.accepts_cleanup() || self.binding.state == BindingState::Active {
            Ok(())
        } else {
            Err(edgequake_storage_contracts::AccessError::Unavailable(
                format!(
                    "binding {} is {}; refuse projection apply",
                    self.binding.binding_id,
                    self.binding.state.as_str()
                ),
            ))
        }
    }
}

/// Durable provider result written into the fenced acknowledgment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionApplyReceipt {
    pub provider_receipt: String,
    pub completion_proof: Vec<u8>,
}

/// Stable, scope-qualified AGE node identity so same-name entities cannot collide.
pub fn scoped_graph_node_id(tenant_id: Uuid, workspace_id: Uuid, logical_name: &str) -> String {
    format!("{tenant_id}:{workspace_id}:{logical_name}")
}
