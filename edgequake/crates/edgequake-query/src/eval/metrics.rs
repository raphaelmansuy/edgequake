//! Lightweight RAG quality metrics (RAGAS-inspired, no external deps).

/// Fraction of expected keywords found in `text` (case-insensitive substring match).
pub fn keyword_recall_in_text(text: &str, keywords: &[String]) -> f32 {
    if keywords.is_empty() {
        return 1.0;
    }
    let lower = text.to_lowercase();
    let hits = keywords
        .iter()
        .filter(|k| lower.contains(&k.to_lowercase()))
        .count();
    hits as f32 / keywords.len() as f32
}

/// Fraction of expected entity names present in retrieved context entity list.
pub fn context_entity_recall(retrieved: &[String], expected: &[String]) -> f32 {
    if expected.is_empty() {
        return 1.0;
    }
    let set: std::collections::HashSet<_> = retrieved.iter().map(|s| s.as_str()).collect();
    let hits = expected.iter().filter(|e| set.contains(e.as_str())).count();
    hits as f32 / expected.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_recall_partial_match() {
        let score = keyword_recall_in_text(
            "Rust is a systems programming language",
            &["rust".to_string(), "python".to_string()],
        );
        assert!((score - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn context_entity_recall_full() {
        let score = context_entity_recall(
            &["ALPHA".to_string(), "BETA".to_string()],
            &["ALPHA".to_string()],
        );
        assert!((score - 1.0).abs() < f32::EPSILON);
    }
}
