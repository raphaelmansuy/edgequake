//! Workspace-scoped vector storage resolution for the library orchestrator.

use std::sync::Arc;

use edgequake_storage::traits::{VectorStorage, WorkspaceVectorRegistry};

use crate::error::{Error, Result};
use crate::workspace_service::WorkspaceService;
use crate::workspace_vector_resolve::{
    resolve_workspace_vector_storage, WorkspaceVectorResolveInput, WorkspaceVectorResolvePolicy,
};

use super::EdgeQuake;

impl EdgeQuake {
    /// Wire per-workspace vector registry + workspace service (production parity with API worker).
    ///
    /// `strict = true` fails ingestion when workspace context cannot be resolved;
    /// `strict = false` falls back to the registry default storage (tests).
    pub fn with_workspace_vector_support(
        mut self,
        registry: Arc<dyn WorkspaceVectorRegistry>,
        workspace_service: Arc<dyn WorkspaceService>,
        strict: bool,
    ) -> Self {
        self.vector_registry = Some(registry);
        self.workspace_service = Some(workspace_service);
        self.strict_workspace_vectors = strict;
        self
    }

    /// Resolve vector storage for [`super::ingestion::EdgeQuake::insert`].
    pub(super) async fn resolve_ingestion_vector_storage(&self) -> Result<Arc<dyn VectorStorage>> {
        let default_storage = self
            .vector_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Vector storage not initialized"))?
            .clone();

        let Some(registry) = &self.vector_registry else {
            return Ok(default_storage);
        };

        let policy = if self.strict_workspace_vectors {
            WorkspaceVectorResolvePolicy::Strict
        } else {
            WorkspaceVectorResolvePolicy::AllowDefaultFallback
        };

        resolve_workspace_vector_storage(
            registry.as_ref(),
            default_storage,
            self.workspace_service.as_deref(),
            self.config.embedding_dim,
            WorkspaceVectorResolveInput::new(self.config.workspace_id.as_deref(), "default"),
            policy,
        )
        .await
    }
}
