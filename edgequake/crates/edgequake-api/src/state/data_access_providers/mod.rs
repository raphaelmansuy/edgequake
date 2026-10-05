//! Three independently replaceable provider factories. Implementations own
//! their clients; this composition contract exposes domain ports, never drivers.
//!
//! A graph provider may share the relational database (AGE by default). A
//! dedicated graph database is an optional deployment choice, not a requirement.
//!
//! Implement one factory and the existing ports for that axis, then pass it in
//! [`ProviderOverrides`] to [`super::AppState::new_postgres_with_providers`].
//! Factories own credentials/clients and must return coherent read, write and
//! projection handles. With PostgreSQL authority, provision scope bindings via
//! `BindingRegistry` (or your control plane) before ingestion; each descriptor's
//! provider must match its factory/applier name. Existing bindings are preserved.
//! A ledger shared by different deployments must implement provider-filtered
//! claims; PostgreSQL does so before acquiring delivery leases.
//!
//! This replaces DAL services; PostgreSQL platform infrastructure (migration
//! ledger, audit, budgets and remaining SQL helper routes) is still required.
//! Environment-selected SQLite/Qdrant/Neo4j profiles keep their certification gate.
//! Factory failure aborts startup; partial builds must clean up their own resources.

#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "postgres")]
pub use postgres::{
    AgeProviderFactory, PgRelationalProviderFactory, PgVectorProviderFactory,
    PostgresProviderResources,
};

use std::sync::Arc;

use async_trait::async_trait;
use edgequake_storage::{
    contracts::{DocumentReader, IngestionCommitter, LifecycleCommitter},
    traits::{GraphStorage, KVStorage, VectorStorage, WorkspaceVectorRegistry},
    GraphProjectionApplier, ProjectionWorkLedger, StorageError, VectorProjectionApplier,
};

use super::{OperationalStores, SharedConversationService, SharedWorkspaceService};

/// Driver-free settings shared by factories; credentials remain implementation-owned.
#[derive(Debug, Clone)]
pub struct ProviderContext {
    pub namespace: String,
    pub embedding_dimension: usize,
    pub embedding_model: String,
    pub provision_defaults: bool,
}

impl ProviderContext {
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.namespace.trim().is_empty()
            || self.embedding_model.trim().is_empty()
            || self.embedding_dimension == 0
        {
            return Err(StorageError::InvalidConfig(
                "provider context requires namespace, model and positive dimensions".into(),
            ));
        }
        Ok(())
    }
}

/// Common relational services. Atomic ingestion and its delivery ledger must
/// use the same authority; graph/vector services never participate in that transaction.
pub struct RelationalProviderRuntime {
    pub kv_storage: Arc<dyn KVStorage>,
    pub kv_query: Arc<dyn KVStorage>,
    pub ingestion_committer: Arc<dyn IngestionCommitter>,
    pub lifecycle_committer: Arc<dyn LifecycleCommitter>,
    pub document_reader: Arc<dyn DocumentReader>,
    pub projection_ledger: Arc<dyn ProjectionWorkLedger>,
    pub operational: OperationalStores,
    pub workspace_service: SharedWorkspaceService,
    pub conversation_service: SharedConversationService,
    pub task_storage: edgequake_tasks::SharedTaskStorage,
    pub federation_store: crate::services::federation::SharedFederationStore,
    pub decision_store: Arc<dyn edgequake_storage::decision::DecisionStore>,
    pub documents: DocumentStores,
}

/// Binary documents and their relational metadata use small existing ports.
pub struct DocumentStores {
    pub pdf: Arc<dyn edgequake_storage::PdfDocumentStorage>,
    pub originals: Arc<dyn edgequake_storage::DocumentOriginalStorage>,
    pub multimodal_assets: Arc<dyn edgequake_storage::DocumentMmAssetStorage>,
    pub page_layouts: Arc<dyn edgequake_storage::DocumentPageLayoutStorage>,
    pub page_states: Arc<dyn edgequake_storage::PageStateStorage>,
}

/// Query, workspace isolation and durable delivery belong to one vector provider.
pub struct VectorProviderRuntime {
    pub storage: Arc<dyn VectorStorage>,
    pub query: Arc<dyn VectorStorage>,
    pub registry: Arc<dyn WorkspaceVectorRegistry>,
    pub projection_applier: Arc<dyn VectorProjectionApplier>,
    pub recreated_default_vector: bool,
}

/// Graph read/write handles and projection delivery come from the same provider.
pub struct GraphProviderRuntime {
    pub storage: Arc<dyn GraphStorage>,
    pub query: Arc<dyn GraphStorage>,
    pub projection_applier: Arc<dyn GraphProjectionApplier>,
}

#[async_trait]
pub trait RelationalProviderFactory: Send + Sync {
    fn provider_name(&self) -> &str;
    async fn build(
        &self,
        context: &ProviderContext,
    ) -> Result<RelationalProviderRuntime, StorageError>;
}

#[async_trait]
pub trait VectorProviderFactory: Send + Sync {
    fn provider_name(&self) -> &str;
    async fn build(&self, context: &ProviderContext)
        -> Result<VectorProviderRuntime, StorageError>;
}

#[async_trait]
pub trait GraphProviderFactory: Send + Sync {
    fn provider_name(&self) -> &str;
    async fn build(&self, context: &ProviderContext) -> Result<GraphProviderRuntime, StorageError>;
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ProviderNames {
    pub relational: String,
    pub vector: String,
    pub graph: String,
}

pub struct MaterializedProviders {
    pub names: ProviderNames,
    pub relational: RelationalProviderRuntime,
    pub vector: VectorProviderRuntime,
    pub graph: GraphProviderRuntime,
}

/// Explicit factories allow new implementations without adding enum branches.
pub struct ProviderFactories {
    pub relational: Arc<dyn RelationalProviderFactory>,
    pub vector: Arc<dyn VectorProviderFactory>,
    pub graph: Arc<dyn GraphProviderFactory>,
}

/// Replace any axis while retaining the built-in providers for the other two.
#[derive(Default)]
pub struct ProviderOverrides {
    pub relational: Option<Arc<dyn RelationalProviderFactory>>,
    pub vector: Option<Arc<dyn VectorProviderFactory>>,
    pub graph: Option<Arc<dyn GraphProviderFactory>>,
}

impl ProviderFactories {
    pub fn with_overrides(mut self, overrides: ProviderOverrides) -> Self {
        if let Some(factory) = overrides.relational {
            self.relational = factory;
        }
        if let Some(factory) = overrides.vector {
            self.vector = factory;
        }
        if let Some(factory) = overrides.graph {
            self.graph = factory;
        }
        self
    }

    pub async fn materialize(
        &self,
        context: &ProviderContext,
    ) -> Result<MaterializedProviders, StorageError> {
        context.validate()?;
        let names = ProviderNames {
            relational: checked_provider_name(self.relational.provider_name())?,
            vector: checked_provider_name(self.vector.provider_name())?,
            graph: checked_provider_name(self.graph.provider_name())?,
        };
        let relational = self.relational.build(context).await?;
        if !relational.operational.required_ports_present() {
            return Err(StorageError::InvalidConfig("relational provider requires identity, session, workspace and checkpoint/artifact ports".into()));
        }
        let vector = self.vector.build(context).await?;
        if vector.storage.dimension() == 0
            || vector.storage.dimension() != vector.query.dimension()
            || vector.storage.dimension() != vector.registry.default_storage().dimension()
            || vector.storage.namespace() != vector.query.namespace()
            || vector.storage.namespace() != vector.registry.default_storage().namespace()
        {
            return Err(StorageError::InvalidConfig(
                "vector provider read/write/registry dimensions and namespaces must agree".into(),
            ));
        }
        if vector.projection_applier.provider_name() != names.vector {
            return Err(StorageError::InvalidConfig(
                "vector projection applier does not match selected provider".into(),
            ));
        }
        let graph = self.graph.build(context).await?;
        if graph.storage.namespace() != graph.query.namespace() {
            return Err(StorageError::InvalidConfig(
                "graph provider read/write namespaces must agree".into(),
            ));
        }
        if graph.projection_applier.provider_name() != names.graph {
            return Err(StorageError::InvalidConfig(
                "graph projection applier does not match selected provider".into(),
            ));
        }
        Ok(MaterializedProviders {
            names,
            relational,
            vector,
            graph,
        })
    }
}

fn checked_provider_name(name: &str) -> Result<String, StorageError> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_' || c == b'-')
    {
        return Err(StorageError::InvalidConfig(
            "provider name must be a nonempty lowercase identifier of at most 64 bytes".into(),
        ));
    }
    Ok(name.to_owned())
}

#[cfg(test)]
mod tests;
