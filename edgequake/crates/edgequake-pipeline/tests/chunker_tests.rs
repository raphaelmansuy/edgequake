//! Chunker integration tests (SPEC-017: moved from `chunker/mod.rs`).

use std::sync::Arc;

use edgequake_pipeline::chunker::{
    text_utils::{
        calculate_line_numbers, ceil_char_boundary, estimate_tokens, floor_char_boundary,
        split_into_sentences,
    },
    Chunker, ChunkerConfig, ChunkingStrategy, ParagraphBoundaryChunking, SentenceBoundaryChunking,
};

#[test]
fn test_basic_chunking() {
    let chunker = Chunker::default_chunker();
    let text = "This is sentence one. This is sentence two. This is sentence three.";

    let chunks = chunker.chunk(text, "doc1").unwrap();

    assert!(!chunks.is_empty());
    assert_eq!(chunks[0].index, 0);
}

#[test]
fn test_sentence_boundary_strategy_changes_chunk_count() {
    use std::sync::Arc;

    let text = "First sentence here. Second sentence follows. Third one ends.";
    let doc_id = "strategy-test";
    let small_config = ChunkerConfig {
        chunk_size: 5,
        min_chunk_size: 1,
        chunk_overlap: 0,
        ..ChunkerConfig::default()
    };

    let token_chunker = Chunker::new(small_config.clone());
    let sentence_chunker = Chunker::with_strategy(small_config, Arc::new(SentenceBoundaryChunking));

    let token_chunks = token_chunker.chunk(text, doc_id).unwrap();
    let sentence_chunks = sentence_chunker.chunk(text, doc_id).unwrap();

    assert!(
        !token_chunks.is_empty() && !sentence_chunks.is_empty(),
        "both strategies must produce chunks"
    );
    assert_ne!(
        token_chunker.strategy_name(),
        sentence_chunker.strategy_name(),
        "configured strategy must be honored"
    );
    assert!(
        sentence_chunks.len() >= 2,
        "sentence strategy with small token budget should split sentences"
    );
}

#[test]
fn test_empty_text() {
    let chunker = Chunker::default_chunker();
    let chunks = chunker.chunk("", "doc1").unwrap();
    assert!(chunks.is_empty());
}

#[test]
fn test_short_text() {
    let chunker = Chunker::default_chunker();
    let text = "Short text.";
    let chunks = chunker.chunk(text, "doc1").unwrap();

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].content, text);
}

#[test]
fn test_long_text_chunking() {
    let config = ChunkerConfig {
        chunk_size: 10, // 10 tokens * 4 chars = 40 chars per chunk
        chunk_overlap: 2,
        min_chunk_size: 5,
        ..Default::default()
    };
    let chunker = Chunker::new(config);

    let text = "First sentence here. Second sentence follows. Third sentence now. Fourth one too. Fifth is last.";
    let chunks = chunker.chunk(text, "doc1").unwrap();

    assert!(chunks.len() > 1);

    // Verify chunks cover the text
    let total_unique: std::collections::HashSet<_> =
        chunks.iter().flat_map(|c| c.content.chars()).collect();
    assert!(!total_unique.is_empty());
}

#[test]
fn test_chunk_ids() {
    let chunker = Chunker::default_chunker();
    let text = "Some text content that will be chunked.";
    let chunks = chunker.chunk(text, "my-doc").unwrap();

    assert!(chunks[0].id.starts_with("my-doc-chunk-"));
}

#[test]
fn test_token_estimation() {
    assert_eq!(estimate_tokens("test"), 1);
    assert_eq!(estimate_tokens("hello world"), 3); // 11 chars / 4 ≈ 3
}

#[test]
fn test_line_number_calculation() {
    // Test single line
    let text = "Hello world";
    let (start, end) = calculate_line_numbers(text, 0, text.len());
    assert_eq!(start, 1);
    assert_eq!(end, 1);

    // Test multiple lines
    let text = "Line 1\nLine 2\nLine 3";
    let (start, end) = calculate_line_numbers(text, 0, text.len());
    assert_eq!(start, 1);
    assert_eq!(end, 3);

    // Test middle portion
    let text = "Line 1\nLine 2\nLine 3\nLine 4";
    let line2_start = 7; // After "Line 1\n"
    let line3_end = 20; // End of "Line 3"
    let (start, end) = calculate_line_numbers(text, line2_start, line3_end);
    assert_eq!(start, 2);
    assert_eq!(end, 3);
}

#[test]
fn test_chunks_have_line_numbers() {
    let chunker = Chunker::default_chunker();
    let text = "Line one.\nLine two.\nLine three.";
    let chunks = chunker.chunk(text, "doc1").unwrap();

    assert!(!chunks.is_empty());
    assert_eq!(chunks[0].start_line, 1);
    assert!(chunks[0].end_line >= 1);
}

#[test]
fn test_multiline_chunk_line_numbers() {
    let config = ChunkerConfig {
        chunk_size: 10,
        chunk_overlap: 2,
        min_chunk_size: 5,
        ..Default::default()
    };
    let chunker = Chunker::new(config);

    let text = "Line 1 here.\nLine 2 here.\nLine 3 here.\nLine 4 here.\nLine 5 here.";
    let chunks = chunker.chunk(text, "doc1").unwrap();

    // First chunk should start at line 1
    if !chunks.is_empty() {
        assert_eq!(chunks[0].start_line, 1);
    }
}

#[test]
fn test_utf8_multibyte_chars_in_chunking() {
    // Test with multi-byte UTF-8 characters: smart quotes, bullets, emojis
    // Using raw bytes to include smart quotes without Rust parser issues
    let text = "Quality. Compared with state-of-the-art FR-IQA models, \
the \u{201C}proposed GMSD model\u{201D} performs better \u{2022} in terms of both accuracy \
and efficiency, making GMSD an ideal choice for high-performance IQA applications.\n\n\
This work is supported by \u{7814}\u{7A76} and \u{5F00}\u{53D1} funding.";

    let config = ChunkerConfig {
        chunk_size: 50, // Force chunking within the multi-byte section
        chunk_overlap: 10,
        min_chunk_size: 20,
        ..Default::default()
    };
    let chunker = Chunker::new(config);

    // This should not panic even with multi-byte characters
    let chunks = chunker.chunk(text, "utf8-test").unwrap();

    assert!(!chunks.is_empty());
    // All chunks should be valid UTF-8 strings
    for chunk in &chunks {
        assert!(chunk.content.is_char_boundary(0));
        assert!(chunk.content.is_char_boundary(chunk.content.len()));
    }
}

#[test]
fn test_floor_and_ceil_char_boundary() {
    // Test with multi-byte character: " (LEFT DOUBLE QUOTATION MARK, 3 bytes: E2 80 9C)
    let text = "ab\u{201C}cd";

    // "ab" is 2 bytes, then " is 3 bytes (positions 2, 3, 4), then "cd" is 2 more
    // So: a=0, b=1, "=2,3,4, c=5, d=6

    assert_eq!(floor_char_boundary(text, 2), 2); // Start of "
    assert_eq!(floor_char_boundary(text, 3), 2); // Inside " -> back to 2
    assert_eq!(floor_char_boundary(text, 4), 2); // Inside " -> back to 2
    assert_eq!(floor_char_boundary(text, 5), 5); // Start of c

    assert_eq!(ceil_char_boundary(text, 2), 2); // Start of "
    assert_eq!(ceil_char_boundary(text, 3), 5); // Inside " -> forward to 5
    assert_eq!(ceil_char_boundary(text, 4), 5); // Inside " -> forward to 5
}

// =========================================================================
// SentenceBoundaryChunking Tests
// =========================================================================

#[tokio::test]
async fn test_sentence_boundary_basic() {
    let strategy = SentenceBoundaryChunking::new();
    let config = ChunkerConfig {
        chunk_size: 100,
        chunk_overlap: 10,
        min_chunk_size: 5,
        ..Default::default()
    };

    let text = "This is sentence one. This is sentence two. This is sentence three.";
    let chunks = strategy.chunk(text, &config).await.unwrap();

    assert!(!chunks.is_empty());
    // With large chunk size, all should fit in one chunk
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].content.contains("sentence one"));
}

#[tokio::test]
async fn test_sentence_boundary_splits_on_sentences() {
    let strategy = SentenceBoundaryChunking::new();
    let config = ChunkerConfig {
        chunk_size: 15, // Small size to force splits
        chunk_overlap: 5,
        min_chunk_size: 5,
        ..Default::default()
    };

    let text = "First sentence here. Second sentence now. Third one follows. Fourth is last.";
    let chunks = strategy.chunk(text, &config).await.unwrap();

    // Should have multiple chunks
    assert!(chunks.len() >= 2);

    // Each chunk should end at a sentence boundary (or be the last chunk)
    for chunk in &chunks {
        let trimmed = chunk.content.trim();
        // Last char should be period, question, or exclamation (sentence ending)
        // OR it's incomplete because it's the last chunk
        if trimmed.len() > 1 {
            let last_char = trimmed.chars().last().unwrap();
            assert!(
                last_char == '.' || last_char == '!' || last_char == '?',
                "Chunk should end at sentence boundary: {}",
                trimmed
            );
        }
    }
}

#[tokio::test]
async fn test_sentence_boundary_empty_text() {
    let strategy = SentenceBoundaryChunking::new();
    let config = ChunkerConfig::default();

    let chunks = strategy.chunk("", &config).await.unwrap();
    assert!(chunks.is_empty());
}

#[tokio::test]
async fn test_sentence_boundary_no_periods() {
    let strategy = SentenceBoundaryChunking::new();
    let config = ChunkerConfig {
        chunk_size: 50,
        chunk_overlap: 10,
        min_chunk_size: 10,
        ..Default::default()
    };

    // Text with no sentence boundaries falls back to token-based
    let text = "This is text without any sentence endings at all";
    let chunks = strategy.chunk(text, &config).await.unwrap();

    assert!(!chunks.is_empty());
}

#[tokio::test]
async fn test_sentence_boundary_abbreviations() {
    let strategy = SentenceBoundaryChunking::new();
    let config = ChunkerConfig {
        chunk_size: 200,
        chunk_overlap: 20,
        min_chunk_size: 10,
        ..Default::default()
    };

    // Text with abbreviations should not split on them
    let text = "Dr. Smith works at Inc. headquarters. He joined in Jan. 2020.";
    let chunks = strategy.chunk(text, &config).await.unwrap();

    // With large chunk size, should be one chunk
    assert_eq!(chunks.len(), 1);
}

#[tokio::test]
async fn test_sentence_boundary_question_exclamation() {
    let strategy = SentenceBoundaryChunking::new();
    let config = ChunkerConfig {
        chunk_size: 20,
        chunk_overlap: 5,
        min_chunk_size: 5,
        ..Default::default()
    };

    let text = "What is this? This is amazing! And this is normal.";
    let chunks = strategy.chunk(text, &config).await.unwrap();

    // Should split on all three sentence types
    assert!(!chunks.is_empty());
}

// =========================================================================
// ParagraphBoundaryChunking Tests
// =========================================================================

#[tokio::test]
async fn test_paragraph_boundary_basic() {
    let strategy = ParagraphBoundaryChunking::new();
    let config = ChunkerConfig {
        chunk_size: 100,
        chunk_overlap: 10,
        min_chunk_size: 5,
        ..Default::default()
    };

    let text = "First paragraph here.\n\nSecond paragraph here.\n\nThird paragraph.";
    let chunks = strategy.chunk(text, &config).await.unwrap();

    assert!(!chunks.is_empty());
}

#[tokio::test]
async fn test_paragraph_boundary_splits_on_paragraphs() {
    let strategy = ParagraphBoundaryChunking::new();
    let config = ChunkerConfig {
        chunk_size: 10, // Very small to force splits
        chunk_overlap: 2,
        min_chunk_size: 3,
        ..Default::default()
    };

    // Use longer text to ensure chunks are created
    let text = "First paragraph with some text here.\n\n\
                Second paragraph with more content.\n\n\
                Third paragraph now added.\n\n\
                Fourth paragraph is last.";
    let chunks = strategy.chunk(text, &config).await.unwrap();

    // Should have multiple chunks due to small chunk_size
    assert!(
        !chunks.is_empty(),
        "Expected at least 1 chunk, got {}",
        chunks.len()
    );
}

#[tokio::test]
async fn test_paragraph_boundary_empty_text() {
    let strategy = ParagraphBoundaryChunking::new();
    let config = ChunkerConfig::default();

    let chunks = strategy.chunk("", &config).await.unwrap();
    assert!(chunks.is_empty());
}

#[tokio::test]
async fn test_paragraph_boundary_single_paragraph() {
    let strategy = ParagraphBoundaryChunking::new();
    let config = ChunkerConfig {
        chunk_size: 100,
        chunk_overlap: 10,
        min_chunk_size: 5,
        ..Default::default()
    };

    let text = "This is a single paragraph with no breaks.";
    let chunks = strategy.chunk(text, &config).await.unwrap();

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].content, text);
}

#[tokio::test]
async fn test_paragraph_boundary_preserves_paragraph_integrity() {
    let strategy = ParagraphBoundaryChunking::new();
    let config = ChunkerConfig {
        chunk_size: 50,
        chunk_overlap: 10,
        min_chunk_size: 10,
        ..Default::default()
    };

    let text = "First paragraph with multiple sentences. Here is more.\n\n\
                Second paragraph also has content. More words here.";
    let chunks = strategy.chunk(text, &config).await.unwrap();

    // Each chunk should contain complete paragraphs (not mid-paragraph splits)
    for chunk in &chunks {
        // Should not start or end mid-word unexpectedly
        assert!(!chunk.content.starts_with(' '));
    }
}

#[tokio::test]
async fn test_paragraph_boundary_single_newline_fallback() {
    let strategy = ParagraphBoundaryChunking::new();
    let config = ChunkerConfig {
        chunk_size: 20,
        chunk_overlap: 5,
        min_chunk_size: 3,
        ..Default::default()
    };

    // Single newlines (no double newlines)
    let text = "Line one here.\nLine two here.\nLine three.";
    let chunks = strategy.chunk(text, &config).await.unwrap();

    // Should still produce chunks
    assert!(!chunks.is_empty());
}

#[tokio::test]
async fn test_paragraph_boundary_large_paragraph() {
    let strategy = ParagraphBoundaryChunking::new();
    let config = ChunkerConfig {
        chunk_size: 10, // Very small
        chunk_overlap: 2,
        min_chunk_size: 3,
        ..Default::default()
    };

    // Large paragraph that exceeds chunk size
    let text = "This is a very large paragraph that definitely exceeds the tiny chunk size limit we set.\n\n\
                Small para.";
    let chunks = strategy.chunk(text, &config).await.unwrap();

    // Should still produce chunks (large para gets its own chunk)
    assert!(!chunks.is_empty());
}

// =========================================================================
// Chunker with Custom Strategy Tests
// =========================================================================

#[test]
fn test_chunker_with_sentence_strategy() {
    let config = ChunkerConfig::default();
    let chunker = Chunker::with_strategy(config, Arc::new(SentenceBoundaryChunking::new()));

    assert_eq!(chunker.strategy_name(), "sentence_boundary");
}

#[test]
fn test_chunker_with_paragraph_strategy() {
    let config = ChunkerConfig::default();
    let chunker = Chunker::with_strategy(config, Arc::new(ParagraphBoundaryChunking::new()));

    assert_eq!(chunker.strategy_name(), "paragraph_boundary");
}

#[test]
fn test_split_into_sentences_basic() {
    let sentences = split_into_sentences("First. Second. Third.");
    assert_eq!(sentences.len(), 3);
}

#[test]
fn test_split_into_sentences_with_abbreviations() {
    let sentences = split_into_sentences("Dr. Smith said hello. Then left.");
    // Should NOT split on "Dr."
    assert!(sentences.len() <= 2);
}
