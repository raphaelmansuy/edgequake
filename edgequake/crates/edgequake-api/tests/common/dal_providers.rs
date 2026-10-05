//! External implementations used to certify the public composition extension points.
use async_trait::async_trait;
use edgequake_api::state::data_access_providers::*;
use edgequake_core::InMemoryWorkspaceService;
use edgequake_storage::projection::payload::ProjectionApplyReceipt;
use edgequake_storage::{
    contracts::{AccessResult, DataBindingDescriptor, ProjectionEvent},
    traits::domain::{
        EmbeddingCapabilities, EmbeddingIndex, EmbeddingRow, ModelId, ScoredChunk, UpsertReport,
        VectorQuery, WorkspaceId,
    },
    traits::{GraphStorage, VectorStorage, WorkspaceVectorConfig, WorkspaceVectorRegistry},
    AgeGraphProjectionApplier, GraphProjectionApplier, MemoryGraphStorage, MemoryVectorStorage,
    MemoryWorkspaceVectorRegistry, PgvectorProjectionApplier, StorageError,
    VectorProjectionApplier,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

pub struct TestRelational {
    pub base: Arc<dyn RelationalProviderFactory>,
    pub workspaces: Arc<InMemoryWorkspaceService>,
    pub builds: Arc<AtomicUsize>,
    pub omit_identity: bool,
}
#[async_trait]
impl RelationalProviderFactory for TestRelational {
    fn provider_name(&self) -> &str {
        "test_relational"
    }
    async fn build(&self, c: &ProviderContext) -> Result<RelationalProviderRuntime, StorageError> {
        self.builds.fetch_add(1, Ordering::SeqCst);
        let mut runtime = self.base.build(c).await?;
        runtime.workspace_service = self.workspaces.clone();
        if self.omit_identity {
            runtime.operational.identity = None;
        }
        Ok(runtime)
    }
}

pub struct TestGraph {
    pub pool: sqlx::PgPool,
    pub storage: Arc<dyn GraphStorage>,
    pub applies: Arc<AtomicUsize>,
    pub mismatched_namespace: bool,
}
#[async_trait]
impl GraphProviderFactory for TestGraph {
    fn provider_name(&self) -> &str {
        "test_graph"
    }
    async fn build(&self, _: &ProviderContext) -> Result<GraphProviderRuntime, StorageError> {
        Ok(GraphProviderRuntime {
            storage: self.storage.clone(),
            query: if self.mismatched_namespace {
                Arc::new(MemoryGraphStorage::new("incorrect"))
            } else {
                self.storage.clone()
            },
            projection_applier: Arc::new(TestGraphApplier {
                inner: AgeGraphProjectionApplier::new(self.storage.clone(), self.pool.clone()),
                calls: self.applies.clone(),
            }),
        })
    }
}
struct TestGraphApplier {
    inner: AgeGraphProjectionApplier,
    calls: Arc<AtomicUsize>,
}
#[async_trait]
impl GraphProjectionApplier for TestGraphApplier {
    fn provider_name(&self) -> &str {
        "test_graph"
    }
    async fn apply_batch(
        &self,
        items: &[(&ProjectionEvent, &DataBindingDescriptor)],
    ) -> AccessResult<Vec<ProjectionApplyReceipt>> {
        self.calls.fetch_add(items.len(), Ordering::SeqCst);
        // Reuse PostgreSQL manifest hydration; actual graph writes use MemoryGraphStorage.
        let bindings: Vec<_> = items
            .iter()
            .map(|(_, b)| {
                let mut b = (*b).clone();
                b.provider = "age".into();
                b
            })
            .collect();
        let translated: Vec<_> = items
            .iter()
            .zip(&bindings)
            .map(|((e, _), b)| (*e, b))
            .collect();
        let mut receipts = self.inner.apply_batch(&translated).await?;
        for receipt in &mut receipts {
            receipt.provider_receipt = format!("test_graph:{}", receipt.provider_receipt);
        }
        Ok(receipts)
    }
}

pub struct TestVector {
    pub pool: sqlx::PgPool,
    pub storage: Arc<dyn VectorStorage>,
    pub registry: Arc<dyn WorkspaceVectorRegistry>,
    pub applies: Arc<AtomicUsize>,
    pub mismatched_dimensions: bool,
    pub mismatched_applier: bool,
}
impl TestVector {
    pub fn new(pool: sqlx::PgPool) -> Self {
        let storage: Arc<dyn VectorStorage> = Arc::new(MemoryVectorStorage::new("default", 1536));
        Self {
            pool,
            registry: Arc::new(MemoryWorkspaceVectorRegistry::new(storage.clone())),
            storage,
            applies: Arc::new(AtomicUsize::new(0)),
            mismatched_dimensions: false,
            mismatched_applier: false,
        }
    }
}
#[async_trait]
impl VectorProviderFactory for TestVector {
    fn provider_name(&self) -> &str {
        "test_vector"
    }
    async fn build(&self, _: &ProviderContext) -> Result<VectorProviderRuntime, StorageError> {
        Ok(VectorProviderRuntime {
            storage: self.storage.clone(),
            query: if self.mismatched_dimensions {
                Arc::new(MemoryVectorStorage::new("default", 768))
            } else {
                self.storage.clone()
            },
            registry: self.registry.clone(),
            recreated_default_vector: false,
            projection_applier: Arc::new(TestVectorApplier {
                name: if self.mismatched_applier {
                    "incorrect"
                } else {
                    "test_vector"
                },
                inner: PgvectorProjectionApplier::new(
                    Arc::new(TestEmbeddingIndex {
                        pool: self.pool.clone(),
                        registry: self.registry.clone(),
                    }),
                    None,
                    self.pool.clone(),
                ),
                calls: self.applies.clone(),
            }),
        })
    }
}
struct TestVectorApplier {
    name: &'static str,
    inner: PgvectorProjectionApplier,
    calls: Arc<AtomicUsize>,
}
#[async_trait]
impl VectorProjectionApplier for TestVectorApplier {
    fn provider_name(&self) -> &str {
        self.name
    }
    async fn apply_batch(
        &self,
        items: &[(&ProjectionEvent, &DataBindingDescriptor)],
    ) -> AccessResult<Vec<ProjectionApplyReceipt>> {
        if items.iter().any(|(e, _)| {
            e.operation != edgequake_storage::contracts::ProjectionOperation::Upsert
                || e.object_kind != "document_batch"
        }) {
            return Err(
                edgequake_storage::contracts::AccessError::UnsupportedCapability(
                    "fixture certifies chunk upserts only".into(),
                ),
            );
        }
        self.calls.fetch_add(items.len(), Ordering::SeqCst);
        let bindings: Vec<_> = items
            .iter()
            .map(|(_, b)| {
                let mut b = (*b).clone();
                b.provider = "pgvector".into();
                b
            })
            .collect();
        let translated: Vec<_> = items
            .iter()
            .zip(&bindings)
            .map(|((e, _), b)| (*e, b))
            .collect();
        let mut receipts = self.inner.apply_batch(&translated).await?;
        for receipt in &mut receipts {
            receipt.provider_receipt = format!("test_vector:{}", receipt.provider_receipt);
        }
        Ok(receipts)
    }
}

/// Fixture supports chunk upserts only and rejects unsupported operations explicitly.
struct TestEmbeddingIndex {
    pool: sqlx::PgPool,
    registry: Arc<dyn WorkspaceVectorRegistry>,
}
#[async_trait]
impl EmbeddingIndex for TestEmbeddingIndex {
    fn capabilities(&self) -> EmbeddingCapabilities {
        EmbeddingCapabilities {
            metric: "cosine",
            supports_filters: true,
            supports_rerank: false,
        }
    }
    async fn upsert_batch(
        &self,
        _: ModelId,
        rows: &[EmbeddingRow],
    ) -> Result<UpsertReport, StorageError> {
        for row in rows {
            let ws = row.workspace_id.into_uuid();
            let (doc, tenant, content, index): (uuid::Uuid, uuid::Uuid, String, i32) = sqlx::query_as(
                "SELECT document_id, tenant_id, content, chunk_index FROM chunks WHERE id = $1 AND workspace_id = $2"
            ).bind(row.chunk_id.0).bind(ws).fetch_one(&self.pool).await
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let storage = self
                .registry
                .get_or_create(WorkspaceVectorConfig::new(ws, row.embedding.len()))
                .await?;
            storage
                .upsert(&[(
                    format!("{doc}-chunk-{index}"),
                    row.embedding.clone(),
                    serde_json::json!({
                        "type": "chunk", "tenant_id": tenant, "workspace_id": ws,
                        "document_id": doc, "content": content, "chunk_id": row.chunk_id.0,
                    }),
                )])
                .await?;
        }
        Ok(UpsertReport {
            upserted: rows.len() as u64,
            ..Default::default()
        })
    }
    async fn search(&self, _: &VectorQuery) -> Result<Vec<ScoredChunk>, StorageError> {
        Err(StorageError::InvalidQuery(
            "fixture uses the VectorStorage search port".into(),
        ))
    }
    async fn delete_for_workspace(&self, _: WorkspaceId) -> Result<u64, StorageError> {
        Err(StorageError::InvalidQuery(
            "fixture does not certify lifecycle deletion".into(),
        ))
    }
}
