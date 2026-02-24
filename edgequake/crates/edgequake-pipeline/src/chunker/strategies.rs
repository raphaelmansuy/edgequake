//! Chunking strategy implementations.
//!
//! Provides multiple strategies for splitting text into chunks:
//! - [`TokenBasedChunking`]: Default, token-count-based with overlap
//! - [`CharacterBasedChunking`][]: Character-count-based
//! - [`SentenceBoundaryChunking`]: Sentence-aware splitting
//! - [`ParagraphBoundaryChunking`]: Paragraph-aware splitting

use async_trait::async_trait;

use super::text_utils::{
    estimate_tokens, split_into_sentences, split_text_internal, take_overlap_sentences,
};
use super::types::{ChunkResult, ChunkerConfig, ChunkingStrategy};
use crate::error::Result;

/// Default token-based chunking strategy.
///
/// This is the standard chunking strategy that splits text into chunks
/// based on token count with overlap, respecting sentence boundaries.
pub struct TokenBasedChunking;

#[async_trait]
impl ChunkingStrategy for TokenBasedChunking {
    async fn chunk(&self, content: &str, config: &ChunkerConfig) -> Result<Vec<ChunkResult>> {
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        // Check for split_by_character_only mode (GAP-017)
        if let Some(ref split_char) = config.split_by_character {
            if config.split_by_character_only {
                return Ok(content
                    .split(split_char.as_str())
                    .enumerate()
                    .filter(|(_, s)| !s.trim().is_empty())
                    .map(|(idx, s)| ChunkResult {
                        content: s.to_string(),
                        tokens: estimate_tokens(s),
                        chunk_order_index: idx,
                    })
                    .collect());
            }
        }

        let target_chars = config.chunk_size * 4;
        let overlap_chars = config.chunk_overlap * 4;
        let min_chars = config.min_chunk_size * 4;

        let chunks = split_text_internal(
            content,
            target_chars,
            overlap_chars,
            min_chars,
            &config.separators,
        );

        Ok(chunks
            .into_iter()
            .enumerate()
            .map(
                |(idx, (text, _, _)): (usize, (String, usize, usize))| ChunkResult {
                    content: text.clone(),
                    tokens: estimate_tokens(&text),
                    chunk_order_index: idx,
                },
            )
            .collect())
    }

    fn name(&self) -> &str {
        "token_based"
    }
}

/// Character-based chunking strategy (GAP-017).
///
/// Splits text on a specific character (like newline) for pre-split content.
///
/// @implements FEAT0306 (Character-Based Chunking - CharacterBasedChunking struct)
pub struct CharacterBasedChunking {
    /// Character to split on.
    pub split_character: String,
}

impl CharacterBasedChunking {
    /// Create a new character-based chunking strategy.
    pub fn new(split_character: impl Into<String>) -> Self {
        Self {
            split_character: split_character.into(),
        }
    }

    /// Create a newline-based chunker.
    pub fn by_newline() -> Self {
        Self::new("\n")
    }

    /// Create a paragraph-based chunker.
    pub fn by_paragraph() -> Self {
        Self::new("\n\n")
    }
}

#[async_trait]
impl ChunkingStrategy for CharacterBasedChunking {
    async fn chunk(&self, content: &str, _config: &ChunkerConfig) -> Result<Vec<ChunkResult>> {
        Ok(content
            .split(&self.split_character)
            .enumerate()
            .filter(|(_, s)| !s.trim().is_empty())
            .map(|(idx, s)| ChunkResult {
                content: s.to_string(),
                tokens: estimate_tokens(s),
                chunk_order_index: idx,
            })
            .collect())
    }

    fn name(&self) -> &str {
        "character_based"
    }
}

/// Sentence boundary chunking strategy.
///
/// @implements SPEC-001/Issue-10: Pluggable chunk cutoff system
///
/// This strategy ensures chunks never split mid-sentence, preserving
/// complete sentences for better entity extraction context.
///
/// # Algorithm
///
/// 1. Split text into sentences using period/question/exclamation
/// 2. Accumulate sentences until target chunk size reached
/// 3. Create chunk and start new accumulation
/// 4. Overlap is handled by carrying last N sentences to next chunk
///
/// # WHY Sentence Boundaries?
///
/// Mid-sentence splits can break entity extraction context:
/// - "Dr. Smith works at Microsoft. He" → Entity "He" orphaned
/// - "Dr. Smith works at Microsoft." → Complete context preserved
pub struct SentenceBoundaryChunking;

impl SentenceBoundaryChunking {
    /// Create a new sentence boundary chunker.
    pub fn new() -> Self {
        Self
    }
}

impl Default for SentenceBoundaryChunking {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChunkingStrategy for SentenceBoundaryChunking {
    async fn chunk(&self, content: &str, config: &ChunkerConfig) -> Result<Vec<ChunkResult>> {
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        // Split into sentences (simple heuristic: period, question, exclamation)
        let sentences = split_into_sentences(content);

        if sentences.is_empty() {
            // No sentence boundaries found, fall back to token-based
            return TokenBasedChunking.chunk(content, config).await;
        }

        let target_tokens = config.chunk_size;
        let overlap_tokens = config.chunk_overlap;
        let min_tokens = config.min_chunk_size;

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut current_tokens = 0;
        let mut sentence_buffer: Vec<String> = Vec::new();
        let mut chunk_index = 0;

        for sentence in sentences {
            let sentence_tokens = estimate_tokens(&sentence);

            // If adding this sentence would exceed target, finalize current chunk
            if current_tokens + sentence_tokens > target_tokens && current_tokens >= min_tokens {
                chunks.push(ChunkResult {
                    content: current_chunk.trim().to_string(),
                    tokens: current_tokens,
                    chunk_order_index: chunk_index,
                });
                chunk_index += 1;

                // Start new chunk with overlap (carry some sentences)
                let overlap_sentences = take_overlap_sentences(&sentence_buffer, overlap_tokens);
                current_chunk = overlap_sentences.join(" ");
                current_tokens = estimate_tokens(&current_chunk);
                sentence_buffer.clear();
            }

            // Add sentence to current chunk
            if !current_chunk.is_empty() {
                current_chunk.push(' ');
            }
            current_chunk.push_str(&sentence);
            current_tokens += sentence_tokens;
            sentence_buffer.push(sentence);
        }

        // Add final chunk if non-empty
        if current_tokens >= min_tokens {
            chunks.push(ChunkResult {
                content: current_chunk.trim().to_string(),
                tokens: current_tokens,
                chunk_order_index: chunk_index,
            });
        }

        Ok(chunks)
    }

    fn name(&self) -> &str {
        "sentence_boundary"
    }
}

/// Paragraph boundary chunking strategy.
///
/// @implements SPEC-001/Issue-10: Pluggable chunk cutoff system
///
/// This strategy groups paragraphs together, never splitting within
/// a paragraph. Ideal for structured documents.
///
/// # Algorithm
///
/// 1. Split text on double newlines (paragraphs)
/// 2. Accumulate paragraphs until target chunk size reached
/// 3. Create chunk and start new accumulation
///
/// # WHY Paragraph Boundaries?
///
/// Paragraphs often contain self-contained ideas:
/// - Entity introductions usually complete within paragraph
/// - Relationships described in same paragraph as entities
/// - Splitting preserves narrative flow
pub struct ParagraphBoundaryChunking;

impl ParagraphBoundaryChunking {
    /// Create a new paragraph boundary chunker.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ParagraphBoundaryChunking {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChunkingStrategy for ParagraphBoundaryChunking {
    async fn chunk(&self, content: &str, config: &ChunkerConfig) -> Result<Vec<ChunkResult>> {
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        // Split on double newlines (paragraphs)
        let paragraphs: Vec<&str> = content
            .split("\n\n")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if paragraphs.is_empty() {
            // No paragraphs found, try single newlines
            let single_line_paragraphs: Vec<&str> = content
                .split('\n')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            if single_line_paragraphs.is_empty() {
                return TokenBasedChunking.chunk(content, config).await;
            }
            return chunk_paragraphs(&single_line_paragraphs, config);
        }

        chunk_paragraphs(&paragraphs, config)
    }

    fn name(&self) -> &str {
        "paragraph_boundary"
    }
}

/// Helper to chunk paragraphs into size-limited chunks.
fn chunk_paragraphs(paragraphs: &[&str], config: &ChunkerConfig) -> Result<Vec<ChunkResult>> {
    let target_tokens = config.chunk_size;
    let min_tokens = config.min_chunk_size;

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_tokens = 0;
    let mut chunk_index = 0;

    for para in paragraphs {
        let para_tokens = estimate_tokens(para);

        // If this paragraph alone exceeds target, add it as its own chunk
        if para_tokens >= target_tokens {
            // First, save current accumulation if any
            if current_tokens >= min_tokens {
                chunks.push(ChunkResult {
                    content: current_chunk.trim().to_string(),
                    tokens: current_tokens,
                    chunk_order_index: chunk_index,
                });
                chunk_index += 1;
                current_chunk = String::new();
                current_tokens = 0;
            }

            // Add large paragraph as its own chunk
            chunks.push(ChunkResult {
                content: para.to_string(),
                tokens: para_tokens,
                chunk_order_index: chunk_index,
            });
            chunk_index += 1;
            continue;
        }

        // If adding would exceed target, finalize current chunk
        if current_tokens + para_tokens > target_tokens && current_tokens >= min_tokens {
            chunks.push(ChunkResult {
                content: current_chunk.trim().to_string(),
                tokens: current_tokens,
                chunk_order_index: chunk_index,
            });
            chunk_index += 1;
            current_chunk = String::new();
            current_tokens = 0;
        }

        // Add paragraph to current chunk
        if !current_chunk.is_empty() {
            current_chunk.push_str("\n\n");
        }
        current_chunk.push_str(para);
        current_tokens += para_tokens;
    }

    // Add final chunk
    if current_tokens >= min_tokens {
        chunks.push(ChunkResult {
            content: current_chunk.trim().to_string(),
            tokens: current_tokens,
            chunk_order_index: chunk_index,
        });
    }

    Ok(chunks)
}
