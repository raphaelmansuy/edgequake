//! Relationships resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::graph::*;

pub struct RelationshipsResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> RelationshipsResource<'a> {
    /// `GET /api/v1/relationships`
    pub async fn list(&self) -> Result<Vec<Relationship>> {
        self.client.get("/api/v1/relationships").await
    }

    /// `POST /api/v1/relationships`
    pub async fn create(&self, req: &CreateRelationshipRequest) -> Result<Relationship> {
        self.client.post("/api/v1/relationships", Some(req)).await
    }

    /// `DELETE /api/v1/relationships/{id}`
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!("/api/v1/relationships/{id}"))
            .await
    }
}
