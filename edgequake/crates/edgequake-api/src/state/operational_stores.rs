//! Injectable operational relational ports.

use std::sync::Arc;

use edgequake_storage::contracts::{
    CheckpointArtifactStore, IdentityStore, SessionStore, WorkspaceStore,
};

/// Optional provider-independent stores used during the incremental J21
/// extraction. Missing required ports are treated as unsupported by P3 boot.
#[derive(Clone, Default)]
pub struct OperationalStores {
    pub identity: Option<Arc<dyn IdentityStore>>,
    pub sessions: Option<Arc<dyn SessionStore>>,
    pub workspaces: Option<Arc<dyn WorkspaceStore>>,
    pub checkpoint_artifacts: Option<Arc<dyn CheckpointArtifactStore>>,
}

impl OperationalStores {
    pub fn required_p3_ports_present(&self) -> bool {
        self.identity.is_some()
            && self.sessions.is_some()
            && self.workspaces.is_some()
            && self.checkpoint_artifacts.is_some()
    }
}
