//! Tenants resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::auth::*;

pub struct TenantsResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> TenantsResource<'a> {
    /// `GET /api/v1/tenants` — returns wrapped list.
    pub async fn list(&self) -> Result<TenantListResponse> {
        self.client.get("/api/v1/tenants").await
    }

    /// `POST /api/v1/tenants`
    pub async fn create(&self, req: &CreateTenantRequest) -> Result<TenantInfo> {
        self.client.post("/api/v1/tenants", Some(req)).await
    }

    /// `GET /api/v1/tenants/{id}`
    pub async fn get(&self, id: &str) -> Result<TenantInfo> {
        self.client.get(&format!("/api/v1/tenants/{id}")).await
    }

    /// `DELETE /api/v1/tenants/{id}`
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!("/api/v1/tenants/{id}"))
            .await
    }

    /// `PUT /api/v1/tenants/{id}` — Update tenant.
    pub async fn update(
        &self,
        id: &str,
        body: &serde_json::Value,
    ) -> Result<TenantInfo> {
        self.client
            .put(&format!("/api/v1/tenants/{id}"), Some(body))
            .await
    }
}
