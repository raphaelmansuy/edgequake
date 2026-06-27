//! In-process channel notifier for tests and bridged mode without NATS.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::error::TaskResult;

/// Notifies workers that a task track_id is ready (Postgres SSOT holds payload).
#[async_trait]
pub trait TaskNotifier: Send + Sync {
    async fn notify(&self, track_id: &str) -> TaskResult<()>;
}

pub type SharedTaskNotifier = Arc<dyn TaskNotifier>;

/// No-op notifier for local-only deployments.
#[derive(Debug, Default)]
pub struct NoopTaskNotifier;

#[async_trait]
impl TaskNotifier for NoopTaskNotifier {
    async fn notify(&self, _track_id: &str) -> TaskResult<()> {
        Ok(())
    }
}

/// Broadcast-based notifier for tests and hybrid worker hydration.
#[derive(Debug, Clone)]
pub struct ChannelTaskNotifier {
    broadcast: broadcast::Sender<String>,
}

impl ChannelTaskNotifier {
    pub fn new(capacity: usize) -> Self {
        let (broadcast, _) = broadcast::channel(capacity);
        Self { broadcast }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.broadcast.subscribe()
    }
}

#[async_trait]
impl TaskNotifier for ChannelTaskNotifier {
    async fn notify(&self, track_id: &str) -> TaskResult<()> {
        // Best-effort: no active subscribers is OK at enqueue time (workers may
        // subscribe later and rely on Postgres SSOT requeue on startup).
        let _ = self.broadcast.send(track_id.to_string());
        Ok(())
    }
}
