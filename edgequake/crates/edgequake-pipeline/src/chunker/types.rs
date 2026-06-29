//! Core types and traits for text chunking.
//!
//! Defines the data structures and strategy trait used by the chunker module.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Section metadata for heading-aware chunks (SPEC-026 Phase 2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SectionMetadata {
    /// Full breadcrumb path, e.g. `["Install", "Prerequisites"]`.
    pub heading_path: Vec<String>,
    /// ATX heading level (1–6); 0 for preface/uncategorized.
    pub heading_level: u8,
}

impl SectionMetadata {
    pub fn from_block(parents: &[String], heading: &str, level: u8) -> Self {
        let mut path = parents.to_vec();
        if !heading.is_empty() && heading != crate::markdown_ir::PREFACE_HEADING {
            path.push(heading.to_string());
        }
        Self {
            heading_path: path,
            heading_level: level,
        }
    }
}

/// Standard page marker embedded in PDF-derived markdown by PDF converters.
///
/// Format: `<!-- edgequake-page:N -->` (1-indexed page number).
/// The chunker parses these markers to split content at page boundaries so
/// that **no chunk ever spans two PDF pages** (First Principle: page attribution).
pub const PAGE_MARKER_PREFIX: &str = "<!-- edgequake-page:";
pub const PAGE_MARKER_SUFFIX: &str = " -->";

/// Build the page marker string for a given 1-indexed page number.
pub fn make_page_marker(page: u32) -> String {
    format!("{}{}{}", PAGE_MARKER_PREFIX, page, PAGE_MARKER_SUFFIX)
}

/// Parse page number from a marker line, returns None if not a marker.
pub fn parse_page_marker(line: &str) -> Option<u32> {
    let trimmed = line.trim();
    let inner = trimmed
        .strip_prefix(PAGE_MARKER_PREFIX)?
        .strip_suffix(PAGE_MARKER_SUFFIX)?;
    inner.trim().parse::<u32>().ok()
}

/// Result of a custom chunking operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChunkResult {
    /// The chunk text content.
    pub content: String,
    /// Approximate token count.
    pub tokens: usize,
    /// Zero-based index indicating the chunk's order in the document.
    pub chunk_order_index: usize,
    /// Optional heading breadcrumb metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<SectionMetadata>,
    /// Source document start offset when the strategy preserves spans.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_offset: Option<usize>,
    /// Source document end offset when the strategy preserves spans.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_offset: Option<usize>,
    /// PDF page number (1-indexed) where this chunk starts.
    /// Set by `PageAwareChunking`; None for non-PDF or single-page sources.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_start: Option<u32>,
    /// PDF page number (1-indexed) where this chunk ends.
    /// Always equal to `page_start` — chunks never cross page boundaries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_end: Option<u32>,
}

/// Trait for custom chunking strategies.
///
/// Implement this trait to provide your own chunking logic for document processing.
/// This allows for flexible chunking strategies such as:
/// - Semantic chunking (based on meaning/topics)
/// - Fixed-size chunking with custom separators
/// - Language-specific chunking (code, markdown, etc.)
#[async_trait]
pub trait ChunkingStrategy: Send + Sync {
    /// Chunk the given text content into smaller pieces.
    ///
    /// # Arguments
    /// * `content` - The full text content to chunk
    /// * `config` - The chunking configuration
    ///
    /// # Returns
    /// A vector of chunk results with content, token count, and order index
    async fn chunk(&self, content: &str, config: &ChunkerConfig) -> Result<Vec<ChunkResult>>;

    /// Get the name of this chunking strategy.
    fn name(&self) -> &str;
}

/// Configuration for the chunker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkerConfig {
    /// Target chunk size in tokens.
    pub chunk_size: usize,

    /// Overlap between chunks in tokens.
    pub chunk_overlap: usize,

    /// Minimum chunk size (won't create chunks smaller than this).
    pub min_chunk_size: usize,

    /// Separator characters for splitting.
    pub separators: Vec<String>,

    /// Whether to preserve sentence boundaries.
    pub preserve_sentences: bool,

    /// Optional character to split on first (e.g., "\n" for line-by-line).
    pub split_by_character: Option<String>,

    /// If true, split only on the specified character, don't apply token limits.
    pub split_by_character_only: bool,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            // WHY 800: The chunker estimates 4 chars/token, but dense technical content
            // (scientific tables, formulas, gene names, numeric data) can be 2–3× denser.
            // At 2 chars/true_token: 800 est-tokens × 4 chars = 3200 chars → 1600 true tokens.
            // This keeps chunks safely within embeddinggemma's 2048-token hard limit (80% margin).
            // Prior default of 1200 produced 4800-char chunks → 2400 true tokens → 400 errors.
            chunk_size: 800,
            chunk_overlap: 100,
            min_chunk_size: 100,
            separators: vec![
                "\n\n".to_string(),
                "\n".to_string(),
                ". ".to_string(),
                "! ".to_string(),
                "? ".to_string(),
                "; ".to_string(),
                ", ".to_string(),
                " ".to_string(),
            ],
            preserve_sentences: true,
            split_by_character: None,
            split_by_character_only: false,
        }
    }
}

/// A chunk of text with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChunk {
    /// Unique identifier for the chunk.
    pub id: String,

    /// The chunk text content.
    pub content: String,

    /// Index of this chunk in the document.
    pub index: usize,

    /// Character offset from the start of the document.
    pub start_offset: usize,

    /// Character offset to the end of the chunk.
    pub end_offset: usize,

    /// Starting line number (1-based) in the original document.
    pub start_line: usize,

    /// Ending line number (1-based, inclusive) in the original document.
    pub end_line: usize,

    /// Approximate token count.
    pub token_count: usize,

    /// Chunk embedding.
    pub embedding: Option<Vec<f32>>,

    /// Heading breadcrumb metadata when chunking strategy is markdown-aware.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<SectionMetadata>,

    /// PDF page number (1-indexed) where this chunk starts.
    /// Set when the source document is a PDF processed with `PageAwareChunking`.
    /// None for plain text, Markdown, or single-page PDFs without markers.
    ///
    /// Key invariant: a chunk NEVER spans two pages — `page_start == page_end`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_start: Option<u32>,

    /// PDF page number (1-indexed) where this chunk ends.
    /// Always equal to `page_start` by construction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_end: Option<u32>,
}

impl TextChunk {
    /// Create a new text chunk.
    pub fn new(
        id: impl Into<String>,
        content: impl Into<String>,
        index: usize,
        start_offset: usize,
        end_offset: usize,
    ) -> Self {
        let content = content.into();
        let token_count = super::text_utils::estimate_tokens(&content);
        Self {
            id: id.into(),
            content,
            index,
            start_offset,
            end_offset,
            start_line: 1,
            end_line: 1,
            token_count,
            embedding: None,
            section: None,
            page_start: None,
            page_end: None,
        }
    }

    /// Create a new text chunk with line numbers.
    pub fn with_line_numbers(
        id: impl Into<String>,
        content: impl Into<String>,
        index: usize,
        start_offset: usize,
        end_offset: usize,
        start_line: usize,
        end_line: usize,
    ) -> Self {
        let content = content.into();
        let token_count = super::text_utils::estimate_tokens(&content);
        Self {
            id: id.into(),
            content,
            index,
            start_offset,
            end_offset,
            start_line,
            end_line,
            token_count,
            embedding: None,
            section: None,
            page_start: None,
            page_end: None,
        }
    }

    /// Attach section metadata after creation.
    pub fn with_section(mut self, section: Option<SectionMetadata>) -> Self {
        self.section = section;
        self
    }

    /// Assign PDF page attribution (1-indexed). page_start == page_end always.
    pub fn with_page(mut self, page: u32) -> Self {
        self.page_start = Some(page);
        self.page_end = Some(page);
        self
    }

    /// Assign page attribution from an Option (no-op when None).
    pub fn with_page_opt(mut self, page: Option<u32>) -> Self {
        if let Some(p) = page {
            self.page_start = Some(p);
            self.page_end = Some(p);
        }
        self
    }

    /// Set line numbers after creation.
    pub fn set_line_numbers(&mut self, start_line: usize, end_line: usize) {
        self.start_line = start_line;
        self.end_line = end_line;
    }
}
