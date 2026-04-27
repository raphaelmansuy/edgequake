//! Admin endpoints (`/api/v1/admin/*`).

use serde::{Deserialize, Serialize};

use crate::client::EdgeQuakeClient;
use crate::error::Result;

/// PATCH `/api/v1/admin/tenants/{tenant_id}/quota`
#[derive(Debug, Clone, Serialize)]
pub struct UpdateTenantQuotaRequest {
    pub max_workspaces: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateTenantQuotaResponse {
    pub tenant_id: String,
    pub max_workspaces: usize,
    pub previous_max_workspaces: usize,
    pub current_workspace_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerDefaultsResponse {
    pub default_max_workspaces: usize,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateServerDefaultsRequest {
    pub default_max_workspaces: usize,
}

pub struct AdminResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> AdminResource<'a> {
    pub async fn patch_tenant_quota(
        &self,
        tenant_id: &str,
        max_workspaces: usize,
    ) -> Result<UpdateTenantQuotaResponse> {
        let body = UpdateTenantQuotaRequest { max_workspaces };
        self.client
            .patch(
                &format!("/api/v1/admin/tenants/{tenant_id}/quota"),
                Some(&body),
            )
            .await
    }

    pub async fn get_server_defaults(&self) -> Result<ServerDefaultsResponse> {
        self.client.get("/api/v1/admin/config/defaults").await
    }

    pub async fn patch_server_defaults(
        &self,
        default_max_workspaces: usize,
    ) -> Result<ServerDefaultsResponse> {
        let body = UpdateServerDefaultsRequest {
            default_max_workspaces,
        };
        self.client
            .patch("/api/v1/admin/config/defaults", Some(&body))
            .await
    }
}
