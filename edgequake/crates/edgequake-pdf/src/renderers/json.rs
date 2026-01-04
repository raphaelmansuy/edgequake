//! JSON renderer for document output.

use crate::schema::Document;
use crate::Result;

use super::Renderer;

/// JSON renderer options.
#[derive(Debug, Clone)]
pub struct JsonOptions {
    /// Pretty-print with indentation
    pub pretty: bool,
    /// Indent size (for pretty printing)
    pub indent: usize,
    /// Include empty fields
    pub include_empty: bool,
}

impl Default for JsonOptions {
    fn default() -> Self {
        Self {
            pretty: true,
            indent: 2,
            include_empty: false,
        }
    }
}

/// JSON renderer.
pub struct JsonRenderer {
    options: JsonOptions,
}

impl JsonRenderer {
    /// Create a new JSON renderer.
    pub fn new() -> Self {
        Self {
            options: JsonOptions::default(),
        }
    }

    /// Create with custom options.
    pub fn with_options(options: JsonOptions) -> Self {
        Self { options }
    }

    /// Create a compact (minified) JSON renderer.
    pub fn compact() -> Self {
        Self {
            options: JsonOptions {
                pretty: false,
                ..Default::default()
            },
        }
    }
}

impl Default for JsonRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for JsonRenderer {
    fn render(&self, document: &Document) -> Result<String> {
        let result = if self.options.pretty {
            serde_json::to_string_pretty(document)
        } else {
            serde_json::to_string(document)
        };

        result.map_err(|e| {
            crate::error::PdfError::PdfParse(format!("JSON serialization error: {}", e))
        })
    }

    fn extension(&self) -> &str {
        "json"
    }

    fn mime_type(&self) -> &str {
        "application/json"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Block, BoundingBox, Page};

    fn create_test_document() -> Document {
        let mut doc = Document::new();
        doc.metadata.title = Some("Test Doc".to_string());

        let mut page = Page::new(1, 612.0, 792.0);
        page.add_block(Block::text(
            "Hello world",
            BoundingBox::new(72.0, 72.0, 540.0, 100.0),
        ));
        doc.add_page(page);
        doc
    }

    #[test]
    fn test_json_rendering() {
        let renderer = JsonRenderer::new();
        let doc = create_test_document();
        let result = renderer.render(&doc).unwrap();

        assert!(result.contains("\"title\""));
        assert!(result.contains("Test Doc"));
        assert!(result.contains("Hello world"));
        assert!(result.contains("\"pages\""));
    }

    #[test]
    fn test_json_compact() {
        let renderer = JsonRenderer::compact();
        let doc = create_test_document();
        let result = renderer.render(&doc).unwrap();

        // Compact shouldn't have indentation
        assert!(!result.contains("\n  "));
    }

    #[test]
    fn test_json_extension() {
        let renderer = JsonRenderer::new();
        assert_eq!(renderer.extension(), "json");
        assert_eq!(renderer.mime_type(), "application/json");
    }

    #[test]
    fn test_json_default() {
        let renderer = JsonRenderer::default();
        assert!(renderer.options.pretty);
        assert_eq!(renderer.options.indent, 2);
    }

    #[test]
    fn test_json_options_default() {
        let opts = JsonOptions::default();
        assert!(opts.pretty);
        assert_eq!(opts.indent, 2);
        assert!(!opts.include_empty);
    }

    #[test]
    fn test_json_with_options() {
        let opts = JsonOptions {
            pretty: false,
            indent: 4,
            include_empty: true,
        };
        let renderer = JsonRenderer::with_options(opts);
        assert!(!renderer.options.pretty);
        assert_eq!(renderer.options.indent, 4);
    }

    #[test]
    fn test_json_empty_document() {
        let renderer = JsonRenderer::new();
        let doc = Document::new();
        let result = renderer.render(&doc).unwrap();

        // Should still produce valid JSON
        assert!(result.starts_with("{"));
        assert!(result.ends_with("}"));
        assert!(result.contains("\"pages\""));
    }

    #[test]
    fn test_json_multiple_pages() {
        let renderer = JsonRenderer::new();
        let mut doc = Document::new();

        let page1 = Page::new(1, 612.0, 792.0);
        let page2 = Page::new(2, 612.0, 792.0);
        doc.add_page(page1);
        doc.add_page(page2);

        let result = renderer.render(&doc).unwrap();
        // Should contain both pages
        assert!(result.contains("\"number\":1") || result.contains("\"number\": 1"));
        assert!(result.contains("\"number\":2") || result.contains("\"number\": 2"));
    }

    #[test]
    fn test_json_contains_blocks() {
        let renderer = JsonRenderer::new();
        let doc = create_test_document();
        let result = renderer.render(&doc).unwrap();

        assert!(result.contains("\"blocks\""));
        assert!(result.contains("\"text\""));
    }

    #[test]
    fn test_json_contains_bbox() {
        let renderer = JsonRenderer::new();
        let doc = create_test_document();
        let result = renderer.render(&doc).unwrap();

        // BoundingBox should be serialized
        assert!(result.contains("72.0") || result.contains("72"));
        assert!(result.contains("540.0") || result.contains("540"));
    }
}
