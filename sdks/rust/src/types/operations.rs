//! Operations types — tasks, pipeline, costs, lineage, chunks, provenance, models.

use serde::Deserialize;

// --- Task types ---

/// Task info.
#[derive(Debug, Clone, Deserialize)]
pub struct TaskInfo {
    pub track_id: String,
    pub status: String,
    #[serde(default)]
    pub progress: Option<f64>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub document_id: Option<String>,
    #[serde(default)]
    pub task_type: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Task list response.
#[derive(Debug, Clone, Deserialize)]
pub struct TaskListResponse {
    #[serde(default)]
    pub tasks: Vec<TaskInfo>,
    #[serde(default)]
    pub total: u32,
}

// --- Pipeline types ---

/// Pipeline status.
#[derive(Debug, Clone, Deserialize)]
pub struct PipelineStatus {
    pub status: String,
    #[serde(default)]
    pub active_tasks: u32,
    #[serde(default)]
    pub queued_tasks: u32,
    #[serde(default)]
    pub completed_tasks: u32,
    #[serde(default)]
    pub failed_tasks: u32,
}

/// Queue metrics.
#[derive(Debug, Clone, Deserialize)]
pub struct QueueMetrics {
    #[serde(default)]
    pub queue_depth: u32,
    #[serde(default)]
    pub processing: u32,
    #[serde(default)]
    pub completed_last_hour: u32,
    #[serde(default)]
    pub failed_last_hour: u32,
    #[serde(default)]
    pub avg_processing_time_ms: Option<f64>,
}

// --- Cost types ---

/// Cost summary.
#[derive(Debug, Clone, Deserialize)]
pub struct CostSummary {
    #[serde(default)]
    pub total_cost_usd: f64,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub total_input_tokens: u64,
    #[serde(default)]
    pub total_output_tokens: u64,
    #[serde(default)]
    pub document_count: u32,
    #[serde(default)]
    pub query_count: u32,
}

/// Cost history entry.
#[derive(Debug, Clone, Deserialize)]
pub struct CostEntry {
    pub date: String,
    #[serde(default)]
    pub cost_usd: f64,
    #[serde(default)]
    pub tokens: u64,
    #[serde(default)]
    pub requests: u32,
}

/// Budget info.
#[derive(Debug, Clone, Deserialize)]
pub struct BudgetInfo {
    #[serde(default)]
    pub monthly_budget_usd: Option<f64>,
    #[serde(default)]
    pub current_spend_usd: f64,
    #[serde(default)]
    pub remaining_usd: Option<f64>,
}

// --- Lineage types ---

/// Lineage node.
#[derive(Debug, Clone, Deserialize)]
pub struct LineageNode {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub node_type: Option<String>,
}

/// Lineage edge.
#[derive(Debug, Clone, Deserialize)]
pub struct LineageEdge {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub relationship: Option<String>,
}

/// Lineage graph.
#[derive(Debug, Clone, Deserialize)]
pub struct LineageGraph {
    #[serde(default)]
    pub nodes: Vec<LineageNode>,
    #[serde(default)]
    pub edges: Vec<LineageEdge>,
    #[serde(default)]
    pub root_id: Option<String>,
}

// --- Chunk types ---

/// Chunk detail.
#[derive(Debug, Clone, Deserialize)]
pub struct ChunkDetail {
    pub id: String,
    #[serde(default)]
    pub document_id: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub chunk_index: Option<u32>,
    #[serde(default)]
    pub token_count: Option<u32>,
}

// --- Provenance ---

/// Provenance record.
#[derive(Debug, Clone, Deserialize)]
pub struct ProvenanceRecord {
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub entity_name: Option<String>,
    #[serde(default)]
    pub document_id: Option<String>,
    #[serde(default)]
    pub chunk_id: Option<String>,
    #[serde(default)]
    pub extraction_method: Option<String>,
    #[serde(default)]
    pub confidence: Option<f64>,
}

// --- Settings ---

/// Provider status.
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderStatus {
    #[serde(default)]
    pub current_provider: Option<String>,
    #[serde(default)]
    pub current_model: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

// --- Models ---

/// Model info.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model_type: Option<String>,
    #[serde(default)]
    pub is_available: bool,
}

/// Providers health.
#[derive(Debug, Clone, Deserialize)]
pub struct ProvidersHealth {
    #[serde(default)]
    pub providers: Vec<ProviderHealthInfo>,
}

/// Health info for a provider.
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderHealthInfo {
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub latency_ms: Option<f64>,
    #[serde(default)]
    pub error: Option<String>,
}
