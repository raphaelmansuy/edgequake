//! Metrics types: trigger classification and periodic snapshots.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Describes what triggered a metrics collection.
///
/// WHY three variants: Supports event-driven (document ingestion),
/// cron-like (periodic health checks), and on-demand (admin dashboard)
/// metrics collection patterns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MetricsTriggerType {
    /// Triggered by a specific event (e.g., document ingestion completed).
    Event,
    /// Triggered by a scheduled interval (e.g., every 5 minutes).
    Scheduled,
    /// Triggered manually (e.g., admin dashboard refresh).
    Manual,
}

impl MetricsTriggerType {
    /// Convert to a stable string for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            MetricsTriggerType::Event => "event",
            MetricsTriggerType::Scheduled => "scheduled",
            MetricsTriggerType::Manual => "manual",
        }
    }

    /// Parse from a string (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "event" => Some(MetricsTriggerType::Event),
            "scheduled" => Some(MetricsTriggerType::Scheduled),
            "manual" => Some(MetricsTriggerType::Manual),
            _ => None,
        }
    }
}

impl fmt::Display for MetricsTriggerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A point-in-time snapshot of workspace metrics.
///
/// WHY separate from WorkspaceStats: Snapshots are timestamped records
/// for trend analysis, while WorkspaceStats is the current state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Unique snapshot ID.
    pub id: Uuid,
    /// Workspace this snapshot belongs to.
    pub workspace_id: Uuid,
    /// When the snapshot was recorded.
    pub recorded_at: chrono::DateTime<chrono::Utc>,
    /// What triggered this snapshot.
    pub trigger_type: MetricsTriggerType,
    /// Document count at snapshot time.
    pub document_count: usize,
    /// Chunk count at snapshot time.
    pub chunk_count: usize,
    /// Entity count at snapshot time.
    pub entity_count: usize,
    /// Relationship count at snapshot time.
    pub relationship_count: usize,
    /// Embedding count at snapshot time.
    pub embedding_count: usize,
    /// Storage used in bytes at snapshot time.
    pub storage_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_type_as_str() {
        assert_eq!(MetricsTriggerType::Event.as_str(), "event");
        assert_eq!(MetricsTriggerType::Scheduled.as_str(), "scheduled");
        assert_eq!(MetricsTriggerType::Manual.as_str(), "manual");
    }

    #[test]
    fn test_trigger_type_display() {
        assert_eq!(MetricsTriggerType::Event.to_string(), "event");
        assert_eq!(MetricsTriggerType::Scheduled.to_string(), "scheduled");
        assert_eq!(MetricsTriggerType::Manual.to_string(), "manual");
    }

    #[test]
    fn test_trigger_type_parse_roundtrip() {
        for variant in [MetricsTriggerType::Event, MetricsTriggerType::Scheduled, MetricsTriggerType::Manual] {
            let s = variant.as_str();
            assert_eq!(MetricsTriggerType::parse(s), Some(variant));
        }
    }

    #[test]
    fn test_trigger_type_parse_case_insensitive() {
        assert_eq!(MetricsTriggerType::parse("EVENT"), Some(MetricsTriggerType::Event));
        assert_eq!(MetricsTriggerType::parse("Scheduled"), Some(MetricsTriggerType::Scheduled));
        assert_eq!(MetricsTriggerType::parse("MANUAL"), Some(MetricsTriggerType::Manual));
    }

    #[test]
    fn test_trigger_type_parse_unknown() {
        assert_eq!(MetricsTriggerType::parse("cron"), None);
        assert_eq!(MetricsTriggerType::parse(""), None);
    }

    #[test]
    fn test_metrics_snapshot_construction() {
        let ws_id = Uuid::new_v4();
        let snap = MetricsSnapshot {
            id: Uuid::new_v4(),
            workspace_id: ws_id,
            recorded_at: chrono::Utc::now(),
            trigger_type: MetricsTriggerType::Event,
            document_count: 42,
            chunk_count: 200,
            entity_count: 50,
            relationship_count: 80,
            embedding_count: 200,
            storage_bytes: 1024 * 1024,
        };
        assert_eq!(snap.workspace_id, ws_id);
        assert_eq!(snap.document_count, 42);
        assert_eq!(snap.storage_bytes, 1024 * 1024);
    }
}
