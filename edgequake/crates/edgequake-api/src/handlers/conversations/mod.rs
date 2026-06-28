//! Conversation management handlers.
//!
//! Provides REST API endpoints for managing conversations, messages, and folders.
//!
//! ## Sub-modules
//!
//! | Module     | Purpose                                       |
//! |------------|-----------------------------------------------|
//! | `crud`     | List, create, get, update, delete conversations |
//! | `messages` | Message CRUD within conversations             |
//! | `sharing`  | Share / unshare / get shared conversations    |
//! | `bulk`     | Import, bulk-delete, bulk-archive, bulk-move  |
//! | `folders`  | Folder CRUD for conversation organization     |
//!
//! ## Implements
//!
//! - **FEAT0580**: Conversation listing with pagination and filtering
//! - **FEAT0581**: Conversation creation with mode selection
//! - **FEAT0582**: Message management within conversations
//! - **FEAT0583**: Folder organization for conversation grouping
//!
//! ## Enforces
//!
//! - **BR0580**: Conversations must be scoped to authenticated user
//! - **BR0581**: Messages must have valid roles (user/assistant/system)
//! - **BR0582**: Folder names must be unique per user

mod bulk;
mod crud;
mod folders;
mod messages;
mod share_paths;
mod sharing;

pub use bulk::*;
pub use crud::*;
pub use folders::*;
pub use messages::*;
pub use share_paths::share_api_path;
pub use sharing::*;

// Re-export DTOs from conversations_types module
pub use crate::handlers::conversations_types::*;

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_list_conversations_params_defaults() {
        let json_str = r#"{}"#;
        let params: Result<ListConversationsParams, _> = serde_json::from_str(json_str);
        assert!(params.is_ok());
        let p = params.unwrap();
        assert_eq!(p.limit, 20);
        assert_eq!(p.sort, "updated_at");
        assert_eq!(p.order, "desc");
    }

    #[test]
    fn test_create_conversation_request_deserialization() {
        let json_str = r#"{"title": "Test", "mode": "hybrid"}"#;
        let request: Result<CreateConversationApiRequest, _> = serde_json::from_str(json_str);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.title, Some("Test".to_string()));
        assert_eq!(req.mode, Some("hybrid".to_string()));
    }

    #[test]
    fn test_conversation_response_serialization() {
        let response = ConversationResponse {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            workspace_id: None,
            title: "Test".to_string(),
            mode: "hybrid".to_string(),
            is_pinned: false,
            is_archived: false,
            folder_id: None,
            share_id: None,
            message_count: Some(0),
            last_message_preview: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok());
    }

    #[test]
    fn test_folder_response_serialization() {
        let response = FolderResponse {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            workspace_id: None,
            name: "Test Folder".to_string(),
            parent_id: None,
            position: 0,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok());
    }

    #[test]
    fn test_message_response_serialization() {
        let response = MessageResponse {
            id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            parent_id: None,
            role: "user".to_string(),
            content: "Hello".to_string(),
            mode: Some("hybrid".to_string()),
            tokens_used: Some(10),
            duration_ms: Some(100),
            thinking_time_ms: None,
            context: None,
            is_error: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok());
    }
}
