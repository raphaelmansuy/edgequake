//! Multi-turn conversation formatting for query prompts (SPEC-025 5.1).
//!
//! Chat / Bypass mode (FEAT0106) reuses the same history policy so RAG and
//! pure-chatbot paths stay DRY: one cut strategy, two prompt shapes.
//!
//! # AI Engineering (2026) — history policy
//!
//! 1. **Pin system prompt** — never trimmed (persona/rules always present).
//! 2. **Sliding window** — keep recent turns for high-volume chat.
//! 3. **Token budget** — drop oldest **user+assistant pairs** until under budget
//!    (message-count alone mis-budgets long turns).
//! 4. **Pair-safe** — never leave an orphaned assistant at the front.

use crate::types::ConversationMessage;
use edgequake_llm::traits::{ChatMessage, ImageData};

/// Default max prior **messages** in the sliding window (before token trim).
/// ~3 user/assistant turns; common 2026 starting range is 5–10 turns.
pub const DEFAULT_CONVERSATION_TURN_LIMIT: usize = 6;

/// Soft token budget for conversation history alone (excludes system + current user).
/// Heuristic ~4 chars/token; leaves headroom for system + reply on typical 8k–128k models.
pub const DEFAULT_HISTORY_TOKEN_BUDGET: usize = 3_000;

/// Per-message overhead used by provider chat formats (roles, separators).
const PER_MESSAGE_TOKEN_OVERHEAD: usize = 4;

/// Default system prompt for Bypass / Chat mode (pure chatbot, no KG/RAG).
///
/// WHY (First Principles): Bypass means "talk to the LLM directly". The RAG
/// expert prompt is wrong here — empty context is intentional, not a miss.
/// This persona must survive every history cut (system is never trimmed).
pub const DEFAULT_BYPASS_SYSTEM_PROMPT: &str = "\
You are a helpful, concise chatbot assistant. Answer the user's questions \
directly using the conversation so far. Do not invent knowledge-graph or \
document citations. If you lack information, say so clearly.";

/// Resolve the Bypass/Chat system prompt.
///
/// When `extension` is set (SPEC-004), it is appended to the default persona
/// rather than replacing it — same extend semantics as RAG system prompts.
pub fn resolve_bypass_system_prompt(extension: Option<&str>) -> String {
    match extension.map(str::trim).filter(|s| !s.is_empty()) {
        Some(ext) => format!("{DEFAULT_BYPASS_SYSTEM_PROMPT}\n\n{ext}"),
        None => DEFAULT_BYPASS_SYSTEM_PROMPT.to_string(),
    }
}

/// Rough token estimate (~4 chars/token, floor at word count) — DRY with SimpleTokenizer.
pub fn estimate_message_tokens(content: &str) -> usize {
    let char_estimate = content.len().div_ceil(4);
    let word_count = content.split_whitespace().count();
    char_estimate
        .max(word_count)
        .saturating_add(PER_MESSAGE_TOKEN_OVERHEAD)
}

fn role_is_assistant(role: &str) -> bool {
    role.trim().eq_ignore_ascii_case("assistant")
}

/// Drop a leading orphaned assistant so history never starts mid-turn (pair-safe).
pub fn drop_leading_orphan_assistant(
    mut history: Vec<ConversationMessage>,
) -> Vec<ConversationMessage> {
    while history.first().is_some_and(|m| role_is_assistant(&m.role)) {
        history.remove(0);
    }
    history
}

/// Sliding-window cut: keep the last `max_messages`, then pair-safe the front.
///
/// Full transcript stays in storage; this only shapes the per-request view.
pub fn cut_conversation_history(
    history: &[ConversationMessage],
    max_messages: usize,
) -> Vec<ConversationMessage> {
    if history.is_empty() || max_messages == 0 {
        return Vec::new();
    }

    let start = history.len().saturating_sub(max_messages);
    let window: Vec<ConversationMessage> = history[start..]
        .iter()
        .filter(|m| !m.content.trim().is_empty())
        .cloned()
        .collect();
    drop_leading_orphan_assistant(window)
}

/// Token-budget trim: drop oldest messages in **pairs** until under budget.
///
/// Never splits a user→assistant exchange. System-role history rows (rare) are
/// treated like user turns for pairing purposes.
pub fn trim_history_to_token_budget(
    history: &[ConversationMessage],
    token_budget: usize,
) -> Vec<ConversationMessage> {
    if history.is_empty() || token_budget == 0 {
        return Vec::new();
    }

    let mut rest: Vec<ConversationMessage> = history
        .iter()
        .filter(|m| !m.content.trim().is_empty())
        .cloned()
        .collect();

    let mut total: usize = rest
        .iter()
        .map(|m| estimate_message_tokens(&m.content))
        .sum();

    while !rest.is_empty() && total > token_budget {
        let removed = rest.remove(0);
        total = total.saturating_sub(estimate_message_tokens(&removed.content));
        // Pair-safe: if next is assistant, drop it with the user turn.
        if rest.first().is_some_and(|m| role_is_assistant(&m.role)) {
            let orphan = rest.remove(0);
            total = total.saturating_sub(estimate_message_tokens(&orphan.content));
        }
    }

    drop_leading_orphan_assistant(rest)
}

/// Apply 2026 history policy: message window → token budget → pair-safe.
pub fn apply_history_policy(
    history: &[ConversationMessage],
    max_messages: usize,
    token_budget: usize,
) -> Vec<ConversationMessage> {
    let windowed = cut_conversation_history(history, max_messages);
    trim_history_to_token_budget(&windowed, token_budget)
}

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
    let recent = apply_history_policy(history, max_turns, DEFAULT_HISTORY_TOKEN_BUDGET);
    if recent.is_empty() {
        return None;
    }

    let mut lines = Vec::with_capacity(recent.len());
    for message in &recent {
        let role = message.role.trim();
        let content = message.content.trim();
        let label = match role.to_ascii_lowercase().as_str() {
            "assistant" => "Assistant",
            "system" => "System",
            _ => "User",
        };
        lines.push(format!("{label}: {content}"));
    }

    Some(format!(
        "---Recent Conversation---\n{}\n---End Conversation---",
        lines.join("\n")
    ))
}

/// Build OpenAI-style chat messages for Bypass / Chat mode.
///
/// Shape: `[system] + policy(history) + [user(current)]` — system is pinned;
/// history uses sliding window + token budget + pair-safe trim (2026).
pub fn build_bypass_chat_messages(
    query: &str,
    history: &[ConversationMessage],
    system_prompt_extension: Option<&str>,
    max_messages: usize,
    images: Option<&[ImageData]>,
) -> Vec<ChatMessage> {
    build_bypass_chat_messages_with_budget(
        query,
        history,
        system_prompt_extension,
        max_messages,
        DEFAULT_HISTORY_TOKEN_BUDGET,
        images,
    )
}

/// Same as [`build_bypass_chat_messages`] with an explicit history token budget.
pub fn build_bypass_chat_messages_with_budget(
    query: &str,
    history: &[ConversationMessage],
    system_prompt_extension: Option<&str>,
    max_messages: usize,
    token_budget: usize,
    images: Option<&[ImageData]>,
) -> Vec<ChatMessage> {
    let system = resolve_bypass_system_prompt(system_prompt_extension);
    let prior = apply_history_policy(history, max_messages, token_budget);
    let mut messages = Vec::with_capacity(2 + prior.len());
    messages.push(ChatMessage::system(system));

    for msg in prior {
        let content = msg.content.trim();
        if content.is_empty() {
            continue;
        }
        match msg.role.trim().to_ascii_lowercase().as_str() {
            "assistant" => messages.push(ChatMessage::assistant(content)),
            "system" => {
                // Extra system turns from history are folded into a user note
                // so we keep a single leading system message (provider-safe / cacheable prefix).
                messages.push(ChatMessage::user(format!(
                    "[Earlier instruction]\n{content}"
                )));
            }
            _ => messages.push(ChatMessage::user(content)),
        }
    }

    let user_text = query.trim();
    if let Some(imgs) = images.filter(|i| !i.is_empty()) {
        messages.push(ChatMessage::user_with_images(user_text, imgs.to_vec()));
    } else {
        messages.push(ChatMessage::user(user_text));
    }

    messages
}

/// Flatten chat messages for Langfuse Complete I/O (LAW-145-1).
///
/// Text only — image binaries are noted as a count so observation I/O stays
/// UTF-8 and does not omit the textual LLM turn content.
pub fn format_chat_messages_for_observation(messages: &[ChatMessage]) -> String {
    use edgequake_llm::traits::ChatRole;
    edgequake_observability::format_llm_chat_turns_for_observation(messages.iter().map(|m| {
        let label = match m.role {
            ChatRole::System => "System",
            ChatRole::Assistant => "Assistant",
            ChatRole::User => "User",
            ChatRole::Tool => "Tool",
            ChatRole::Function => "Function",
        };
        let image_count = m.images.as_ref().map(|i| i.len()).unwrap_or(0);
        (label, m.content.as_str(), image_count)
    }))
}

/// Flatten bypass chat messages to a single prompt for providers that only
/// expose text `stream()` (preserves roles; used for true token streaming).
pub fn format_bypass_messages_as_prompt(messages: &[ChatMessage]) -> String {
    let mut prompt = format_chat_messages_for_observation(messages);
    if !prompt.is_empty() {
        prompt.push_str("\n\n");
    }
    prompt.push_str("Assistant:");
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_history_returns_none() {
        assert!(format_conversation_history(&[], 6).is_none());
        assert!(cut_conversation_history(&[], 6).is_empty());
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
    fn cut_keeps_last_n_messages() {
        let history: Vec<_> = (0..10)
            .map(|i| ConversationMessage {
                role: "user".into(),
                content: format!("m-{i}"),
            })
            .collect();
        let cut = cut_conversation_history(&history, DEFAULT_CONVERSATION_TURN_LIMIT);
        assert_eq!(cut.len(), 6);
        assert_eq!(cut[0].content, "m-4");
        assert_eq!(cut[5].content, "m-9");
    }

    #[test]
    fn cut_drops_leading_orphan_assistant() {
        let history = vec![
            ConversationMessage {
                role: "assistant".into(),
                content: "orphan".into(),
            },
            ConversationMessage {
                role: "user".into(),
                content: "hi".into(),
            },
            ConversationMessage {
                role: "assistant".into(),
                content: "hello".into(),
            },
        ];
        let cut = cut_conversation_history(&history, 3);
        assert_eq!(cut[0].role, "user");
        assert!(!cut.iter().any(|m| m.content == "orphan"));
    }

    #[test]
    fn token_budget_drops_oldest_pairs() {
        let history = vec![
            ConversationMessage {
                role: "user".into(),
                content: "AAAA".repeat(200), // ~200 tokens
            },
            ConversationMessage {
                role: "assistant".into(),
                content: "BBBB".repeat(200),
            },
            ConversationMessage {
                role: "user".into(),
                content: "keep-me".into(),
            },
            ConversationMessage {
                role: "assistant".into(),
                content: "keep-too".into(),
            },
        ];
        let trimmed = trim_history_to_token_budget(&history, 80);
        assert!(trimmed.iter().any(|m| m.content == "keep-me"));
        assert!(trimmed.iter().any(|m| m.content == "keep-too"));
        assert!(!trimmed.iter().any(|m| m.content.starts_with("AAAA")));
        assert_eq!(trimmed[0].role, "user");
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

    #[test]
    fn bypass_system_prompt_extends_default() {
        let resolved = resolve_bypass_system_prompt(Some("Always reply in French."));
        assert!(resolved.starts_with(DEFAULT_BYPASS_SYSTEM_PROMPT));
        assert!(resolved.contains("Always reply in French."));
        assert_eq!(
            resolve_bypass_system_prompt(None),
            DEFAULT_BYPASS_SYSTEM_PROMPT
        );
        assert_eq!(
            resolve_bypass_system_prompt(Some("   ")),
            DEFAULT_BYPASS_SYSTEM_PROMPT
        );
    }

    #[test]
    fn bypass_chat_messages_are_chatbot_shaped() {
        let history = vec![
            ConversationMessage {
                role: "user".into(),
                content: "My name is Ada.".into(),
            },
            ConversationMessage {
                role: "assistant".into(),
                content: "Nice to meet you, Ada.".into(),
            },
            ConversationMessage {
                role: "user".into(),
                content: "old-1".into(),
            },
            ConversationMessage {
                role: "assistant".into(),
                content: "old-2".into(),
            },
            ConversationMessage {
                role: "user".into(),
                content: "old-3".into(),
            },
            ConversationMessage {
                role: "assistant".into(),
                content: "old-4".into(),
            },
            ConversationMessage {
                role: "user".into(),
                content: "old-5".into(),
            },
            ConversationMessage {
                role: "assistant".into(),
                content: "old-6".into(),
            },
        ];
        let messages = build_bypass_chat_messages("What is my name?", &history, None, 4, None);
        assert_eq!(messages[0].role, edgequake_llm::traits::ChatRole::System);
        assert!(messages[0]
            .content
            .contains("helpful, concise chatbot assistant"));
        assert_eq!(messages.len(), 1 + 4 + 1);
        assert_eq!(messages.last().unwrap().content, "What is my name?");
        assert!(!messages
            .iter()
            .any(|m| m.content.contains("My name is Ada")));
        assert!(messages.iter().any(|m| m.content == "old-5"));
    }

    #[test]
    fn bypass_prompt_flatten_keeps_roles() {
        let msgs = build_bypass_chat_messages(
            "Hi",
            &[ConversationMessage {
                role: "user".into(),
                content: "Prior".into(),
            }],
            None,
            6,
            None,
        );
        let prompt = format_bypass_messages_as_prompt(&msgs);
        assert!(prompt.contains("System:"));
        assert!(prompt.contains("User: Prior"));
        assert!(prompt.contains("User: Hi"));
        assert!(prompt.ends_with("Assistant:"));
    }
}
