//! Document schema types for block-based extraction.
//!
//! This module defines the core types for representing document structure:
//! - `BlockType`: Semantic classification of document elements
//! - `Block`: A document element with position and content
//! - `BoundingBox`: Spatial coordinates for layout analysis
//! - `Document`: Complete document representation with pages

mod block;
mod block_types;
mod document;
mod geometry;

pub use block::{Block, BlockId, FontStyle, TextSpan};
pub use block_types::BlockType;
pub use document::{Document, DocumentMetadata, ExtractionMethod, Page, PageStats, TocEntry};
pub use geometry::{BoundingBox, Point, Polygon};
