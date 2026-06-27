//! Per-item analyze outcome (LightRAG `llm_analyze_result` aligned).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Item-level analyze outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MultimodalItemStatus {
    Success,
    Skipped,
    Failed,
    /// EdgeQuake extension: soft-fail with placeholder (degraded mode).
    Degraded,
}

/// LightRAG `llm_analyze_result` schema subset.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MultimodalItemRecord {
    pub item_id: String,
    pub modality: String,
    pub status: MultimodalItemStatus,
    pub analyze_time: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub item_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Normalized LaTeX body (equations only; LightRAG `llm_analyze_result.equation`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// LLM cache keys produced during analyze (LightRAG sidecar `llm_cache_list`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub llm_cache_list: Vec<String>,
}

impl MultimodalItemRecord {
    pub fn skipped(item_id: impl Into<String>, modality: &str, message: impl Into<String>) -> Self {
        Self {
            item_id: item_id.into(),
            modality: modality.to_string(),
            status: MultimodalItemStatus::Skipped,
            analyze_time: Utc::now(),
            name: None,
            item_type: None,
            description: None,
            equation: None,
            message: Some(message.into()),
            llm_cache_list: Vec::new(),
        }
    }

    pub fn success_image(
        item_id: impl Into<String>,
        name: String,
        item_type: String,
        description: String,
    ) -> Self {
        Self::success_modality(item_id, "drawing", name, item_type, description)
    }

    pub fn success_modality(
        item_id: impl Into<String>,
        modality: &str,
        name: String,
        item_type: String,
        description: String,
    ) -> Self {
        Self {
            item_id: item_id.into(),
            modality: modality.to_string(),
            status: MultimodalItemStatus::Success,
            analyze_time: Utc::now(),
            name: Some(name),
            item_type: Some(item_type),
            description: Some(description),
            equation: None,
            message: None,
            llm_cache_list: Vec::new(),
        }
    }

    pub fn success_equation(
        item_id: impl Into<String>,
        name: String,
        equation: String,
        description: String,
    ) -> Self {
        Self {
            item_id: item_id.into(),
            modality: "equation".into(),
            status: MultimodalItemStatus::Success,
            analyze_time: Utc::now(),
            name: Some(name),
            item_type: Some("Equation".into()),
            description: Some(description),
            equation: Some(equation),
            message: None,
            llm_cache_list: Vec::new(),
        }
    }

    pub fn failed(item_id: impl Into<String>, modality: &str, message: impl Into<String>) -> Self {
        Self {
            item_id: item_id.into(),
            modality: modality.to_string(),
            status: MultimodalItemStatus::Failed,
            analyze_time: Utc::now(),
            name: None,
            item_type: None,
            description: None,
            equation: None,
            message: Some(message.into()),
            llm_cache_list: Vec::new(),
        }
    }
}

/// Aggregate counts for document metadata (`multimodal_summary`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct MultimodalSummary {
    pub success: u32,
    pub skipped: u32,
    pub failed: u32,
    pub degraded: u32,
}

impl MultimodalSummary {
    pub fn from_records(records: &[MultimodalItemRecord]) -> Self {
        let mut summary = Self::default();
        for record in records {
            match record.status {
                MultimodalItemStatus::Success => summary.success += 1,
                MultimodalItemStatus::Skipped => summary.skipped += 1,
                MultimodalItemStatus::Failed => summary.failed += 1,
                MultimodalItemStatus::Degraded => summary.degraded += 1,
            }
        }
        summary
    }
}
