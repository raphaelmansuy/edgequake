//! Document renderers for output formats.
//!
//! Renderers convert the block-based document representation to
//! various output formats like Markdown, JSON, HTML, etc.

mod json;
mod markdown;

pub use json::JsonRenderer;
pub use markdown::{MarkdownRenderer, MarkdownStyle};

use crate::schema::Document;
use crate::Result;

/// Trait for document renderers.
pub trait Renderer: Send + Sync {
    /// Render a document to a string.
    fn render(&self, document: &Document) -> Result<String>;

    /// Get the file extension for this format.
    fn extension(&self) -> &str;

    /// Get the MIME type for this format.
    fn mime_type(&self) -> &str;
}
