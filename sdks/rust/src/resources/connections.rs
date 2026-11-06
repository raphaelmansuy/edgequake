//! Connections resource.
//!
//! Admin routes for saved LLM provider endpoints (SPEC-163).

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::connections::{Connection, ProbeResult, ProviderTestRequest, UpsertConnection};

pub struct ConnectionsResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> ConnectionsResource<'a> {
    /// `GET /api/v1/connections`
    pub async fn list(&self) -> Result<Vec<Connection>> {
        self.client.get("/api/v1/connections").await
    }

    /// `POST /api/v1/connections?scope=deployment`
    pub async fn create(&self, body: &UpsertConnection) -> Result<Connection> {
        self.client
            .post("/api/v1/connections?scope=deployment", Some(body))
            .await
    }

    /// `PUT /api/v1/connections/{id}?scope=deployment`
    pub async fn update(&self, id: &str, body: &UpsertConnection) -> Result<Connection> {
        self.client
            .put(
                &format!("/api/v1/connections/{id}?scope=deployment"),
                Some(body),
            )
            .await
    }

    /// `DELETE /api/v1/connections/{id}?scope=deployment`
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!("/api/v1/connections/{id}?scope=deployment"))
            .await
    }

    /// `POST /api/v1/connections/{id}/test?scope=deployment`
    pub async fn test(&self, id: &str) -> Result<ProbeResult> {
        self.client
            .post(
                &format!("/api/v1/connections/{id}/test?scope=deployment"),
                None::<&()>,
            )
            .await
    }

    /// `POST /api/v1/providers/test`
    pub async fn test_provider(&self, body: &ProviderTestRequest) -> Result<ProbeResult> {
        self.client.post("/api/v1/providers/test", Some(body)).await
    }
}
