//! Entities resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::graph::*;

pub struct EntitiesResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> EntitiesResource<'a> {
    /// `GET /api/v1/entities`
    pub async fn list(&self) -> Result<Vec<Entity>> {
        self.client.get("/api/v1/entities").await
    }

    /// `GET /api/v1/entities/{name}`
    pub async fn get(&self, name: &str) -> Result<Entity> {
        self.client
            .get(&format!("/api/v1/entities/{}", urlencoding::encode(name)))
            .await
    }

    /// `POST /api/v1/entities`
    pub async fn create(&self, req: &CreateEntityRequest) -> Result<Entity> {
        self.client.post("/api/v1/entities", Some(req)).await
    }

    /// `DELETE /api/v1/entities/{name}`
    pub async fn delete(&self, name: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!("/api/v1/entities/{}", urlencoding::encode(name)))
            .await
    }

    /// `HEAD /api/v1/entities/{name}` — check existence.
    pub async fn exists(&self, name: &str) -> Result<EntityExistsResponse> {
        self.client
            .get(&format!("/api/v1/entities/{}/exists", urlencoding::encode(name)))
            .await
    }

    /// `POST /api/v1/entities/merge` — merge two entities.
    pub async fn merge(&self, source: &str, target: &str) -> Result<MergeEntitiesResponse> {
        let body = MergeEntitiesRequest {
            source: source.to_string(),
            target: target.to_string(),
        };
        self.client
            .post("/api/v1/entities/merge", Some(&body))
            .await
    }

    /// `GET /api/v1/entities/{name}/neighborhood`
    pub async fn neighborhood(&self, name: &str) -> Result<NeighborhoodResponse> {
        self.client
            .get(&format!(
                "/api/v1/entities/{}/neighborhood",
                urlencoding::encode(name)
            ))
            .await
    }

    /// `POST /api/v1/entities/degrees`
    pub async fn degrees(&self, names: &[String]) -> Result<DegreesBatchResponse> {
        self.client
            .post("/api/v1/entities/degrees", Some(&names))
            .await
    }
}
