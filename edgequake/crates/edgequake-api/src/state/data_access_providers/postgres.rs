//! Built-in factories. All concrete database construction stays in this module.

use super::*;
use edgequake_storage::{
    DimensionEnsureOutcome, DimensionReconcilePolicy, PgIngestionCommitter, PgProjectionLedger,
    PgVectorStorage, PgWorkspaceVectorRegistry, PostgresAGEGraphStorage, PostgresConfig,
    PostgresKVStorage, PostgresPool,
};

#[derive(Clone)]
pub struct PostgresProviderResources {
    pub ingest: PostgresPool,
    pub query: PostgresPool,
    pub admin: sqlx::PgPool,
    pub queue: sqlx::PgPool,
    pub config: PostgresConfig,
}

pub struct PgRelationalProviderFactory(pub PostgresProviderResources);
pub struct PgVectorProviderFactory(pub PostgresProviderResources);
pub struct AgeProviderFactory(pub PostgresProviderResources);

impl ProviderFactories {
    pub fn postgres(resources: PostgresProviderResources) -> Self {
        Self {
            relational: Arc::new(PgRelationalProviderFactory(resources.clone())),
            vector: Arc::new(PgVectorProviderFactory(resources.clone())),
            graph: Arc::new(AgeProviderFactory(resources)),
        }
    }
}

#[async_trait]
impl RelationalProviderFactory for PgRelationalProviderFactory {
    fn provider_name(&self) -> &str {
        "postgres"
    }

    async fn build(
        &self,
        context: &ProviderContext,
    ) -> Result<RelationalProviderRuntime, StorageError> {
        context.validate()?;
        let r = &self.0;
        let pool = r.ingest.get().await?;
        let kv = Arc::new(PostgresKVStorage::with_pool(
            r.ingest.clone(),
            r.config.clone(),
        ));
        let kv_query = Arc::new(PostgresKVStorage::with_pool(
            r.query.clone(),
            r.config.clone(),
        ));
        let posture = edgequake_storage::detect_cutover_posture(&r.admin).await?;
        edgequake_storage::validate_cutover_flags(&posture).map_err(StorageError::InvalidConfig)?;
        kv.seed_relation_from_dropped(posture.kv_store_dropped);
        kv_query.seed_relation_from_dropped(posture.kv_store_dropped);
        let committer = Arc::new(PgIngestionCommitter::new(pool.clone()));
        let workspace = edgequake_core::WorkspaceServiceImpl::new(pool.clone());
        if context.provision_defaults {
            workspace.ensure_defaults().await.map_err(|e| {
                StorageError::Database(format!("provision relational defaults: {e}"))
            })?;
        }
        Ok(RelationalProviderRuntime {
            kv_storage: kv,
            kv_query,
            ingestion_committer: committer.clone(),
            lifecycle_committer: committer.clone(),
            document_reader: committer,
            projection_ledger: Arc::new(PgProjectionLedger::new(pool.clone())),
            operational: OperationalStores {
                identity: Some(Arc::new(crate::services::postgres_identity_store::PostgresIdentityStore::new(pool.clone()))),
                sessions: Some(Arc::new(crate::services::postgres_session_store::PostgresSessionStore::new(pool.clone()))),
                workspaces: Some(Arc::new(crate::services::postgres_workspace_store::PostgresWorkspaceStore::new(pool.clone()))),
                checkpoint_artifacts: Some(Arc::new(crate::services::postgres_checkpoint_artifact_store::PostgresCheckpointArtifactStore::new(r.admin.clone()))),
            },
            workspace_service: Arc::new(workspace),
            conversation_service: Arc::new(edgequake_core::ConversationServiceImpl::new(pool.clone())),
            task_storage: Arc::new(edgequake_tasks::postgres::PostgresTaskStorage::new(r.queue.clone())),
            federation_store: Arc::new(crate::services::federation::pg_store::PgFederationStore::new(pool.clone())),
            decision_store: Arc::new(edgequake_storage::adapters::postgres::decision_store::PostgresDecisionStore::new(pool.clone())),
            documents: DocumentStores {
                pdf: Arc::new(edgequake_storage::PostgresPdfStorage::new(pool.clone())),
                originals: Arc::new(edgequake_storage::PostgresOriginalStorage::new(pool.clone())),
                multimodal_assets: Arc::new(edgequake_storage::PostgresMmAssetStorage::new(pool.clone())),
                page_layouts: Arc::new(edgequake_storage::PostgresPageLayoutStorage::new(pool.clone())),
                page_states: Arc::new(edgequake_storage::PostgresPageStateStorage::new(pool)),
            },
        })
    }
}

#[async_trait]
impl VectorProviderFactory for PgVectorProviderFactory {
    fn provider_name(&self) -> &str {
        "pgvector"
    }

    async fn build(
        &self,
        context: &ProviderContext,
    ) -> Result<VectorProviderRuntime, StorageError> {
        context.validate()?;
        let r = &self.0;
        let provisional = PgVectorStorage::with_pool_and_dimension(
            r.ingest.clone(),
            r.config.clone(),
            context.embedding_dimension,
        );
        let outcome = provisional
            .reconcile_dimension(
                context.embedding_dimension,
                DimensionReconcilePolicy::PreferExisting,
            )
            .await?;
        let (storage, recreated_default_vector): (Arc<dyn VectorStorage>, bool) = match outcome {
            DimensionEnsureOutcome::Matched => (Arc::new(provisional), false),
            DimensionEnsureOutcome::Recreated => (Arc::new(provisional), true),
            DimensionEnsureOutcome::KeptExisting { stored, required } => {
                tracing::warn!(
                    stored_dimension = stored,
                    provider_dimension = required,
                    "Default vector table kept at stored dimension (PreferExisting)"
                );
                (
                    Arc::new(PgVectorStorage::with_pool_and_dimension(
                        r.ingest.clone(),
                        r.config.clone(),
                        stored,
                    )),
                    false,
                )
            }
        };
        let query = Arc::new(PgVectorStorage::with_pool_and_dimension(
            r.query.clone(),
            r.config.clone(),
            storage.dimension(),
        ));
        let registry = Arc::new(PgWorkspaceVectorRegistry::new(
            r.config.clone(),
            r.ingest.clone(),
            Arc::clone(&storage),
            context.embedding_dimension,
        ));
        let chunk = Arc::new(edgequake_storage::PgChunkEmbeddingIndex::new(
            r.admin.clone(),
            &context.embedding_model,
        ));
        let fleet = Arc::new(edgequake_storage::PgFleetEmbeddingIndex::new(
            r.admin.clone(),
            &context.embedding_model,
        ));
        Ok(VectorProviderRuntime {
            storage,
            query,
            registry,
            recreated_default_vector,
            projection_applier: Arc::new(edgequake_storage::PgvectorProjectionApplier::new(
                chunk,
                Some(fleet),
                r.admin.clone(),
            )),
        })
    }
}

#[async_trait]
impl GraphProviderFactory for AgeProviderFactory {
    fn provider_name(&self) -> &str {
        "age"
    }

    async fn build(&self, context: &ProviderContext) -> Result<GraphProviderRuntime, StorageError> {
        context.validate()?;
        let r = &self.0;
        let storage: Arc<dyn GraphStorage> = Arc::new(PostgresAGEGraphStorage::with_pool(
            r.ingest.clone(),
            r.config.clone(),
        ));
        let query = Arc::new(PostgresAGEGraphStorage::with_pool(
            r.query.clone(),
            r.config.clone(),
        ));
        let projection_applier = Arc::new(edgequake_storage::AgeGraphProjectionApplier::new(
            Arc::clone(&storage),
            r.admin.clone(),
        ));
        Ok(GraphProviderRuntime {
            storage,
            query,
            projection_applier,
        })
    }
}
