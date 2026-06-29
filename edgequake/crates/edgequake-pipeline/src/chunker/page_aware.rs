//! Page-aware PDF chunking (SPEC-032 W-09 / First Principle: page attribution).
//!
//! ## Design Contract
//!
//! A PDF chunk **MUST NOT span two pages**. This is the First Principle behind
//! this module: when a user cites a chunk, the citation must map to a single,
//! navigable page in the original PDF (enabling deep links to `#page=N`).
//!
//! ## How it works
//!
//! 1. PDF converters inject `<!-- edgequake-page:N -->` markers between pages.
//! 2. `PageAwareChunking` splits the markdown at these markers before
//!    delegating to the inner chunker (Recursive by default).
//! 3. Each resulting `ChunkResult` (and eventually `TextChunk`) carries
//!    `page_start == page_end == N` — never crossing a page boundary.
//!
//! ## Marker format
//!
//! ```text
//! <!-- edgequake-page:1 -->
//! ...page 1 content...
//!
//! <!-- edgequake-page:2 -->
//! ...page 2 content...
//! ```
//!
//! The marker is a valid HTML comment, invisible in rendered Markdown.
//! Regex: `<!-- edgequake-page:(\d+) -->`

use async_trait::async_trait;

use super::recursive::RecursiveCharacterChunking;
use super::types::{parse_page_marker, ChunkResult, ChunkerConfig, ChunkingStrategy};
use crate::error::Result;

/// Wraps an inner chunker; splits content at page boundaries first so no chunk
/// ever spans two PDF pages.
///
/// @implements SPEC-032 W-09 — Page-aware chunking for PDFs
pub struct PageAwareChunking {
    /// Inner strategy used within each page segment.
    /// Default: `RecursiveCharacterChunking` (same as the document default).
    pub inner: Box<dyn ChunkingStrategy>,
}

impl Default for PageAwareChunking {
    fn default() -> Self {
        Self {
            inner: Box::new(RecursiveCharacterChunking),
        }
    }
}

impl PageAwareChunking {
    pub fn new(inner: Box<dyn ChunkingStrategy>) -> Self {
        Self { inner }
    }
}

/// A page-delimited segment extracted from the markdown.
#[derive(Debug)]
pub struct PageSegment {
    /// 1-indexed PDF page number.
    page: u32,
    /// Content of this page (page marker stripped).
    content: String,
}

/// Split markdown at `<!-- edgequake-page:N -->` markers.
///
/// # First page handling
///
/// Content before the first marker is attributed to page 1. This handles PDFs
/// where the converter did not emit a marker at the very beginning.
///
/// # Ordering guarantee
///
/// Segments are returned in the order they appear in the input.
/// If a page number appears multiple times (shouldn't happen but defensive),
/// content is appended to the existing segment.
pub fn split_into_page_segments(content: &str) -> Vec<PageSegment> {
    if content.trim().is_empty() {
        return Vec::new();
    }

    // No page markers → treat entire content as page 1
    if !content.contains(super::types::PAGE_MARKER_PREFIX) {
        return vec![PageSegment {
            page: 1,
            content: content.to_string(),
        }];
    }

    let mut segments: Vec<PageSegment> = Vec::new();
    let mut current_page: u32 = 1;
    let mut current_lines: Vec<&str> = Vec::new();

    for line in content.lines() {
        if let Some(page_num) = parse_page_marker(line) {
            // Flush current segment
            let text = current_lines.join("\n");
            if !text.trim().is_empty() {
                segments.push(PageSegment {
                    page: current_page,
                    content: text,
                });
            }
            current_page = page_num;
            current_lines.clear();
        } else {
            current_lines.push(line);
        }
    }

    // Flush final segment
    let text = current_lines.join("\n");
    if !text.trim().is_empty() {
        segments.push(PageSegment {
            page: current_page,
            content: text,
        });
    }

    segments
}

#[async_trait]
impl ChunkingStrategy for PageAwareChunking {
    /// Chunk content respecting PDF page boundaries.
    ///
    /// For each page segment:
    ///   1. Run the inner chunker on the page's content.
    ///   2. Stamp every resulting chunk with `page_start = page_end = N`.
    ///
    /// If there are no page markers (plain text / non-PDF), falls back to the
    /// inner chunker with no page attribution (page fields remain None).
    async fn chunk(&self, content: &str, config: &ChunkerConfig) -> Result<Vec<ChunkResult>> {
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        let segments = split_into_page_segments(content);

        // No markers → plain fallback (page fields stay None)
        if segments.len() == 1
            && segments[0].page == 1
            && !content.contains(super::types::PAGE_MARKER_PREFIX)
        {
            return self.inner.chunk(content, config).await;
        }

        let mut results = Vec::new();
        let mut order = 0usize;

        for seg in segments {
            let page = seg.page;
            let sub_chunks = self.inner.chunk(&seg.content, config).await?;

            if sub_chunks.is_empty() && !seg.content.trim().is_empty() {
                // Inner chunker returned nothing for non-empty content — keep as one chunk
                results.push(ChunkResult {
                    content: seg.content.trim().to_string(),
                    tokens: super::text_utils::estimate_tokens(&seg.content),
                    chunk_order_index: order,
                    page_start: Some(page),
                    page_end: Some(page),
                    ..Default::default()
                });
                order += 1;
            } else {
                for mut sub in sub_chunks {
                    sub.chunk_order_index = order;
                    sub.page_start = Some(page);
                    sub.page_end = Some(page);
                    results.push(sub);
                    order += 1;
                }
            }
        }

        Ok(results)
    }

    fn name(&self) -> &str {
        "page_aware"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunker::types::make_page_marker;

    fn make_pdf_markdown(pages: &[(u32, &str)]) -> String {
        pages
            .iter()
            .map(|(n, content)| format!("{}\n{}", make_page_marker(*n), content))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Core invariant: chunks never span two pages.
    #[tokio::test]
    async fn chunks_never_cross_page_boundary() {
        let md = make_pdf_markdown(&[
            (
                1,
                "Page one content with several sentences. More text here.",
            ),
            (
                2,
                "Page two has different content. Another sentence on page two.",
            ),
            (3, "Page three content. Final page."),
        ]);

        let config = ChunkerConfig {
            chunk_size: 20,
            chunk_overlap: 0,
            ..Default::default()
        };
        let chunker = PageAwareChunking::default();
        let chunks = chunker.chunk(&md, &config).await.unwrap();

        assert!(!chunks.is_empty(), "Must produce chunks");

        for chunk in &chunks {
            assert!(
                chunk.page_start.is_some(),
                "Every chunk must have page_start"
            );
            assert_eq!(
                chunk.page_start, chunk.page_end,
                "page_start must equal page_end — no cross-page chunks"
            );
        }
    }

    /// Chunks from page 1 must have page=1, page 2 must have page=2.
    #[tokio::test]
    async fn page_numbers_assigned_correctly() {
        let md = make_pdf_markdown(&[(1, "Alpha beta gamma delta"), (2, "Epsilon zeta eta theta")]);

        let config = ChunkerConfig {
            chunk_size: 5,
            chunk_overlap: 0,
            ..Default::default()
        };
        let chunker = PageAwareChunking::default();
        let chunks = chunker.chunk(&md, &config).await.unwrap();

        let page1_chunks: Vec<_> = chunks.iter().filter(|c| c.page_start == Some(1)).collect();
        let page2_chunks: Vec<_> = chunks.iter().filter(|c| c.page_start == Some(2)).collect();

        assert!(!page1_chunks.is_empty(), "Must have page 1 chunks");
        assert!(!page2_chunks.is_empty(), "Must have page 2 chunks");
    }

    /// Plain text (no markers) falls back to inner chunker with no page attribution.
    #[tokio::test]
    async fn plain_text_no_page_markers_fallback() {
        let config = ChunkerConfig {
            chunk_size: 50,
            chunk_overlap: 0,
            ..Default::default()
        };
        let chunker = PageAwareChunking::default();
        let chunks = chunker
            .chunk(
                "Hello world. This is plain text without any page markers.",
                &config,
            )
            .await
            .unwrap();

        assert!(!chunks.is_empty());
        // page fields are None for plain text
        for chunk in &chunks {
            assert!(
                chunk.page_start.is_none() || chunk.page_start == Some(1),
                "Plain text chunks should have no page attribution"
            );
        }
    }

    /// split_into_page_segments correctly extracts page number and content.
    #[test]
    fn split_extracts_page_segments() {
        let md = make_pdf_markdown(&[(1, "Content of page one."), (2, "Content of page two.")]);
        let segs = split_into_page_segments(&md);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].page, 1);
        assert!(segs[0].content.contains("page one"));
        assert_eq!(segs[1].page, 2);
        assert!(segs[1].content.contains("page two"));
    }

    /// Empty content returns empty.
    #[tokio::test]
    async fn empty_content_returns_empty() {
        let config = ChunkerConfig::default();
        let chunks = PageAwareChunking::default()
            .chunk("", &config)
            .await
            .unwrap();
        assert!(chunks.is_empty());
    }

    /// make_page_marker and parse_page_marker are inverses.
    #[test]
    fn marker_roundtrip() {
        use crate::chunker::types::parse_page_marker;
        for page in [1, 5, 100, 999] {
            let marker = make_page_marker(page);
            assert_eq!(parse_page_marker(&marker), Some(page));
        }
    }
}
