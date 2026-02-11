//! Relationships resource.
//!
//! WHY: All relationship endpoints live under /api/v1/graph/relationships.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::graph::*;

pub struct RelationshipsResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> RelationshipsResource<'a> {
    /// `GET /api/v1/graph/relationships` — paginated list.
    pub async fn list(&self) -> Result<RelationshipListResponse> {
        self.client.get("/api/v1/graph/relationships").await
    }

    /// `POST /api/v1/graph/relationships`
    pub async fn create(&self, req: &CreateRelationshipRequest) -> Result<Relationship> {
        self.client
            .post("/api/v1/graph/relationships", Some(req))
            .await
    }

    /// `DELETE /api/v1/graph/relationships/{id}`
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!("/api/v1/graph/relationships/{id}"))
            .await
    }
}
