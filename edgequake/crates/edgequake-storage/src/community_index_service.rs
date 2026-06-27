//! Debounced community index refresh scheduler (SPEC-024 Phase 4.2 / SRP).
//!
//! Coalesces burst ingests into one Louvain run per workspace window.
//! Called from [`crate::community_persist::schedule_community_index_refresh`].

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::community::CommunityConfig;
use crate::community_persist::{community_features_enabled, detect_and_persist_communities};
use crate::traits::GraphStorage;

/// Debounce window for post-ingest community refresh (default 300s).
pub fn community_refresh_debounce_secs() -> u64 {
    std::env::var("EDGEQUAKE_COMMUNITY_REFRESH_DEBOUNCE_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300)
}

fn debounce_duration() -> Duration {
    Duration::from_secs(community_refresh_debounce_secs())
}

/// Schedule a debounced community refresh for a workspace (SPEC-024 1.3).
pub async fn schedule_community_index_refresh(
    graph: Arc<dyn GraphStorage>,
    workspace_id: Option<String>,
) {
    if !community_features_enabled() {
        return;
    }
    community_scheduler().schedule(workspace_id, graph).await;
}

struct CommunityRefreshScheduler {
    timers: Mutex<HashMap<String, JoinHandle<()>>>,
}

impl CommunityRefreshScheduler {
    async fn pending_workspace_count(&self) -> usize {
        self.timers.lock().await.len()
    }

    async fn schedule(&self, workspace_id: Option<String>, graph: Arc<dyn GraphStorage>) {
        let key = workspace_id.unwrap_or_else(|| "default".to_string());
        let debounce = debounce_duration();

        let mut timers = self.timers.lock().await;
        if let Some(handle) = timers.remove(&key) {
            handle.abort();
        }

        let handle = tokio::spawn(async move {
            tokio::time::sleep(debounce).await;
            run_community_refresh(graph).await;
        });

        timers.insert(key, handle);
    }
}

fn community_scheduler() -> &'static CommunityRefreshScheduler {
    static SCHEDULER: OnceLock<CommunityRefreshScheduler> = OnceLock::new();
    SCHEDULER.get_or_init(|| CommunityRefreshScheduler {
        timers: Mutex::new(HashMap::new()),
    })
}

async fn run_community_refresh(graph: Arc<dyn GraphStorage>) {
    if !community_features_enabled() {
        return;
    }
    match detect_and_persist_communities(graph, &CommunityConfig::default()).await {
        Ok(result) => tracing::debug!(
            communities = result.communities.len(),
            labeled_nodes = result.node_to_community.len(),
            "Community index refreshed after ingest"
        ),
        Err(e) => tracing::warn!(
            error = %e,
            "Community index refresh failed (non-fatal)"
        ),
    }
}

/// Number of workspaces with a debounced community refresh timer pending (scale signal).
pub async fn pending_community_refresh_workspaces() -> usize {
    if !community_features_enabled() {
        return 0;
    }
    community_scheduler().pending_workspace_count().await
}

/// Run community refresh immediately (tests / one-shot hooks).
pub async fn refresh_community_index_now(graph: Arc<dyn GraphStorage>) {
    run_community_refresh(graph).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debounce_default_is_five_minutes() {
        std::env::remove_var("EDGEQUAKE_COMMUNITY_REFRESH_DEBOUNCE_SECS");
        assert_eq!(community_refresh_debounce_secs(), 300);
    }
}
