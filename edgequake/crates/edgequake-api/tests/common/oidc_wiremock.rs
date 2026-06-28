//! Wiremock OIDC IdP helpers (DRY — SPEC-027 / SPEC-028 OAuth e2e).

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub async fn mount_oidc_discovery(mock: &MockServer, signing_alg: &str) {
    let issuer = mock.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_json(json!({
                    "issuer": issuer,
                    "authorization_endpoint": format!("{issuer}/authorize"),
                    "token_endpoint": format!("{issuer}/token"),
                    "jwks_uri": format!("{issuer}/jwks"),
                    "response_types_supported": ["code"],
                    "subject_types_supported": ["public"],
                    "id_token_signing_alg_values_supported": [signing_alg],
                    "scopes_supported": ["openid", "email"],
                    "claims_supported": ["sub", "iss", "aud", "exp", "iat", "nonce", "email"],
                    "token_endpoint_auth_methods_supported": [
                        "client_secret_post",
                        "client_secret_basic"
                    ],
                })),
        )
        .mount(mock)
        .await;

    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_json(json!({ "keys": [] })),
        )
        .mount(mock)
        .await;
}

pub fn sign_hs256_id_token(
    issuer: &str,
    client_id: &str,
    client_secret: &str,
    nonce: &str,
    subject: &str,
    email: &str,
) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = json!({
        "iss": issuer,
        "sub": subject,
        "aud": client_id,
        "nonce": nonce,
        "email": email,
        "preferred_username": "mcp_user",
        "exp": now + 3600,
        "iat": now,
    });
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(client_secret.as_bytes()),
    )
    .expect("id_token encode")
}
