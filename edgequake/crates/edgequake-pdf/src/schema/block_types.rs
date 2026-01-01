//! Block type definitions for document elements.

use serde::{Deserialize, Serialize};

/// Block types in a document (similar to Marker's schema).
///
/// These types represent the semantic classification of document elements,
/// enabling proper formatting and processing in the extraction pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[derive(Default)]
pub enum BlockType {
    // Container blocks
    /// The entire document
    Document,
    /// A single page
    Page,

    // Content blocks
    /// Regular text paragraph
    #[default]
    Text,
    /// Text containing inline math expressions
    TextInlineMath,
    /// A paragraph block (semantic grouping)
    Paragraph,
    /// Section header (h1, h2, etc.)
    SectionHeader,
    /// List item (bullet or numbered)
    ListItem,

    // Special content
    /// Table structure
    Table,
    /// Individual table cell
    TableCell,
    /// Table row
    TableRow,
    /// Figure (chart, diagram, etc.)
    Figure,
    /// Picture/image
    Picture,
    /// Caption for figures or tables
    Caption,
    /// Code block
    Code,
    /// Mathematical equation (display mode)
    Equation,
    /// Form element
    Form,
    /// Footnote
    Footnote,

    // Document structure
    /// Page header (running header)
    PageHeader,
    /// Page footer (running footer)
    PageFooter,
    /// Table of contents
    TableOfContents,

    // Handwritten content
    /// Handwritten text
    Handwriting,

    // Fallback
    /// Unknown or unclassified block
    Unknown,
}

impl BlockType {
    /// Returns true if this block type is a container (can have children).
    pub fn is_container(&self) -> bool {
        matches!(
            self,
            BlockType::Document | BlockType::Page | BlockType::Table | BlockType::TableRow
        )
    }

    /// Returns true if this block type typically contains text.
    pub fn has_text(&self) -> bool {
        matches!(
            self,
            BlockType::Text
                | BlockType::TextInlineMath
                | BlockType::Paragraph
                | BlockType::SectionHeader
                | BlockType::ListItem
                | BlockType::Caption
                | BlockType::Code
                | BlockType::Footnote
                | BlockType::Handwriting
                | BlockType::TableCell
        )
    }

    /// Returns true if this block type should be excluded from main content.
    pub fn is_artifact(&self) -> bool {
        matches!(
            self,
            BlockType::PageHeader | BlockType::PageFooter | BlockType::TableOfContents
        )
    }

    /// Returns the markdown heading level for section headers.
    pub fn heading_level(&self) -> Option<u8> {
        match self {
            BlockType::SectionHeader => Some(2), // Default to h2
            _ => None,
        }
    }

    /// Returns a human-readable label for this block type.
    pub fn label(&self) -> &'static str {
        match self {
            BlockType::Document => "Document",
            BlockType::Page => "Page",
            BlockType::Text => "Text",
            BlockType::TextInlineMath => "Text with Math",
            BlockType::Paragraph => "Paragraph",
            BlockType::SectionHeader => "Section Header",
            BlockType::ListItem => "List Item",
            BlockType::Table => "Table",
            BlockType::TableCell => "Table Cell",
            BlockType::TableRow => "Table Row",
            BlockType::Figure => "Figure",
            BlockType::Picture => "Picture",
            BlockType::Caption => "Caption",
            BlockType::Code => "Code",
            BlockType::Equation => "Equation",
            BlockType::Form => "Form",
            BlockType::Footnote => "Footnote",
            BlockType::PageHeader => "Page Header",
            BlockType::PageFooter => "Page Footer",
            BlockType::TableOfContents => "Table of Contents",
            BlockType::Handwriting => "Handwriting",
            BlockType::Unknown => "Unknown",
        }
    }
}

impl std::fmt::Display for BlockType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_type_is_container() {
        assert!(BlockType::Document.is_container());
        assert!(BlockType::Page.is_container());
        assert!(BlockType::Table.is_container());
        assert!(!BlockType::Text.is_container());
        assert!(!BlockType::SectionHeader.is_container());
    }

    #[test]
    fn test_block_type_has_text() {
        assert!(BlockType::Text.has_text());
        assert!(BlockType::SectionHeader.has_text());
        assert!(BlockType::Code.has_text());
        assert!(!BlockType::Figure.has_text());
        assert!(!BlockType::Picture.has_text());
    }

    #[test]
    fn test_block_type_is_artifact() {
        assert!(BlockType::PageHeader.is_artifact());
        assert!(BlockType::PageFooter.is_artifact());
        assert!(!BlockType::Text.is_artifact());
        assert!(!BlockType::SectionHeader.is_artifact());
    }

    #[test]
    fn test_block_type_serialization() {
        let json = serde_json::to_string(&BlockType::SectionHeader).unwrap();
        assert_eq!(json, r#""SectionHeader""#);

        let parsed: BlockType = serde_json::from_str(r#""Text""#).unwrap();
        assert_eq!(parsed, BlockType::Text);
    }
}
