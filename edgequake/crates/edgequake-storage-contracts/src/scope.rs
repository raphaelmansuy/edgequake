//! Validated tenant/workspace scope carried by every user-data operation.

use crate::ids::{TenantId, WorkspaceId};
use serde::{Deserialize, Serialize};

/// Scope admitted by the authorization boundary.
///
/// Fields are intentionally private so callers cannot partially initialize a
/// scope or accidentally treat a missing dimension as a wildcard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccessScope {
    tenant: TenantId,
    workspace: WorkspaceId,
}

impl AccessScope {
    pub const fn new(tenant: TenantId, workspace: WorkspaceId) -> Self {
        Self { tenant, workspace }
    }

    pub const fn tenant(&self) -> TenantId {
        self.tenant
    }

    pub const fn workspace(&self) -> WorkspaceId {
        self.workspace
    }
}
