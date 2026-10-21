//! Lakebase OAuth password. The token is the Postgres password and expires.

use std::time::Duration;

use super::auth::DatabricksBearer;

pub fn lakebase_auth_enabled() -> bool {
    matches!(
        std::env::var("EDGEQUAKE_DB_AUTH")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "lakebase_oauth" | "databricks"
    )
}

pub fn database_url_from_parts(
    user: &str,
    password: &str,
    host: &str,
    port: u16,
    database: &str,
    sslmode: &str,
) -> String {
    format!(
        "postgresql://{}:{}@{}:{}/{}?sslmode={}",
        urlencoding::encode(user),
        urlencoding::encode(password),
        host,
        port,
        database,
        sslmode
    )
}

pub async fn fresh_database_url() -> Result<String, String> {
    let (password, _) = fresh_password().await?;
    let user = required("PGUSER")?;
    let host = required("PGHOST")?;
    let database = required("PGDATABASE")?;
    let port = std::env::var("PGPORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5432);
    let sslmode = std::env::var("PGSSLMODE").unwrap_or_else(|_| "require".into());
    Ok(database_url_from_parts(
        &user, &password, &host, port, &database, &sslmode,
    ))
}

pub async fn fresh_password() -> Result<(String, Duration), String> {
    let bearer = DatabricksBearer::from_env()?;
    let endpoint = required("LAKEBASE_ENDPOINT")?;
    let token = bearer.access_token().await?;
    let url = format!("{}/api/2.0/postgres/credentials", bearer.host());
    let response = bearer
        .client()
        .post(url)
        .bearer_auth(token)
        .json(&serde_json::json!({
            "endpoint": endpoint,
            "ttl": "3600s"
        }))
        .send()
        .await
        .map_err(|e| format!("lakebase credential: {e}"))?;
    let status = response.status();
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("lakebase credential body: {e}"))?;
    if !status.is_success() {
        let message = body
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("credential request failed");
        return Err(format!("lakebase credential status {status}: {message}"));
    }
    let password = body
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or("lakebase credential response has no token")?
        .to_string();
    if password.is_empty() {
        return Err("lakebase credential token is empty".into());
    }
    Ok((password, Duration::from_secs(3600)))
}

fn required(name: &str) -> Result<String, String> {
    let value = std::env::var(name).map_err(|_| format!("{name} is required for Lakebase"))?;
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(format!("{name} is empty"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_encodes_user_and_password() {
        let url = database_url_from_parts(
            "user@example.com",
            "p@ss/word",
            "ep.example",
            5432,
            "databricks_postgres",
            "require",
        );
        assert!(url.contains("user%40example.com"));
        assert!(url.contains("p%40ss%2Fword"));
        assert!(url.contains("sslmode=require"));
        assert!(!url.contains("p@ss"));
    }

    #[tokio::test]
    async fn credential_post_uses_bearer_and_returns_ttl() {
        let _guard = crate::databricks::env_lock();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = vec![0_u8; 4096];
            let _ = stream.read(&mut buf).await.unwrap();
            let body = br#"{"token":"db-secret","expire_time":"2099-01-01T00:00:00Z"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.write_all(body).await.unwrap();
        });

        std::env::set_var("DATABRICKS_HOST", format!("http://{addr}"));
        std::env::set_var("DATABRICKS_TOKEN", "probe-token");
        std::env::remove_var("DATABRICKS_CLIENT_ID");
        std::env::remove_var("DATABRICKS_CLIENT_SECRET");
        std::env::set_var(
            "LAKEBASE_ENDPOINT",
            "projects/p/branches/production/endpoints/primary",
        );
        let (password, ttl) = fresh_password().await.expect("password");
        assert_eq!(password, "db-secret");
        assert_eq!(ttl, Duration::from_secs(3600));
        std::env::remove_var("DATABRICKS_HOST");
        std::env::remove_var("DATABRICKS_TOKEN");
        std::env::remove_var("LAKEBASE_ENDPOINT");
    }
}
