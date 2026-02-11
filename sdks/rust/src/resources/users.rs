//! Users resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::auth::*;

pub struct UsersResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> UsersResource<'a> {
    /// `GET /api/v1/users`
    pub async fn list(&self) -> Result<Vec<UserInfo>> {
        self.client.get("/api/v1/users").await
    }

    /// `POST /api/v1/users`
    pub async fn create(&self, req: &CreateUserRequest) -> Result<UserInfo> {
        self.client.post("/api/v1/users", Some(req)).await
    }

    /// `GET /api/v1/users/{id}`
    pub async fn get(&self, id: &str) -> Result<UserInfo> {
        self.client.get(&format!("/api/v1/users/{id}")).await
    }

    /// `DELETE /api/v1/users/{id}`
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!("/api/v1/users/{id}"))
            .await
    }
}
