//! Auto-generation of conversation titles from the first user message.
//!
//! Uses the workspace-resolved LLM provider to generate a concise, meaningful
//! title (3-8 words) from the first user message in a conversation.
//!
//! ## Implements
//!
//! - **FEAT0505**: Auto-generated conversation titles from first message
//!
//! ## Use Cases
//!
//! - **UC2105**: System auto-generates conversation title on first message
//!
//! ## Enforces
//!
//! - **BR0505**: Title generation must not block chat response
//! - **BR0506**: Fallback to truncated first message on LLM failure

use std::sync::Arc;
use tracing::{debug, warn};

use edgequake_llm::traits::{ChatMessage, CompletionOptions, LLMProvider};

/// System prompt for title generation.
const TITLE_SYSTEM_PROMPT: &str = "\
Generate a short, concise title (3-8 words) that captures the main topic \
of the following user message. Return ONLY the title text. \
Do not use quotes, prefixes, or explanations.";

/// Maximum characters of the user message to send for title generation.
const MAX_MESSAGE_CHARS: usize = 500;

/// Maximum length of the generated title in characters.
const MAX_TITLE_LENGTH: usize = 80;

/// Generate a conversation title from the first user message using the LLM.
///
/// Returns a concise title (3-8 words). Falls back to truncated message
/// if the LLM call fails.
///
/// # Arguments
///
/// * `llm_provider` - The resolved LLM provider to use for generation
/// * `first_message` - The first user message content
///
/// # Returns
///
/// A `String` with the generated title (always succeeds due to fallback).
pub async fn generate_title(llm_provider: Arc<dyn LLMProvider>, first_message: &str) -> String {
    // Truncate the message to avoid sending excessive content
    let truncated_message: String = first_message.chars().take(MAX_MESSAGE_CHARS).collect();

    let messages = vec![
        ChatMessage::system(TITLE_SYSTEM_PROMPT),
        ChatMessage::user(&truncated_message),
    ];

    let options = CompletionOptions {
        max_tokens: Some(30),
        temperature: Some(0.3),
        ..Default::default()
    };

    match llm_provider.chat(&messages, Some(&options)).await {
        Ok(response) => {
            let title = clean_title(&response.content);
            if title.is_empty() {
                debug!("LLM returned empty title, using fallback");
                fallback_title(first_message)
            } else {
                debug!(title = %title, "Generated conversation title");
                title
            }
        }
        Err(e) => {
            warn!(error = %e, "Failed to generate title via LLM, using fallback");
            fallback_title(first_message)
        }
    }
}

/// Clean up the LLM-generated title.
///
/// Removes surrounding quotes, trims whitespace, and limits length.
fn clean_title(raw: &str) -> String {
    let mut title = raw.trim().to_string();

    // Remove surrounding quotes (single, double, or backticks)
    for quote in ['"', '\'', '`'] {
        if title.starts_with(quote) && title.ends_with(quote) && title.len() > 1 {
            title = title[1..title.len() - 1].to_string();
        }
    }

    // Remove common prefixes the LLM might add
    for prefix in ["Title:", "title:", "TITLE:"] {
        if let Some(stripped) = title.strip_prefix(prefix) {
            title = stripped.trim().to_string();
        }
    }

    // Limit length
    if title.chars().count() > MAX_TITLE_LENGTH {
        title = title.chars().take(MAX_TITLE_LENGTH).collect::<String>() + "...";
    }

    title.trim().to_string()
}

/// Fallback title from the first message (truncated to 50 chars).
fn fallback_title(message: &str) -> String {
    let truncated: String = message.chars().take(50).collect();
    if truncated.chars().count() < message.chars().count() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_title_removes_double_quotes() {
        assert_eq!(clean_title("\"My Title\""), "My Title");
    }

    #[test]
    fn test_clean_title_removes_single_quotes() {
        assert_eq!(clean_title("'My Title'"), "My Title");
    }

    #[test]
    fn test_clean_title_removes_backticks() {
        assert_eq!(clean_title("`My Title`"), "My Title");
    }

    #[test]
    fn test_clean_title_removes_prefix() {
        assert_eq!(clean_title("Title: My Title"), "My Title");
        assert_eq!(clean_title("title: My Title"), "My Title");
        assert_eq!(clean_title("TITLE: My Title"), "My Title");
    }

    #[test]
    fn test_clean_title_trims_whitespace() {
        assert_eq!(clean_title("  My Title  "), "My Title");
    }

    #[test]
    fn test_clean_title_limits_length() {
        let long = "a".repeat(100);
        let result = clean_title(&long);
        // MAX_TITLE_LENGTH chars + "..."
        assert!(result.chars().count() <= MAX_TITLE_LENGTH + 3);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_clean_title_preserves_normal_title() {
        assert_eq!(
            clean_title("Graph Database Architecture"),
            "Graph Database Architecture"
        );
    }

    #[test]
    fn test_clean_title_empty_returns_empty() {
        assert_eq!(clean_title(""), "");
        assert_eq!(clean_title("   "), "");
    }

    #[test]
    fn test_fallback_title_short_message() {
        assert_eq!(fallback_title("short message"), "short message");
    }

    #[test]
    fn test_fallback_title_long_message() {
        let long = "a".repeat(100);
        let result = fallback_title(&long);
        assert!(result.ends_with("..."));
        assert_eq!(result.chars().count(), 53); // 50 + "..."
    }

    #[test]
    fn test_fallback_title_exactly_50_chars() {
        let exact = "a".repeat(50);
        let result = fallback_title(&exact);
        assert_eq!(result, exact); // No ellipsis
    }

    #[test]
    fn test_fallback_title_unicode() {
        let unicode = "Qu'est-ce que la recherche augmentée par graphe de connaissances?";
        let result = fallback_title(unicode);
        assert!(result.chars().count() <= 53);
    }
}
