//! Conversations resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::conversations::*;

pub struct ConversationsResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> ConversationsResource<'a> {
    /// `GET /api/v1/conversations` — paginated (`items` + `pagination`), server defaults.
    pub async fn list(&self) -> Result<PaginatedConversations> {
        self.list_with_query(&ConversationListQuery::default()).await
    }

    /// `GET /api/v1/conversations` with cursor, `filter[…]`, `sort`, and `order` query params.
    pub async fn list_with_query(
        &self,
        query: &ConversationListQuery,
    ) -> Result<PaginatedConversations> {
        let qs = conversation_list_query_string(query);
        let path = if qs.is_empty() {
            "/api/v1/conversations".to_string()
        } else {
            format!("/api/v1/conversations?{}", qs)
        };
        self.client.get(&path).await
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

    /// `GET /api/v1/conversations/{id}/messages` — server default pagination.
    pub async fn list_messages(&self, conversation_id: &str) -> Result<PaginatedMessages> {
        self.list_messages_with_query(conversation_id, &ListMessagesQuery::default())
            .await
    }

    /// `GET /api/v1/conversations/{id}/messages` with `cursor` and `limit`.
    pub async fn list_messages_with_query(
        &self,
        conversation_id: &str,
        query: &ListMessagesQuery,
    ) -> Result<PaginatedMessages> {
        let qs = list_messages_query_string(query);
        let path = if qs.is_empty() {
            format!("/api/v1/conversations/{conversation_id}/messages")
        } else {
            format!(
                "/api/v1/conversations/{conversation_id}/messages?{}",
                qs
            )
        };
        self.client.get(&path).await
    }

    /// Pin conversation (`PATCH /api/v1/conversations/{id}` with `is_pinned: true`).
    pub async fn pin(&self, id: &str) -> Result<()> {
        let body = serde_json::json!({ "is_pinned": true });
        self.update(id, &body).await?;
        Ok(())
    }

    /// Unpin conversation (`PATCH` with `is_pinned: false`).
    pub async fn unpin(&self, id: &str) -> Result<()> {
        let body = serde_json::json!({ "is_pinned": false });
        self.update(id, &body).await?;
        Ok(())
    }

    /// `POST /api/v1/conversations/{id}/share`
    pub async fn share(&self, id: &str) -> Result<ShareLink> {
        self.client
            .post::<(), ShareLink>(&format!("/api/v1/conversations/{id}/share"), None)
            .await
    }

    /// `POST /api/v1/conversations/bulk/delete`
    pub async fn bulk_delete(&self, ids: &[String]) -> Result<BulkConversationOpResponse> {
        let body = serde_json::json!({ "conversation_ids": ids });
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
    pub async fn update(&self, id: &str, body: &serde_json::Value) -> Result<ConversationInfo> {
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

    /// `POST /api/v1/conversations/bulk/archive` — Bulk archive / unarchive.
    pub async fn bulk_archive(
        &self,
        ids: &[String],
        archive: bool,
    ) -> Result<BulkConversationOpResponse> {
        let body = serde_json::json!({ "conversation_ids": ids, "archive": archive });
        self.client
            .post("/api/v1/conversations/bulk/archive", Some(&body))
            .await
    }

    /// `POST /api/v1/conversations/bulk/move` — `folder_id` `None` clears folder per API.
    pub async fn bulk_move(
        &self,
        ids: &[String],
        folder_id: Option<&str>,
    ) -> Result<BulkConversationOpResponse> {
        let body = match folder_id {
            Some(fid) => serde_json::json!({ "conversation_ids": ids, "folder_id": fid }),
            None => serde_json::json!({ "conversation_ids": ids }),
        };
        self.client
            .post("/api/v1/conversations/bulk/move", Some(&body))
            .await
    }

    /// `PATCH /api/v1/messages/{message_id}`
    pub async fn update_message(
        &self,
        message_id: &str,
        req: &UpdateMessageRequest,
    ) -> Result<Message> {
        self.client
            .patch(
                &format!("/api/v1/messages/{message_id}"),
                Some(req),
            )
            .await
    }

    /// `DELETE /api/v1/messages/{message_id}`
    pub async fn delete_message(&self, message_id: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!("/api/v1/messages/{message_id}"))
            .await
    }
}
