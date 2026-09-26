//! Owned projection worker lifecycle for P0 postgres serving.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::{
    GraphProjectionApplier, ProjectionWorkLedger, ProjectionWorker, ProjectionWorkerConfig,
    ServingFenceOpener, VectorProjectionApplier,
};

/// Cancellable projection replay runtime owned by `AppState`.
pub struct ProjectionWorkerRuntime {
    shutdown_tx: watch::Sender<bool>,
    join: JoinHandle<()>,
}

impl ProjectionWorkerRuntime {
    /// Spawn a bounded polling loop. Dropping this handle signals shutdown and
    /// waits briefly for the task to exit.
    pub fn spawn(
        owner_token: uuid::Uuid,
        ledger: Arc<dyn ProjectionWorkLedger>,
        graph: Arc<dyn GraphProjectionApplier>,
        vector: Arc<dyn VectorProjectionApplier>,
        config: ProjectionWorkerConfig,
        poll_interval: Duration,
    ) -> Self {
        Self::spawn_with_serving_fence(
            owner_token,
            ledger,
            graph,
            vector,
            config,
            poll_interval,
            None,
        )
    }

    /// Like [`spawn`], and opens the SPEC-091 serving fence after settled
    /// `document_batch` projection acks when `serving_fence` is set.
    pub fn spawn_with_serving_fence(
        owner_token: uuid::Uuid,
        ledger: Arc<dyn ProjectionWorkLedger>,
        graph: Arc<dyn GraphProjectionApplier>,
        vector: Arc<dyn VectorProjectionApplier>,
        config: ProjectionWorkerConfig,
        poll_interval: Duration,
        serving_fence: Option<Arc<dyn ServingFenceOpener>>,
    ) -> Self {
        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        let worker = ProjectionWorker::with_serving_fence(
            owner_token,
            ledger,
            graph,
            vector,
            config,
            serving_fence,
        );
        let join = tokio::spawn(async move {
            let mut interval = tokio::time::interval(poll_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            info!("SPEC-149 projection worker shutting down");
                            break;
                        }
                    }
                    _ = interval.tick() => {
                        match worker.run_once().await {
                            Ok(report) if report.claimed > 0 => info!(
                                claimed = report.claimed,
                                applied = report.applied,
                                quarantined = report.quarantined,
                                transient_failures = report.transient_failures,
                                "SPEC-149 projection replay batch completed"
                            ),
                            Ok(_) => {}
                            Err(error) => warn!(
                                error = %error,
                                "SPEC-149 projection replay iteration failed"
                            ),
                        }
                    }
                }
            }
        });
        Self { shutdown_tx, join }
    }

    pub fn request_shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

impl Drop for ProjectionWorkerRuntime {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.send(true);
        self.join.abort();
    }
}
