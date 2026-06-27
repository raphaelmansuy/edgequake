//! Multi-turn conversation formatting for query prompts (SPEC-025 5.1).

use crate::types::ConversationMessage;

/// Default number of prior turns included in prompts / keyword extraction.
pub const DEFAULT_CONVERSATION_TURN_LIMIT: usize = 6;

/// Build a query string that includes recent conversation for keyword extraction.
pub fn query_with_conversation_context(
    query: &str,
    history: &[ConversationMessage],
    max_turns: usize,
) -> String {
    let Some(section) = format_conversation_history(history, max_turns) else {
        return query.to_string();
    };
    format!("{section}\n\nCurrent question: {query}")
}

/// Format recent conversation turns for injection into RAG prompts.
pub fn format_conversation_history(
    history: &[ConversationMessage],
    max_turns: usize,
) -> Option<String> {
    if history.is_empty() || max_turns == 0 {
        return None;
    }

    let start = history.len().saturating_sub(max_turns);
    let mut lines = Vec::with_capacity(max_turns.min(history.len()));
    for message in &history[start..] {
        let role = message.role.trim();
        let content = message.content.trim();
        if content.is_empty() {
            continue;
        }
        let label = match role.to_ascii_lowercase().as_str() {
            "assistant" => "Assistant",
            "system" => "System",
            _ => "User",
        };
        lines.push(format!("{label}: {content}"));
    }

    if lines.is_empty() {
        None
    } else {
        Some(format!(
            "---Recent Conversation---\n{}\n---End Conversation---",
            lines.join("\n")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_history_returns_none() {
        assert!(format_conversation_history(&[], 6).is_none());
    }

    #[test]
    fn formats_recent_turns_only() {
        let history: Vec<_> = (0..8)
            .map(|i| ConversationMessage {
                role: if i % 2 == 0 {
                    "user".into()
                } else {
                    "assistant".into()
                },
                content: format!("turn-{i}"),
            })
            .collect();
        let formatted = format_conversation_history(&history, 2).expect("section");
        assert!(formatted.contains("turn-6"));
        assert!(formatted.contains("turn-7"));
        assert!(!formatted.contains("turn-0"));
    }

    #[test]
    fn query_context_includes_current_question() {
        let history = vec![ConversationMessage {
            role: "user".into(),
            content: "What is EdgeQuake?".into(),
        }];
        let q = query_with_conversation_context("Tell me more", &history, 6);
        assert!(q.contains("What is EdgeQuake?"));
        assert!(q.contains("Current question: Tell me more"));
    }
}
