//! Text utility functions for chunking operations.
//!
//! Provides string splitting, UTF-8 boundary handling, and sentence detection
//! used by the chunker and its strategies.

/// Calculate line numbers for a chunk based on character offsets.
///
/// # Arguments
/// * `full_text` - The complete document text
/// * `start_offset` - Starting character offset of the chunk
/// * `end_offset` - Ending character offset of the chunk
///
/// # Returns
/// A tuple of (start_line, end_line), both 1-based
pub fn calculate_line_numbers(
    full_text: &str,
    start_offset: usize,
    end_offset: usize,
) -> (usize, usize) {
    // Ensure offsets are on valid char boundaries
    let safe_start = floor_char_boundary(full_text, start_offset.min(full_text.len()));
    let safe_end = floor_char_boundary(full_text, end_offset.min(full_text.len()));

    // Count newlines before the start offset to get start line
    let before_chunk = &full_text[..safe_start];
    let start_line = before_chunk.chars().filter(|&c| c == '\n').count() + 1;

    // Count newlines within the chunk to get end line
    let chunk_text = &full_text[safe_start..safe_end];
    let lines_in_chunk = chunk_text.chars().filter(|&c| c == '\n').count();
    let end_line = start_line + lines_in_chunk;

    (start_line, end_line)
}

/// Estimate token count (rough approximation: 1 token ≈ 4 chars).
pub(super) fn estimate_tokens(text: &str) -> usize {
    (text.len() as f32 / 4.0).ceil() as usize
}

/// Split text into sentences using simple heuristics.
///
/// WHY: Avoids splitting on common abbreviations (Dr., Mr., Inc., etc.)
/// while still detecting sentence boundaries at '.', '!', '?' characters.
pub(super) fn split_into_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();

    for c in text.chars() {
        current.push(c);

        // Check for sentence endings (simple heuristic)
        if c == '.' || c == '!' || c == '?' {
            // Avoid splitting on abbreviations like "Dr." "Mr." "Inc."
            let trimmed = current.trim();
            if trimmed.len() >= 3 {
                // Check if previous word is an abbreviation
                let words: Vec<&str> = trimmed.split_whitespace().collect();
                if let Some(last_word) = words.last() {
                    let abbrevs = [
                        "Dr.", "Mr.", "Mrs.", "Ms.", "Jr.", "Sr.", "Inc.", "Ltd.", "etc.", "vs.",
                        "e.g.", "i.e.", "No.", "St.",
                    ];
                    if !abbrevs.contains(last_word) {
                        sentences.push(current.trim().to_string());
                        current = String::new();
                    }
                }
            }
        }
    }

    // Don't forget trailing text without sentence ending
    if !current.trim().is_empty() {
        sentences.push(current.trim().to_string());
    }

    sentences
}

/// Take sentences from buffer to achieve approximately target overlap tokens.
pub(super) fn take_overlap_sentences(buffer: &[String], target_tokens: usize) -> Vec<String> {
    let mut overlap = Vec::new();
    let mut tokens = 0;

    // Take from end of buffer
    for sentence in buffer.iter().rev() {
        let sentence_tokens = estimate_tokens(sentence);
        if tokens + sentence_tokens > target_tokens && !overlap.is_empty() {
            break;
        }
        overlap.insert(0, sentence.clone());
        tokens += sentence_tokens;
    }

    overlap
}

/// Find the nearest valid UTF-8 char boundary at or before the given byte position.
pub(super) fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    // Walk backwards to find a valid char boundary
    let mut i = index;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Find the nearest valid UTF-8 char boundary at or after the given byte position.
pub(super) fn ceil_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    // Walk forward to find a valid char boundary
    let mut i = index;
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

/// Split text into chunks with overlap, respecting separator boundaries.
///
/// Returns a vector of (content, start_offset, end_offset) tuples.
pub(super) fn split_text_internal(
    text: &str,
    target_size: usize,
    overlap: usize,
    min_size: usize,
    separators: &[String],
) -> Vec<(String, usize, usize)> {
    if text.len() <= target_size {
        return vec![(text.to_string(), 0, text.len())];
    }

    let mut chunks = Vec::new();
    let mut current_pos = 0;

    while current_pos < text.len() {
        // Ensure current_pos is on a char boundary
        current_pos = ceil_char_boundary(text, current_pos);

        let remaining = &text[current_pos..];

        if remaining.len() <= target_size {
            chunks.push((remaining.to_string(), current_pos, text.len()));
            break;
        }

        // Calculate end position, ensuring it's on a char boundary
        let end_pos = floor_char_boundary(text, current_pos + target_size);
        let chunk_text = &text[current_pos..end_pos.min(text.len())];

        let split_point = find_split_point_internal(chunk_text, target_size, separators);
        // Ensure actual_end is on a char boundary
        let actual_end = floor_char_boundary(text, current_pos + split_point);

        let chunk_content = text[current_pos..actual_end].to_string();

        if chunk_content.len() >= min_size {
            chunks.push((chunk_content, current_pos, actual_end));
        }

        // Calculate overlap position, ensuring it's on a char boundary
        let overlap_pos = actual_end.saturating_sub(overlap);
        current_pos = ceil_char_boundary(text, overlap_pos);

        if current_pos >= actual_end {
            current_pos = actual_end;
        }
    }

    chunks
}

/// Find the best split point near the target size using separator hierarchy.
pub(super) fn find_split_point_internal(text: &str, target: usize, separators: &[String]) -> usize {
    // Ensure search boundaries are on valid char boundaries
    let search_start = floor_char_boundary(text, target.saturating_sub(target / 4));
    let search_end = floor_char_boundary(text, target.min(text.len()));

    // Only search if we have a valid range
    if search_start >= search_end {
        return floor_char_boundary(text, target.min(text.len()));
    }

    for separator in separators {
        if let Some(pos) = text[search_start..search_end].rfind(separator.as_str()) {
            return search_start + pos + separator.len();
        }
    }

    floor_char_boundary(text, target.min(text.len()))
}
