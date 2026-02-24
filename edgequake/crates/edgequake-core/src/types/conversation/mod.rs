//! Conversation and message types for query history persistence.
//!
//! This module defines the domain entities for managing conversations and
//! messages in the EdgeQuake query interface.
//!
//! ## Sub-modules
//!
//! | Module     | Responsibility                                      |
//! |------------|-----------------------------------------------------|
//! | `enums`    | ConversationMode, MessageRole + Display/FromStr      |
//! | `models`   | Conversation, Message, Folder structs + builders     |
//! | `context`  | MessageContext, entities, relationships, sources      |
//! | `requests` | Request DTOs, filters, pagination, import types       |

mod context;
mod enums;
mod models;
mod requests;

pub use context::{
    MessageContext, MessageContextEntity, MessageContextRelationship, MessageSource,
};
pub use enums::{ConversationMode, MessageRole};
pub use models::{Conversation, Folder, Message};
pub use requests::{
    ConversationFilter, ConversationSortField, CreateConversationRequest, CreateFolderRequest,
    CreateMessageRequest, ImportError, ImportResult, PaginatedConversations, PaginatedMessages,
    PaginationMeta, UpdateConversationRequest, UpdateFolderRequest, UpdateMessageRequest,
};

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_conversation_mode_display() {
        assert_eq!(ConversationMode::Local.to_string(), "local");
        assert_eq!(ConversationMode::Global.to_string(), "global");
        assert_eq!(ConversationMode::Hybrid.to_string(), "hybrid");
        assert_eq!(ConversationMode::Naive.to_string(), "naive");
        assert_eq!(ConversationMode::Mix.to_string(), "mix");
    }

    #[test]
    fn test_conversation_mode_parse() {
        assert_eq!(
            "local".parse::<ConversationMode>().unwrap(),
            ConversationMode::Local
        );
        assert_eq!(
            "HYBRID".parse::<ConversationMode>().unwrap(),
            ConversationMode::Hybrid
        );
        assert!("invalid".parse::<ConversationMode>().is_err());
    }

    #[test]
    fn test_message_role_display() {
        assert_eq!(MessageRole::User.to_string(), "user");
        assert_eq!(MessageRole::Assistant.to_string(), "assistant");
        assert_eq!(MessageRole::System.to_string(), "system");
    }

    #[test]
    fn test_conversation_builder() {
        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        let conv = Conversation::new(tenant_id, user_id)
            .with_title("Test Chat")
            .with_mode(ConversationMode::Local)
            .with_workspace(workspace_id);

        assert_eq!(conv.title, "Test Chat");
        assert_eq!(conv.mode, ConversationMode::Local);
        assert_eq!(conv.workspace_id, Some(workspace_id));
        assert_eq!(conv.tenant_id, tenant_id);
        assert_eq!(conv.user_id, user_id);
    }

    #[test]
    fn test_message_builder() {
        let conversation_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();

        let msg = Message::user(conversation_id, "Hello, world!")
            .with_parent(parent_id)
            .with_mode(ConversationMode::Hybrid);

        assert_eq!(msg.content, "Hello, world!");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.parent_id, Some(parent_id));
        assert_eq!(msg.mode, Some(ConversationMode::Hybrid));
    }

    #[test]
    fn test_folder_builder() {
        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();

        let folder = Folder::new(tenant_id, user_id, "My Folder")
            .with_parent(parent_id)
            .with_position(5);

        assert_eq!(folder.name, "My Folder");
        assert_eq!(folder.parent_id, Some(parent_id));
        assert_eq!(folder.position, 5);
    }
}
