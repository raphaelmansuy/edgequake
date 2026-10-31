//! What a normal-profile query carries beyond a graph query.

use edgequake_storage::traits::ChunkKinds;
use serde::{Deserialize, Serialize};

/// Which inputs formed the dense query (telemetry and fusion shape).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryModality {
    Text,
    Image,
    Both,
}

impl QueryModality {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Both => "both",
        }
    }
}

/// Presence on a [`crate::QueryRequest`] means "this workspace is `normal`".
///
/// The API layer fills it from the workspace; the engine never asks which profile a
/// workspace has, it only asks whether a plan is present.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NormalQueryPlan {
    /// Text query vector computed by the workspace embedder (a multimodal embedder for
    /// `text,image` workspaces). `None` means the engine embeds the query text itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_vector: Option<Vec<f32>>,
    /// Query image vector (SPEC-180 09: its own dense search, never concatenated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_vector: Option<Vec<f32>>,
    /// Document-kind, MIME and chunk-modality predicates, pushed into SQL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kinds: Option<ChunkKinds>,
}

impl NormalQueryPlan {
    /// A plan whose text vector the engine computes.
    pub fn text() -> Self {
        Self::default()
    }

    pub fn with_kinds(mut self, kinds: Option<ChunkKinds>) -> Self {
        self.kinds = kinds.filter(|k| !k.is_empty());
        self
    }

    pub fn with_image_vector(mut self, vector: Vec<f32>) -> Self {
        self.image_vector = Some(vector);
        self
    }

    pub fn with_text_vector(mut self, vector: Vec<f32>) -> Self {
        self.text_vector = Some(vector);
        self
    }

    /// Which inputs this plan carries; the text vector is implied when absent.
    pub fn modality(&self) -> QueryModality {
        match (self.image_vector.is_some(), self.text_vector.is_some()) {
            (true, true) => QueryModality::Both,
            (true, false) => QueryModality::Image,
            _ => QueryModality::Text,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modality_follows_the_vectors() {
        assert_eq!(NormalQueryPlan::text().modality(), QueryModality::Text);
        let image = NormalQueryPlan::text().with_image_vector(vec![1.0]);
        assert_eq!(image.modality(), QueryModality::Image);
        assert_eq!(
            image.with_text_vector(vec![1.0]).modality(),
            QueryModality::Both
        );
    }

    #[test]
    fn empty_kinds_are_dropped() {
        let plan = NormalQueryPlan::text().with_kinds(Some(ChunkKinds::default()));
        assert!(plan.kinds.is_none());
        let kept = NormalQueryPlan::text().with_kinds(Some(ChunkKinds {
            doc_kinds: Some(vec!["pdf".into()]),
            ..Default::default()
        }));
        assert!(kept.kinds.is_some());
    }
}
