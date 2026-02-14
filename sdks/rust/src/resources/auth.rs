//! Auth resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::auth::*;

pub struct AuthResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> AuthResource<'a> {
    /// `POST /api/v1/auth/login`
    pub async fn login(&self, req: &LoginRequest) -> Result<TokenResponse> {
        self.client.post("/api/v1/auth/login", Some(req)).await
    }

    /// `POST /api/v1/auth/refresh`
    pub async fn refresh(&self, req: &RefreshRequest) -> Result<TokenResponse> {
        self.client.post("/api/v1/auth/refresh", Some(req)).await
    }

    /// `GET /api/v1/auth/me`
    pub async fn me(&self) -> Result<UserInfo> {
        self.client.get("/api/v1/auth/me").await
    }

    /// `POST /api/v1/auth/logout`
    pub async fn logout(&self) -> Result<()> {
        self.client
            .post_no_content::<()>("/api/v1/auth/logout", None)
            .await
    }
}
