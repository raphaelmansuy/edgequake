//! Text chunking with overlap for document processing.
//!
//! @implements FEAT0002
//! @implements FEAT0301
//! @implements FEAT0302
//!
//! # Implements
//!
//! - **FEAT0002**: Text Chunking with Overlap
//! - **FEAT0301**: Character-Based Chunking
//! - **FEAT0302**: Token-Based Chunking
//!
//! # Enforces
//!
//! - **BR0002**: Chunk size 800 tokens, overlap 100 tokens (default config)
//!   Reduced from 1200 to prevent "input length exceeds context length" errors
//!   on embedding models with 2048-token limits (e.g., embeddinggemma) when
//!   processing dense scientific text (tables, formulas) where actual token
//!   density is 2–3× higher than the 4 chars/token estimation.
//!
//! # WHY: Overlapping Chunks
//!
//! Overlap between chunks ensures:
//! 1. Context continuity across chunk boundaries
//! 2. Entity mentions spanning two chunks are captured
//! 3. Better retrieval for queries at chunk boundaries
//!
//! The default 100-token overlap (~8% of chunk size) balances:
//! - Coverage (entities not missed)
//! - Efficiency (minimal duplicate processing)
//!
//! This module provides flexible text chunking with support for custom chunking functions.
//! Users can implement the `ChunkingStrategy` trait to provide their own chunking logic.
//!
//! # Architecture
//!
//! - `types`: Core data types (ChunkResult, ChunkerConfig, TextChunk, ChunkingStrategy trait)
//! - [`text_utils`]: String splitting, UTF-8 boundary, sentence detection utilities
//! - `strategies`: Chunking strategy implementations (token, character, sentence, paragraph)

mod markdown_chunking;
mod page_aware;
mod recursive;
pub mod registry;
mod strategies;
pub mod text_utils;
mod types;

use std::sync::Arc;

use crate::error::Result;

// Re-export types
pub use registry::{resolve_chunker, ChunkOptions, ChunkStrategy};
pub use types::{
    make_page_marker, parse_page_marker, ChunkResult, ChunkerConfig, ChunkingStrategy,
    SectionMetadata, TextChunk, PAGE_MARKER_PREFIX, PAGE_MARKER_SUFFIX,
};

// Re-export text utilities needed by external consumers
pub use text_utils::calculate_line_numbers;

// Re-export strategies
pub use markdown_chunking::MarkdownChunking;
pub use page_aware::{split_into_page_segments, PageAwareChunking};
pub use recursive::{default_recursive_separators, RecursiveCharacterChunking};
pub use strategies::{
    CharacterBasedChunking, ParagraphBoundaryChunking, SentenceBoundaryChunking, TokenBasedChunking,
};

/// Text chunker for splitting documents.
pub struct Chunker {
    config: ChunkerConfig,
    strategy: Arc<dyn ChunkingStrategy>,
}

impl Chunker {
    /// Create a new chunker with the given configuration.
    pub fn new(config: ChunkerConfig) -> Self {
        Self {
            config,
            strategy: Arc::new(TokenBasedChunking),
        }
    }

    /// Create a new chunker with a custom chunking strategy.
    pub fn with_strategy(config: ChunkerConfig, strategy: Arc<dyn ChunkingStrategy>) -> Self {
        Self { config, strategy }
    }

    /// Create a chunker with default configuration.
    pub fn default_chunker() -> Self {
        Self::new(ChunkerConfig::default())
    }

    /// Create a chunker that splits by character only.
    pub fn character_chunker(split_character: impl Into<String>) -> Self {
        let config = ChunkerConfig {
            split_by_character: Some(split_character.into()),
            split_by_character_only: true,
            ..ChunkerConfig::default()
        };
        Self {
            config,
            strategy: Arc::new(CharacterBasedChunking::by_newline()),
        }
    }

    /// Chunk text into overlapping segments using the configured [`ChunkingStrategy`].
    ///
    /// Sync entry point for tests and legacy callers. Production async pipeline
    /// should prefer [`chunk_async`](Self::chunk_async) to avoid nested runtime issues.
    pub fn chunk(&self, text: &str, doc_id: &str) -> Result<Vec<TextChunk>> {
        futures::executor::block_on(self.chunk_async(text, doc_id))
    }

    /// Chunk text asynchronously using the configured strategy.
    pub async fn chunk_async(&self, text: &str, doc_id: &str) -> Result<Vec<TextChunk>> {
        let results = self.strategy.chunk(text, &self.config).await?;

        // Track cumulative offset for line number calculation
        let mut cumulative_offset = 0;

        Ok(results
            .into_iter()
            .map(|result| {
                let id = edgequake_storage::kv_keys::doc_chunk(doc_id, result.chunk_order_index);
                let (start_offset, end_offset) = match (result.start_offset, result.end_offset) {
                    (Some(start), Some(end)) => (start, end),
                    _ => {
                        let start = cumulative_offset;
                        let end = cumulative_offset + result.content.len();
                        cumulative_offset = end;
                        (start, end)
                    }
                };
                let (start_line, end_line) = calculate_line_numbers(text, start_offset, end_offset);

                TextChunk::with_line_numbers(
                    id,
                    result.content.clone(),
                    result.chunk_order_index,
                    start_offset,
                    end_offset,
                    start_line,
                    end_line,
                )
                .with_section(result.section)
                .with_page_opt(result.page_start)
            })
            .collect())
    }

    /// Find the best split point near the target size.
    #[allow(dead_code)]
    fn find_split_point(&self, text: &str, target: usize) -> usize {
        text_utils::find_split_point_internal(text, target, &self.config.separators)
    }

    /// Get the chunker configuration.
    pub fn config(&self) -> &ChunkerConfig {
        &self.config
    }

    /// Get the chunking strategy name.
    pub fn strategy_name(&self) -> &str {
        self.strategy.name()
    }
}

impl Default for Chunker {
    fn default() -> Self {
        Self {
            config: ChunkerConfig::default(),
            strategy: Arc::new(TokenBasedChunking),
        }
    }
}
