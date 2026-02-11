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
}
