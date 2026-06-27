//! Adaptive chunk sizing for large documents (LightRAG best practice).
//!
//! SSOT for library (`EdgeQuake::insert`) and HTTP worker ingestion paths (SPEC-025 5.2).

/// Recommended chunk size in tokens from document byte length.
///
/// Thresholds (from LightRAG empirical testing):
/// - `<50KB` → 1200 tokens
/// - `50–100KB` → 800 tokens
/// - `>100KB` → 600 tokens
pub fn calculate_adaptive_chunk_size(document_size_bytes: usize) -> usize {
    if document_size_bytes > 100_000 {
        600
    } else if document_size_bytes > 50_000 {
        800
    } else {
        1200
    }
}

/// Overlap as ~8.3% of chunk size (LightRAG best practice).
pub fn adaptive_chunk_overlap(chunk_size: usize) -> usize {
    (chunk_size as f32 * 0.083) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_sizes_match_lightrag_thresholds() {
        assert_eq!(calculate_adaptive_chunk_size(30_000), 1200);
        assert_eq!(calculate_adaptive_chunk_size(80_000), 800);
        assert_eq!(calculate_adaptive_chunk_size(200_000), 600);
    }

    #[test]
    fn overlap_is_proportional_to_chunk_size() {
        assert_eq!(adaptive_chunk_overlap(1200), 99);
        assert_eq!(adaptive_chunk_overlap(600), 49);
    }
}
