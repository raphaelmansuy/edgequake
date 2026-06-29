//! OIDC authorization-code flow with PKCE (SPEC-027 phase 54).

use std::sync::Arc;

use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use edgequake_auth::OidcConfig;

#[derive(Debug, Error)]
pub enum OidcServiceError {
    #[error("OIDC not configured")]
    NotConfigured,
    #[error("OIDC provider error: {0}")]
    Provider(String),
    #[error("OIDC state mismatch")]
    StateMismatch,
}

/// Serializable OIDC pending session stored in KV between login redirect and callback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcPendingSession {
    pub csrf_token: String,
    pub pkce_verifier: String,
    pub nonce: String,
}

/// Result of starting an OIDC login.
pub struct OidcLoginStart {
    pub authorization_url: String,
    pub pending: OidcPendingSession,
}

/// Normalized identity from IdP.
#[derive(Debug, Clone)]
pub struct OidcIdentity {
    pub subject: String,
    pub email: String,
    pub username: String,
}

pub struct OidcFlowService {
    config: OidcConfig,
    http_client: reqwest::Client,
}

impl OidcFlowService {
    pub fn new(config: OidcConfig) -> Self {
        let http_client = reqwest::ClientBuilder::new()
            .redirect(Policy::none())
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            config,
            http_client,
        }
    }

    pub fn config(&self) -> &OidcConfig {
        &self.config
    }

    pub async fn begin_login(&self) -> Result<OidcLoginStart, OidcServiceError> {
        if !self.config.is_runtime_builtin() {
            return Err(OidcServiceError::NotConfigured);
        }

        let issuer = IssuerUrl::new(self.config.issuer_url.clone())
            .map_err(|e| OidcServiceError::Provider(format!("invalid issuer URL: {e}")))?;
        let metadata = CoreProviderMetadata::discover_async(issuer, &self.http_client)
            .await
            .map_err(|e| OidcServiceError::Provider(format!("discovery failed: {e}")))?;
        let client_secret = self
            .config
            .client_secret
            .as_ref()
            .map(|s| ClientSecret::new(s.clone()));
        let redirect = RedirectUrl::new(self.config.redirect_uri.clone())
            .map_err(|e| OidcServiceError::Provider(format!("invalid redirect URI: {e}")))?;
        let client = CoreClient::from_provider_metadata(
            metadata,
            ClientId::new(self.config.client_id.clone()),
            client_secret,
        )
        .set_redirect_uri(redirect);
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token, nonce) = client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        Ok(OidcLoginStart {
            authorization_url: auth_url.to_string(),
            pending: OidcPendingSession {
                csrf_token: csrf_token.secret().to_string(),
                pkce_verifier: pkce_verifier.secret().to_string(),
                nonce: nonce.secret().to_string(),
            },
        })
    }

    pub async fn complete_login(
        &self,
        code: &str,
        state: &str,
        pending: &OidcPendingSession,
    ) -> Result<OidcIdentity, OidcServiceError> {
        if pending.csrf_token != state {
            return Err(OidcServiceError::StateMismatch);
        }

        let issuer = IssuerUrl::new(self.config.issuer_url.clone())
            .map_err(|e| OidcServiceError::Provider(format!("invalid issuer URL: {e}")))?;
        let metadata = CoreProviderMetadata::discover_async(issuer, &self.http_client)
            .await
            .map_err(|e| OidcServiceError::Provider(format!("discovery failed: {e}")))?;
        let client_secret = self
            .config
            .client_secret
            .as_ref()
            .map(|s| ClientSecret::new(s.clone()));
        let redirect = RedirectUrl::new(self.config.redirect_uri.clone())
            .map_err(|e| OidcServiceError::Provider(format!("invalid redirect URI: {e}")))?;
        let client = CoreClient::from_provider_metadata(
            metadata,
            ClientId::new(self.config.client_id.clone()),
            client_secret,
        )
        .set_redirect_uri(redirect);
        let pkce_verifier = PkceCodeVerifier::new(pending.pkce_verifier.clone());
        let nonce = Nonce::new(pending.nonce.clone());

        let token_response = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .map_err(|e| OidcServiceError::Provider(format!("token exchange setup: {e}")))?
            .set_pkce_verifier(pkce_verifier)
            .request_async(&self.http_client)
            .await
            .map_err(|e| OidcServiceError::Provider(format!("token exchange failed: {e}")))?;

        let id_token = token_response
            .id_token()
            .ok_or_else(|| OidcServiceError::Provider("missing id_token".into()))?;

        let id_token_verifier = client.id_token_verifier();
        let claims = id_token
            .claims(&id_token_verifier, &nonce)
            .map_err(|e| OidcServiceError::Provider(format!("id_token verify failed: {e}")))?;

        let subject = claims.subject().to_string();
        let email = claims
            .email()
            .map(|e| e.to_string())
            .unwrap_or_else(|| format!("oidc-{subject}@edgequake.local"));

        let username = claims
            .preferred_username()
            .map(|u| u.as_str().to_string())
            .or_else(|| email.split('@').next().map(str::to_string))
            .unwrap_or_else(|| format!("oidc_{subject}"));

        Ok(OidcIdentity {
            subject,
            email,
            username,
        })
    }
}

pub type SharedOidcFlowService = Arc<OidcFlowService>;
