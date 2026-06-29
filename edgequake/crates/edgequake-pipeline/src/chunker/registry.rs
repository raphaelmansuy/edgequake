//! Chunk strategy registry (SPEC-026 Phase 2 — Open/Closed selection SSOT).

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::markdown_chunking::MarkdownChunking;
use super::page_aware::PageAwareChunking;
use super::recursive::RecursiveCharacterChunking;
use super::strategies::TokenBasedChunking;
use super::types::{ChunkerConfig, ChunkingStrategy};
use super::Chunker;

/// Ingestion chunk strategy (LightRAG F/R/P subset — no semantic vector).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChunkStrategy {
    /// Fixed token sliding window with adaptive sizing (LightRAG `F`).
    Fixed,
    /// Recursive character cascade (LightRAG `R`) — **EdgeQuake default** (SPEC-026).
    #[default]
    Recursive,
    /// Markdown heading-aware splits with breadcrumbs.
    Markdown,
    /// Page-aware chunking for PDF sources (SPEC-032 W-09).
    ///
    /// Wraps the Recursive strategy but treats `<!-- edgequake-page:N -->`
    /// markers as hard split points so **no chunk ever crosses a page boundary**.
    Pdf,
}

impl ChunkStrategy {
    /// Parse from API string; returns None for unknown values.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "fixed" | "f" => Some(Self::Fixed),
            "recursive" | "r" => Some(Self::Recursive),
            "markdown" | "md" | "p" => Some(Self::Markdown),
            "pdf" => Some(Self::Pdf),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Recursive => "recursive",
            Self::Markdown => "markdown",
            Self::Pdf => "pdf",
        }
    }

    /// Auto-select the best strategy for an uploaded file.
    ///
    /// - `.md` / `text/markdown` → `Markdown`
    /// - `.pdf` / `application/pdf` → `Pdf`  (page-aware, SPEC-032)
    /// - anything else → `Recursive` (default)
    pub fn resolve_for_upload(
        explicit: Option<Self>,
        mime_type: Option<&str>,
        filename: &str,
    ) -> Self {
        if let Some(s) = explicit {
            return s;
        }
        let lc = filename.to_lowercase();
        let is_md = mime_type.is_some_and(|m| m.contains("markdown"))
            || lc.ends_with(".md")
            || lc.ends_with(".markdown");
        let is_pdf = mime_type.is_some_and(|m| m.contains("pdf")) || lc.ends_with(".pdf");
        if is_md {
            Self::Markdown
        } else if is_pdf {
            Self::Pdf
        } else {
            Self::default()
        }
    }
}

/// Optional per-document chunk tuning from API.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkOptions {
    pub chunk_token_size: Option<usize>,
    pub chunk_overlap_token_size: Option<usize>,
    #[serde(default)]
    pub separators: Vec<String>,
}

impl ChunkOptions {
    /// Validate API input (max 16 separators, 8 chars each).
    pub fn validate(&self) -> Result<(), String> {
        if self.separators.len() > 16 {
            return Err("separators: max 16 entries".to_string());
        }
        for sep in &self.separators {
            if sep.chars().count() > 8 {
                return Err("separators: max 8 chars per entry".to_string());
            }
        }
        Ok(())
    }

    pub fn apply_to_config(&self, config: &mut ChunkerConfig) {
        if let Some(size) = self.chunk_token_size {
            config.chunk_size = size;
        }
        if let Some(overlap) = self.chunk_overlap_token_size {
            config.chunk_overlap = overlap;
        }
        if !self.separators.is_empty() {
            config.separators = self.separators.clone();
        }
    }
}

/// Resolve a [`Chunker`] for the given strategy and base config.
pub fn resolve_chunker(strategy: ChunkStrategy, config: ChunkerConfig) -> Chunker {
    let strategy_impl: Arc<dyn ChunkingStrategy> = match strategy {
        ChunkStrategy::Fixed => Arc::new(TokenBasedChunking),
        ChunkStrategy::Recursive => Arc::new(RecursiveCharacterChunking),
        ChunkStrategy::Markdown => Arc::new(MarkdownChunking),
        // Pdf wraps Recursive: splits at page markers first, then applies
        // the same recursive token-based strategy within each page.
        ChunkStrategy::Pdf => Arc::new(PageAwareChunking::default()),
    };
    Chunker::with_strategy(config, strategy_impl)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_selects_strategy_by_enum() {
        let config = ChunkerConfig::default();
        assert_eq!(
            resolve_chunker(ChunkStrategy::Recursive, config.clone()).strategy_name(),
            "recursive_character"
        );
        assert_eq!(
            resolve_chunker(ChunkStrategy::Fixed, config.clone()).strategy_name(),
            "token_based"
        );
        assert_eq!(
            resolve_chunker(ChunkStrategy::Markdown, config).strategy_name(),
            "markdown"
        );
    }

    #[test]
    fn default_strategy_is_recursive() {
        assert_eq!(ChunkStrategy::default(), ChunkStrategy::Recursive);
    }

    #[test]
    fn auto_selects_markdown_for_md_files() {
        assert_eq!(
            ChunkStrategy::resolve_for_upload(None, None, "readme.md"),
            ChunkStrategy::Markdown
        );
        assert_eq!(
            ChunkStrategy::resolve_for_upload(None, Some("text/markdown"), "doc"),
            ChunkStrategy::Markdown
        );
        assert_eq!(
            ChunkStrategy::resolve_for_upload(None, None, "doc.txt"),
            ChunkStrategy::Recursive
        );
        assert_eq!(
            ChunkStrategy::resolve_for_upload(None, None, "doc.txt"),
            ChunkStrategy::default()
        );
    }
}
