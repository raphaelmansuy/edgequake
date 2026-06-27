//! API-facing multimodal status DTOs (E52 document detail).

use serde::Serialize;
use utoipa::ToSchema;

use super::item_record::{MultimodalItemRecord, MultimodalItemStatus, MultimodalSummary};
use super::manifest::{ManifestItem, MultimodalManifest};

/// Per-item analyze status for document detail API.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MultimodalItemStatusView {
    pub item_id: String,
    pub modality: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl MultimodalItemStatusView {
    pub fn from_manifest_item(item: &ManifestItem) -> Self {
        match item.analyze_result.as_ref() {
            Some(record) => Self::from_record(&item.item_id, &item.modality, record),
            None => Self {
                item_id: item.item_id.clone(),
                modality: item.modality.clone(),
                status: "pending".into(),
                name: None,
                item_type: None,
                message: None,
            },
        }
    }

    fn from_record(item_id: &str, modality: &str, record: &MultimodalItemRecord) -> Self {
        let status = match record.status {
            MultimodalItemStatus::Success => "success",
            MultimodalItemStatus::Skipped => "skipped",
            MultimodalItemStatus::Failed => "failed",
            MultimodalItemStatus::Degraded => "degraded",
        };
        Self {
            item_id: item_id.to_string(),
            modality: modality.to_string(),
            status: status.into(),
            name: record.name.clone(),
            item_type: record.item_type.clone(),
            message: record.message.clone(),
        }
    }
}

/// Build API item list from manifest (DRY for document detail handler).
pub fn manifest_item_status_views(manifest: &MultimodalManifest) -> Vec<MultimodalItemStatusView> {
    manifest
        .items
        .iter()
        .map(MultimodalItemStatusView::from_manifest_item)
        .collect()
}

/// Parse summary from document metadata JSON.
pub fn summary_from_metadata(metadata: &serde_json::Value) -> Option<MultimodalSummary> {
    metadata
        .get("multimodal_summary")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}
