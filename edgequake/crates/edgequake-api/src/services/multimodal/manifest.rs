//! Multimodal manifest discovery from markdown (virtual sidecar SSOT).

use serde::{Deserialize, Serialize};

use super::item_record::MultimodalItemRecord;
use super::scan::scan_manifest_items;
use super::sidecar::MultimodalHeading;

/// Document-level multimodal manifest (LightRAG sidecar aggregate).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MultimodalManifest {
    pub version: u32,
    pub items: Vec<ManifestItem>,
}

/// Discoverable item before analyze.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestItem {
    pub item_id: String,
    pub modality: String,
    pub start: usize,
    pub end: usize,
    pub matched: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Table/equation body (tables only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Caption from sidecar tag (`caption="…"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    /// Footnote from sidecar tag (`footnote="…"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footnote: Option<String>,
    /// LightRAG sidecar `footnotes` list (merged from tag + backfill).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub footnotes: Vec<String>,
    /// LightRAG `blockid` — scopes surrounding to one blocks.jsonl row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_id: Option<String>,
    /// Nested heading provenance (`heading` dict in LightRAG sidecars).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading: Option<MultimodalHeading>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analyze_result: Option<MultimodalItemRecord>,
}

impl MultimodalManifest {
    pub const CURRENT_VERSION: u32 = 1;

    /// Build manifest from converted markdown (drawings, data-URIs, tables).
    pub fn from_markdown(markdown: &str) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            items: scan_manifest_items(markdown),
        }
    }

    pub fn image_items(&self) -> impl Iterator<Item = &ManifestItem> {
        self.items.iter().filter(|item| item.modality == "drawing")
    }

    pub fn table_items(&self) -> impl Iterator<Item = &ManifestItem> {
        self.items.iter().filter(|item| item.modality == "table")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_drawing_and_data_uri() {
        let b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
        let md = format!(
            r#"<drawing id="im-1" path="assets/x.png" format="png" />
![y](data:image/png;base64,{b64})"#
        );
        let manifest = MultimodalManifest::from_markdown(&md);
        assert_eq!(manifest.items.len(), 2);
        assert_eq!(
            manifest.items[0].asset_path.as_deref(),
            Some("assets/x.png")
        );
    }
}
