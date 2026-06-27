//! LightRAG nested sidecar schema (`pipeline.py` `_build_mm_chunks_from_sidecars`).

use serde::{Deserialize, Serialize};

/// Nested heading block on mm chunks (LightRAG sidecar `heading` dict).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MultimodalHeading {
    pub level: u32,
    pub heading: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parent_headings: Vec<String>,
}

impl MultimodalHeading {
    pub fn from_legacy(
        heading_text: Option<&str>,
        level: u32,
        parents: Vec<String>,
    ) -> Option<Self> {
        let heading = heading_text.unwrap_or("").trim().to_string();
        if heading.is_empty() && parents.is_empty() && level == 0 {
            return None;
        }
        Some(Self {
            level,
            heading,
            parent_headings: parents,
        })
    }
}

/// Sidecar reference entry (`refs` array item).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MultimodalSidecarRef {
    #[serde(rename = "type")]
    pub ref_type: String,
    pub id: String,
}

/// Sidecar provenance block on indexed mm chunks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MultimodalSidecar {
    #[serde(rename = "type")]
    pub sidecar_type: String,
    pub id: String,
    pub refs: Vec<MultimodalSidecarRef>,
}

/// Build LightRAG `sidecar` dict for one manifest item.
pub fn build_sidecar_block(modality: &str, item_id: &str) -> MultimodalSidecar {
    MultimodalSidecar {
        sidecar_type: modality.to_string(),
        id: item_id.to_string(),
        refs: vec![MultimodalSidecarRef {
            ref_type: modality.to_string(),
            id: item_id.to_string(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_block_matches_lightrag_shape() {
        let block = build_sidecar_block("drawing", "d1");
        assert_eq!(block.sidecar_type, "drawing");
        assert_eq!(block.id, "d1");
        assert_eq!(block.refs.len(), 1);
        assert_eq!(block.refs[0].ref_type, "drawing");
    }
}
