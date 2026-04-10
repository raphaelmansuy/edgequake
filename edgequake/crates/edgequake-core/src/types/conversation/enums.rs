//! Conversation and message enums.
//!
//! Defines the mode and role enumerations used across the conversation system.

use serde::{Deserialize, Serialize};

/// Conversation mode for RAG queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConversationMode {
    /// Local search only (entity-based).
    Local,
    /// Global search only (community summaries).
    Global,
    /// Hybrid search (combines local and global).
    #[default]
    Hybrid,
    /// Naive search (simple vector similarity).
    Naive,
    /// Mix mode (weighted combination).
    Mix,
}

impl std::fmt::Display for ConversationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => write!(f, "local"),
            Self::Global => write!(f, "global"),
            Self::Hybrid => write!(f, "hybrid"),
            Self::Naive => write!(f, "naive"),
            Self::Mix => write!(f, "mix"),
        }
    }
}

impl std::str::FromStr for ConversationMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "global" => Ok(Self::Global),
            "hybrid" => Ok(Self::Hybrid),
            "naive" => Ok(Self::Naive),
            "mix" => Ok(Self::Mix),
            _ => Err(format!("Unknown mode: {}", s)),
        }
    }
}

/// Message role in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// User message.
    User,
    /// Assistant response.
    Assistant,
    /// System message.
    System,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
            Self::System => write!(f, "system"),
        }
    }
}

impl std::str::FromStr for MessageRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            "system" => Ok(Self::System),
            _ => Err(format!("Unknown role: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ConversationMode ───────────────────────────────────

    #[test]
    fn test_conversation_mode_default_is_hybrid() {
        assert_eq!(ConversationMode::default(), ConversationMode::Hybrid);
    }

    #[test]
    fn test_conversation_mode_display_roundtrip() {
        let modes = [
            ConversationMode::Local,
            ConversationMode::Global,
            ConversationMode::Hybrid,
            ConversationMode::Naive,
            ConversationMode::Mix,
        ];
        for mode in &modes {
            let s = mode.to_string();
            let parsed: ConversationMode = s.parse().unwrap();
            assert_eq!(*mode, parsed);
        }
    }

    #[test]
    fn test_conversation_mode_from_str_case_insensitive() {
        assert_eq!("LOCAL".parse::<ConversationMode>().unwrap(), ConversationMode::Local);
        assert_eq!("Hybrid".parse::<ConversationMode>().unwrap(), ConversationMode::Hybrid);
    }

    #[test]
    fn test_conversation_mode_from_str_error() {
        let result = "unknown".parse::<ConversationMode>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown mode"));
    }

    // ── MessageRole ────────────────────────────────────────

    #[test]
    fn test_message_role_display_roundtrip() {
        let roles = [MessageRole::User, MessageRole::Assistant, MessageRole::System];
        for role in &roles {
            let s = role.to_string();
            let parsed: MessageRole = s.parse().unwrap();
            assert_eq!(*role, parsed);
        }
    }

    #[test]
    fn test_message_role_from_str_error() {
        let result = "bot".parse::<MessageRole>();
        assert!(result.is_err());
    }
}
