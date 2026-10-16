//! Load recent conversation history for chat completions (DRY / SOLID).
//!
//! Single responsibility: map persisted messages → engine `ConversationMessage`
//! **recent pool**. History *policy* (window + token budget + pair-safe) lives
//! only in `edgequake_query::conversation_context` (2026 AI eng SSOT).

use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use edgequake_core::conversation_service::ConversationService;
use edgequake_query::ConversationMessage;

/// Raw recent pool before policy (enough for token-budget trim of long turns).
///
/// Fetched with a backward index scan. One extra row covers the just-saved
/// user message, which this loader drops before the pool is trimmed.
const RECENT_HISTORY_POOL: usize = 64;

/// Load prior turns for a conversation, excluding the just-saved user message.
///
/// Returns the recent chronological pool (not yet token-trimmed). The engine
/// applies [`edgequake_query::conversation_context::apply_history_policy`].
pub async fn load_recent_conversation_history(
    conversation_service: &dyn ConversationService,
    conversation_id: Uuid,
    exclude_message_id: Uuid,
) -> ApiResult<Vec<ConversationMessage>> {
    let page = conversation_service
        .list_recent_messages(conversation_id, RECENT_HISTORY_POOL + 1)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to load conversation history: {e}")))?;

    let mut prior: Vec<ConversationMessage> = page
        .into_iter()
        .filter(|m| m.message_id != exclude_message_id)
        .filter(|m| !m.content.trim().is_empty())
        .map(|m| ConversationMessage {
            role: m.role.to_string(),
            content: m.content,
        })
        .collect();

    // ASC list → keep recent tail only (First Principles: model needs recency).
    if prior.len() > RECENT_HISTORY_POOL {
        prior = prior.split_off(prior.len() - RECENT_HISTORY_POOL);
    }

    Ok(prior)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_pool_is_larger_than_default_window() {
        const {
            assert!(RECENT_HISTORY_POOL >= 16);
            assert!(RECENT_HISTORY_POOL < 10_000);
        }
    }
}
