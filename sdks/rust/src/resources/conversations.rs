//! Conversations resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::conversations::*;

pub struct ConversationsResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> ConversationsResource<'a> {
    /// `GET /api/v1/conversations`
    pub async fn list(&self) -> Result<Vec<ConversationInfo>> {
        self.client.get("/api/v1/conversations").await
    }

    /// `POST /api/v1/conversations`
    pub async fn create(&self, req: &CreateConversationRequest) -> Result<ConversationInfo> {
        self.client.post("/api/v1/conversations", Some(req)).await
    }

    /// `GET /api/v1/conversations/{id}`
    pub async fn get(&self, id: &str) -> Result<ConversationDetail> {
        self.client
            .get(&format!("/api/v1/conversations/{id}"))
            .await
    }

    /// `DELETE /api/v1/conversations/{id}`
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!("/api/v1/conversations/{id}"))
            .await
    }

    /// `POST /api/v1/conversations/{id}/messages`
    pub async fn create_message(
        &self,
        conversation_id: &str,
        req: &CreateMessageRequest,
    ) -> Result<Message> {
        self.client
            .post(
                &format!("/api/v1/conversations/{conversation_id}/messages"),
                Some(req),
            )
            .await
    }

    /// `GET /api/v1/conversations/{id}/messages`
    pub async fn list_messages(&self, conversation_id: &str) -> Result<Vec<Message>> {
        self.client
            .get(&format!(
                "/api/v1/conversations/{conversation_id}/messages"
            ))
            .await
    }

    /// `POST /api/v1/conversations/{id}/pin`
    pub async fn pin(&self, id: &str) -> Result<()> {
        self.client
            .post_no_content::<()>(&format!("/api/v1/conversations/{id}/pin"), None)
            .await
    }

    /// `DELETE /api/v1/conversations/{id}/pin`
    pub async fn unpin(&self, id: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!("/api/v1/conversations/{id}/pin"))
            .await
    }

    /// `POST /api/v1/conversations/{id}/share`
    pub async fn share(&self, id: &str) -> Result<ShareLink> {
        self.client
            .post::<(), ShareLink>(&format!("/api/v1/conversations/{id}/share"), None)
            .await
    }

    /// `POST /api/v1/conversations/bulk/delete`
    pub async fn bulk_delete(&self, ids: &[String]) -> Result<BulkDeleteResponse> {
        let body = serde_json::json!({ "ids": ids });
        self.client
            .post("/api/v1/conversations/bulk/delete", Some(&body))
            .await
    }

    /// `POST /api/v1/conversations/import` — Import conversations.
    pub async fn import(&self, body: &serde_json::Value) -> Result<serde_json::Value> {
        self.client
            .post("/api/v1/conversations/import", Some(body))
            .await
    }

    /// `PATCH /api/v1/conversations/{id}` — Update conversation title/metadata.
    pub async fn update(
        &self,
        id: &str,
        body: &serde_json::Value,
    ) -> Result<ConversationInfo> {
        self.client
            .patch(&format!("/api/v1/conversations/{id}"), Some(body))
            .await
    }

    /// `DELETE /api/v1/conversations/{id}/share` — Unshare conversation.
    pub async fn unshare(&self, id: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!("/api/v1/conversations/{id}/share"))
            .await
    }

    /// `POST /api/v1/conversations/bulk/archive` — Bulk archive conversations.
    pub async fn bulk_archive(&self, ids: &[String]) -> Result<serde_json::Value> {
        let body = serde_json::json!({ "ids": ids });
        self.client
            .post("/api/v1/conversations/bulk/archive", Some(&body))
            .await
    }

    /// `POST /api/v1/conversations/bulk/move` — Bulk move conversations to folder.
    pub async fn bulk_move(
        &self,
        ids: &[String],
        folder_id: &str,
    ) -> Result<serde_json::Value> {
        let body = serde_json::json!({ "ids": ids, "folder_id": folder_id });
        self.client
            .post("/api/v1/conversations/bulk/move", Some(&body))
            .await
    }
}
