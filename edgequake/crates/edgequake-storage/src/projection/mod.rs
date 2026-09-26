//! Durable graph/vector projection delivery (SPEC-149).

pub mod appliers;
#[cfg(feature = "provider-access-fault")]
pub mod fault;
pub mod ledger;
pub mod payload;
pub mod runtime;
pub mod serving_fence_port;
pub mod worker;

pub use appliers::{AgeGraphProjectionApplier, PgvectorProjectionApplier};
pub use edgequake_storage_contracts::ProjectionLedger;
pub use ledger::{PgProjectionLedger, ProjectionWorkLedger};
pub use payload::{
    scoped_graph_node_id, ProjectionApplyReceipt, ProjectionTarget, ProjectionWorkItem,
    PROJECTION_SCHEMA_V1,
};
pub use runtime::ProjectionWorkerRuntime;
pub use serving_fence_port::ServingFenceOpener;
pub use worker::{
    GraphProjectionApplier, ProjectionRunReport, ProjectionWorker, ProjectionWorkerConfig,
    ProjectionWorkerCounterSnapshot, ProjectionWorkerCounters, VectorProjectionApplier,
};

#[cfg(test)]
pub use worker::{NoopGraphProjectionApplier, NoopVectorProjectionApplier};
