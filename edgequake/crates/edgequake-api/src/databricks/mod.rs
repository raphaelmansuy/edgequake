//! Databricks Apps, Lakebase, and the MLflow AI Gateway.
//!
//! Docker keeps its static `DATABASE_URL` and its existing LLM provider.
//! This module runs only when `EDGEQUAKE_DB_AUTH=lakebase_oauth` or
//! `EDGEQUAKE_LLM_PROVIDER=databricks`.

mod auth;
mod lakebase;
mod mlflow_content;
mod provider;

pub use auth::DatabricksBearer;
pub use lakebase::{database_url_from_parts, lakebase_auth_enabled};
pub use mlflow_content::{answer_and_thinking, answer_text};
pub use provider::{databricks_llm_enabled, is_databricks_provider, DatabricksMLflowProvider};

use std::time::Duration;

#[cfg(test)]
pub(crate) fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

/// Copy Apps runtime values into the names the server already reads.
///
/// Apps set `DATABRICKS_APP_PORT`. The server reads `PORT` and `HOST`.
pub fn apply_runtime_env() {
    let port_unset = std::env::var("PORT")
        .ok()
        .map(|v| v.trim().is_empty())
        .unwrap_or(true);
    if port_unset {
        if let Ok(port) = std::env::var("DATABRICKS_APP_PORT") {
            let port = port.trim();
            if !port.is_empty() {
                std::env::set_var("PORT", port);
            }
        }
    }
    let host_unset = std::env::var("HOST")
        .ok()
        .map(|v| v.trim().is_empty())
        .unwrap_or(true);
    if host_unset && std::env::var("DATABRICKS_APP_PORT").is_ok() {
        std::env::set_var("HOST", "0.0.0.0");
    }
    if databricks_llm_enabled() {
        set_if_empty("EDGEQUAKE_EMBEDDING_PROVIDER", "databricks");
        set_if_empty("EDGEQUAKE_EMBEDDING_MODEL", "system.ai.gte-large-en");
        set_if_empty("EDGEQUAKE_EMBEDDING_DIMENSION", "1024");
        set_if_empty("EDGEQUAKE_LLM_MODEL", "system.ai.llama-4-maverick");
    }
}

fn set_if_empty(name: &str, value: &str) {
    let missing = std::env::var(name)
        .ok()
        .map(|current| current.trim().is_empty())
        .unwrap_or(true);
    if missing {
        std::env::set_var(name, value);
    }
}

/// When Lakebase auth is on, mint a database password and set `DATABASE_URL`.
///
/// Other modes leave the environment unchanged.
pub async fn prepare_process_env() -> Result<(), String> {
    apply_runtime_env();
    if !lakebase_auth_enabled() {
        return Ok(());
    }
    let url = lakebase::fresh_database_url().await?;
    std::env::set_var("DATABASE_URL", url);
    tracing::info!("Lakebase OAuth database URL is ready");
    Ok(())
}

/// Refresh the password used for future pool connections.
///
/// Open connections stay valid after the token expires. New connections read
/// the options updated here. All clones of a pool share those options.
#[cfg(feature = "postgres")]
pub fn spawn_pool_refresh(bundle: edgequake_storage::PgPoolBundle) {
    if !lakebase_auth_enabled() {
        return;
    }
    tokio::spawn(async move {
        loop {
            match lakebase::fresh_password().await {
                Ok((password, ttl)) => {
                    apply_password(&bundle, &password);
                    let wait = ttl
                        .saturating_sub(Duration::from_secs(300))
                        .max(Duration::from_secs(30));
                    tracing::info!(
                        wait_secs = wait.as_secs(),
                        "Lakebase credential applied to future connections"
                    );
                    tokio::time::sleep(wait).await;
                }
                Err(error) => {
                    tracing::warn!(%error, "Lakebase credential refresh failed");
                    tokio::time::sleep(Duration::from_secs(15)).await;
                }
            }
        }
    });
}

#[cfg(feature = "postgres")]
fn apply_password(bundle: &edgequake_storage::PgPoolBundle, password: &str) {
    for pool in [&bundle.query, &bundle.ingest, &bundle.queue, &bundle.admin] {
        let options = (*pool.connect_options()).clone().password(password);
        pool.set_connect_options(options);
    }
}
