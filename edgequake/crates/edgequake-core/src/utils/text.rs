//! Text processing utilities.

/// Normalize text by trimming whitespace and collapsing multiple spaces.
pub fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Truncate text to a maximum length, adding ellipsis if truncated.
pub fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else if max_len <= 3 {
        text.chars().take(max_len).collect()
    } else {
        let truncated: String = text.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}

/// Split text into sentences (simple heuristic).
pub fn split_sentences(text: &str) -> Vec<&str> {
    text.split(['.', '!', '?'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Count words in text.
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Extract the first N words from text.
pub fn first_n_words(text: &str, n: usize) -> String {
    text.split_whitespace()
        .take(n)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Clean text by removing control characters and normalizing whitespace.
pub fn clean_text(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_text() {
        assert_eq!(normalize_text("  hello   world  "), "hello world");
        assert_eq!(normalize_text("single"), "single");
    }

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("hello", 10), "hello");
        assert_eq!(truncate_text("hello world", 8), "hello...");
        assert_eq!(truncate_text("hi", 2), "hi");
    }

    #[test]
    fn test_split_sentences() {
        let sentences = split_sentences("Hello. World! How are you?");
        assert_eq!(sentences.len(), 3);
        assert_eq!(sentences[0], "Hello");
    }

    #[test]
    fn test_word_count() {
        assert_eq!(word_count("hello world"), 2);
        assert_eq!(word_count(""), 0);
        assert_eq!(word_count("   "), 0);
    }

    #[test]
    fn test_first_n_words() {
        assert_eq!(first_n_words("one two three four", 2), "one two");
        assert_eq!(first_n_words("single", 5), "single");
    }

    #[test]
    fn test_clean_text() {
        assert_eq!(clean_text("hello\x00world"), "helloworld");
        assert_eq!(clean_text("  multiple   spaces  "), "multiple spaces");
    }
}
