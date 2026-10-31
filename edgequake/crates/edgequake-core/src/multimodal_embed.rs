//! SPEC-180 08: the port for embedders that take text and images in one vector space.
//!
//! `EmbeddingProvider::embed(&[String])` stays the text-only port. A normal workspace with
//! `embedding_modalities = text,image` uses this one only; nothing wraps one in the other
//! by dropping the images. Pure types: the HTTP client lives in `edgequake-api`.

use async_trait::async_trait;

/// One piece of an embedding input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbedPart {
    Text(String),
    Image { mime: String, bytes: Vec<u8> },
}

/// One input yields one vector, whatever its parts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmbedInput {
    pub parts: Vec<EmbedPart>,
}

impl EmbedInput {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            parts: vec![EmbedPart::Text(text.into())],
        }
    }

    pub fn image(mime: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            parts: vec![EmbedPart::Image {
                mime: mime.into(),
                bytes,
            }],
        }
    }

    /// An image first, then text that has no task prefix (R-180-5).
    pub fn image_with_text(
        mime: impl Into<String>,
        bytes: Vec<u8>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            parts: vec![
                EmbedPart::Image {
                    mime: mime.into(),
                    bytes,
                },
                EmbedPart::Text(text.into()),
            ],
        }
    }

    pub fn image_count(&self) -> usize {
        self.parts
            .iter()
            .filter(|p| matches!(p, EmbedPart::Image { .. }))
            .count()
    }
}

/// Longest document title kept in the text prefix, in characters.
pub const MAX_TITLE_PREFIX_CHARS: usize = 200;

/// Text part of a document or chunk input: `title: {title} | text: {chunk}`.
pub fn document_prefix(title: &str, chunk: &str) -> String {
    let title: String = title.trim().chars().take(MAX_TITLE_PREFIX_CHARS).collect();
    let title = if title.is_empty() { "none" } else { &title };
    format!("title: {title} | text: {chunk}")
}

/// Text part of a query input: `task: search result | query: {query}`.
pub fn query_prefix(query: &str) -> String {
    format!("task: search result | query: {}", query.trim())
}

/// Stable codes of the embedding failures (SPEC-180 08 errors table).
pub mod embed_codes {
    pub const AUTH: &str = "embedding_auth";
    pub const DIMENSION_MISMATCH: &str = "embedding_dimension_mismatch";
    pub const IMAGE_TOO_LARGE: &str = "image_too_large";
    pub const IMAGE_ANIMATED: &str = "image_animated_unsupported";
    pub const IMAGE_DECODE_FAILED: &str = "image_decode_failed";
    pub const REQUEST: &str = "embedding_request_failed";
    pub const INPUT_TOO_LARGE: &str = "embedding_input_too_large";
}

/// A coded embedding failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct EmbedError {
    pub code: &'static str,
    pub message: String,
}

impl EmbedError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// An embedder whose inputs may carry images.
#[async_trait]
pub trait MultimodalEmbedder: Send + Sync {
    /// Logical model name (the typed ANN registry key).
    fn model(&self) -> &str;

    /// Width of every returned vector; fixed at construction.
    fn dimension(&self) -> usize;

    /// One vector per input, in order, each L2-normalised.
    async fn embed(&self, inputs: &[EmbedInput]) -> Result<Vec<Vec<f32>>, EmbedError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_prefix_truncates_the_title_only() {
        let long = "t".repeat(MAX_TITLE_PREFIX_CHARS + 50);
        let text = document_prefix(&long, "body");
        assert!(text.starts_with(&format!(
            "title: {} | text: body",
            "t".repeat(MAX_TITLE_PREFIX_CHARS)
        )));
        assert!(text.ends_with("| text: body"));
    }

    #[test]
    fn blank_title_reads_none() {
        assert_eq!(document_prefix("  ", "x"), "title: none | text: x");
    }

    #[test]
    fn query_prefix_trims() {
        assert_eq!(
            query_prefix("  kestrel "),
            "task: search result | query: kestrel"
        );
    }

    #[test]
    fn image_with_text_puts_the_image_first() {
        let input = EmbedInput::image_with_text("image/png", vec![1], "caption");
        assert!(matches!(input.parts[0], EmbedPart::Image { .. }));
        assert_eq!(input.image_count(), 1);
    }
}
