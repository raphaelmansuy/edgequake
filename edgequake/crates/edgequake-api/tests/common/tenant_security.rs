use super::common::provider_access::http_harness::LiveServer;
use edgequake_auth::{Claims, Role};
use reqwest::{Method, StatusCode};
use uuid::Uuid;

pub async fn seed_user(
    server: &LiveServer,
    role: &str,
    tenant: Uuid,
    workspace: Uuid,
    membership_role: &str,
) -> Uuid {
    let user = Uuid::new_v4();
    let name = format!("tenant-security-{user}");
    let hash = server
        .state
        .auth
        .password
        .hash_password("TenantTest123!")
        .unwrap();
    sqlx::query("INSERT INTO users(user_id,tenant_id,username,email,password_hash,role,is_active) VALUES($1,$2,$3,$4,$5,$6,true)")
        .bind(user).bind(edgequake_api::middleware::default_tenant_uuid()).bind(&name).bind(format!("{name}@example.test")).bind(hash).bind(role)
        .execute(&server.pool).await.unwrap();
    sqlx::query("INSERT INTO memberships(membership_id,tenant_id,workspace_id,user_id,role,is_active) VALUES($1,$2,$3,$4,$5,true)")
        .bind(Uuid::new_v4()).bind(tenant).bind(workspace).bind(user).bind(membership_role).execute(&server.pool).await.unwrap();
    user
}

pub fn token(server: &LiveServer, user: Uuid, role: Role, scope: Option<(Uuid, Uuid)>) -> String {
    let mut claims = Claims::new(user, role, 600);
    if let Some((tenant, workspace)) = scope {
        claims = claims
            .with_tenant_id(tenant.to_string())
            .with_workspace_id(workspace.to_string());
    }
    server
        .state
        .auth
        .jwt
        .generate_token_with_claims(claims)
        .unwrap()
}

pub async fn request(
    server: &LiveServer,
    token: &str,
    method: Method,
    path: &str,
    headers: Option<(String, String)>,
    body: Option<serde_json::Value>,
) -> (StatusCode, String) {
    let mut request = server
        .client
        .request(method, format!("{}{path}", server.base))
        .bearer_auth(token);
    if let Some((tenant, workspace)) = headers {
        request = request
            .header("X-Tenant-ID", tenant)
            .header("X-Workspace-ID", workspace);
    }
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request.send().await.unwrap();
    let status = response.status();
    (status, response.text().await.unwrap())
}

pub async fn websocket_status(
    server: &LiveServer,
    credential: &str,
    scope: Option<(Uuid, Uuid)>,
) -> StatusCode {
    use tokio_tungstenite::{
        connect_async,
        tungstenite::{client::IntoClientRequest, Error},
    };
    let mut req = format!(
        "{}/ws/pipeline/progress",
        server.base.replace("http://", "ws://")
    )
    .into_client_request()
    .unwrap();
    req.headers_mut()
        .insert("Origin", "http://localhost:3000".parse().unwrap());
    req.headers_mut().insert(
        "Authorization",
        format!("Bearer {credential}").parse().unwrap(),
    );
    if let Some((tenant, workspace)) = scope {
        req.headers_mut()
            .insert("X-Tenant-ID", tenant.to_string().parse().unwrap());
        req.headers_mut()
            .insert("X-Workspace-ID", workspace.to_string().parse().unwrap());
    }
    match connect_async(req).await {
        Ok((mut socket, response)) => {
            socket.close(None).await.unwrap();
            response.status()
        }
        Err(Error::Http(response)) => response.status(),
        Err(error) => panic!("WebSocket connection failed: {error}"),
    }
}
