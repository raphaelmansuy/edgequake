//! Driver-free contracts shared by EdgeQuake storage providers.

pub mod binding;
pub mod error;
pub mod graph;
pub mod ids;
pub mod ingestion_validate;
pub mod operational;
pub mod projection;
pub mod relational;
pub mod scope;
pub mod vector;

pub use binding::{
    require_active_roles, BindingRegistry, BindingRole, BindingState, DataBindingDescriptor,
    P0_REQUIRED_ROLES,
};
pub use error::{AccessError, AccessResult};
pub use graph::{EdgeDirection, IncidentEdgesRequest, ScopedGraphRead, VersionedEdge};
pub use ids::{DocumentId, EmbeddingKey, GraphEdgeKey, GraphNodeKey, TenantId, WorkspaceId};
pub use ingestion_validate::{
    checked_i64, decode_commit_receipt, last_included_cursor, payload_digest, physical_revision_id,
    validate_prepared_ingestion_batch, ValidatedBatch, MAX_BATCH_RECORDS,
};
pub use operational::{
    ApiKey, CheckpointArtifactStore, IdentityStore, IdentityUser, RefreshToken, SessionStore,
    WorkspaceRecord, WorkspaceStore,
};
pub use projection::{
    all_contributors_serving, AckDelivery, ClaimDeliveries, DeliveryState, ProjectionDelivery,
    ProjectionEvent, ProjectionLedger, ProjectionOperation, QuarantineDelivery, RenewDelivery,
    VisibilityKey, VisibilityLifecycle, VisibilityRepository, VisibilityState,
};
pub use relational::{
    CommitReceipt, CommittedRevision, CursorPage, DeleteDocument, DeleteReceipt, Digest,
    DocumentPageRequest, DocumentReader, DocumentView, IngestionCommitter, LifecycleCommitter,
    PreparedIngestionBatch, PreparedRecord,
};
pub use scope::AccessScope;
pub use vector::{
    ScopedVectorSearch, VectorModelDescriptor, VectorSearchHit, VectorSearchMode, VectorSearchPage,
    VectorSearchRequest,
};

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Arc;

    struct StubVectorSearch;

    #[async_trait]
    impl ScopedVectorSearch for StubVectorSearch {
        async fn search(&self, _request: &VectorSearchRequest) -> AccessResult<VectorSearchPage> {
            Ok(VectorSearchPage {
                hits: Vec::new(),
                next_cursor: None,
                budget_exhausted: false,
            })
        }
    }

    struct StubIngestionCommitter;

    #[async_trait]
    impl IngestionCommitter for StubIngestionCommitter {
        async fn commit_batch(
            &self,
            command: &PreparedIngestionBatch,
        ) -> AccessResult<CommitReceipt> {
            Ok(CommitReceipt {
                request_key: command.idempotency_key.clone(),
                command_digest: command.canonical_digest,
                document_generation: command.ingest_generation,
                committed: Vec::new(),
                manifest_id: uuid::Uuid::nil(),
                durable_commit_token: "stub".into(),
            })
        }
    }

    #[test]
    fn core_async_ports_are_dyn_compatible() {
        let _: Arc<dyn ScopedVectorSearch> = Arc::new(StubVectorSearch);
        let _: Arc<dyn IngestionCommitter> = Arc::new(StubIngestionCommitter);
    }
}
