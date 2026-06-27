//! Lightweight Markdown IR for heading breadcrumbs (SPEC-026 Phase 2 P-03).
//!
//! Ports LightRAG `parser/markdown/extract.py` heading stack semantics without
//! full parser registry complexity.

mod breadcrumb;
mod parse;

pub use breadcrumb::{format_breadcrumb, HEADING_BREADCRUMB_SEP};
pub use parse::{extract_markdown_blocks, MarkdownBlock, PREFACE_HEADING};
