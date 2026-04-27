//! Entities resource.
//!
//! WHY: All entity endpoints live under /api/v1/graph/entities in the real API.
//! Verified against edgequake/crates/edgequake-api/src/routes.rs.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::graph::{
    BatchDegreeRequest, CreateEntityRequest, CreateEntityResponse, DegreesBatchResponse,
    EntityDetailResponse, EntityExistsResponse, EntityListResponse, MergeEntitiesRequest,
    MergeEntitiesResponse, NeighborhoodResponse,
};

pub struct EntitiesResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> EntitiesResource<'a> {
    /// `GET /api/v1/graph/entities` — paginated list.
    pub async fn list(&self) -> Result<EntityListResponse> {
        self.client.get("/api/v1/graph/entities").await
    }

    /// `GET /api/v1/graph/entities/{name}` — entity detail with relationships + statistics.
    pub async fn get(&self, name: &str) -> Result<EntityDetailResponse> {
        self.client
            .get(&format!(
                "/api/v1/graph/entities/{}",
                urlencoding::encode(name)
            ))
            .await
    }

    /// `POST /api/v1/graph/entities` — create a new entity.
    pub async fn create(&self, req: &CreateEntityRequest) -> Result<CreateEntityResponse> {
        self.client.post("/api/v1/graph/entities", Some(req)).await
    }

    /// `DELETE /api/v1/graph/entities/{name}?confirm=true`
    pub async fn delete(&self, name: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!(
                "/api/v1/graph/entities/{}?confirm=true",
                urlencoding::encode(name)
            ))
            .await
    }

    /// `GET /api/v1/graph/entities/exists?entity_name=NAME` — check existence.
    pub async fn exists(&self, name: &str) -> Result<EntityExistsResponse> {
        self.client
            .get(&format!(
                "/api/v1/graph/entities/exists?entity_name={}",
                urlencoding::encode(name)
            ))
            .await
    }

    /// `POST /api/v1/graph/entities/merge` — merge two entities.
    pub async fn merge(&self, source: &str, target: &str) -> Result<MergeEntitiesResponse> {
        let body = MergeEntitiesRequest {
            source_entity: source.to_string(),
            target_entity: target.to_string(),
        };
        self.client
            .post("/api/v1/graph/entities/merge", Some(&body))
            .await
    }

    /// `GET /api/v1/graph/entities/{name}/neighborhood`
    pub async fn neighborhood(&self, name: &str) -> Result<NeighborhoodResponse> {
        self.client
            .get(&format!(
                "/api/v1/graph/entities/{}/neighborhood",
                urlencoding::encode(name)
            ))
            .await
    }

    /// `POST /api/v1/graph/degrees/batch`
    pub async fn degrees(&self, node_ids: &[String]) -> Result<DegreesBatchResponse> {
        let body = BatchDegreeRequest {
            node_ids: node_ids.to_vec(),
        };
        self.client
            .post("/api/v1/graph/degrees/batch", Some(&body))
            .await
    }

    /// `PUT /api/v1/graph/entities/{name}` — Update entity.
    pub async fn update(&self, name: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        self.client
            .put(
                &format!("/api/v1/graph/entities/{}", urlencoding::encode(name)),
                Some(body),
            )
            .await
    }
}
