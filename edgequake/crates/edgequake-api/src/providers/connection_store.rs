//! Load a stored provider connection into a [`ConnectionSpec`].

use std::sync::Arc;
use uuid::Uuid;

#[cfg(feature = "postgres")]
use edgequake_llm::traits::EmbeddingProvider;
use edgequake_llm::traits::LLMProvider;

#[cfg(feature = "postgres")]
tokio::task_local! {
    static SAVED_CONNECTION_POOL: sqlx::PgPool;
}

/// Pool handle threaded into ingest tasks so role resolution can decrypt a connection.
#[cfg(feature = "postgres")]
pub type AmbientPool = Option<sqlx::PgPool>;
#[cfg(not(feature = "postgres"))]
pub type AmbientPool = ();

/// Run `fut` with the saved-connection pool visible to [`llm_from_ambient_connection`].
pub async fn scope_saved_connection_pool<T>(
    pool: AmbientPool,
    fut: impl std::future::Future<Output = T>,
) -> T {
    #[cfg(feature = "postgres")]
    if let Some(pool) = pool {
        return SAVED_CONNECTION_POOL.scope(pool, fut).await;
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = pool;
    }
    fut.await
}

pub fn ambient_pool_from_state(state: &crate::state::AppState) -> AmbientPool {
    #[cfg(feature = "postgres")]
    {
        state.pg_pool.clone()
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = state;
        ()
    }
}

/// LLM built from a connection id when an ambient pool was scoped on this task.
pub async fn llm_from_ambient_connection(id: &str, model: &str) -> Option<Arc<dyn LLMProvider>> {
    #[cfg(feature = "postgres")]
    {
        let pool = SAVED_CONNECTION_POOL.try_with(|p| p.clone()).ok()?;
        return llm_from_pool(&pool, id, model).await;
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (id, model);
        None
    }
}

/// Parse a workspace role `connection_id` string.
pub fn parse_connection_id(raw: &str) -> Option<Uuid> {
    Uuid::parse_str(raw.trim()).ok()
}

/// Decrypt a `provider_connections` row into a runtime spec.
#[cfg(feature = "postgres")]
pub async fn load_connection_spec(
    pool: &sqlx::PgPool,
    id: Uuid,
    model: &str,
) -> Result<crate::providers::connection_factory::ConnectionSpec, crate::error::ApiError> {
    use crate::error::ApiError;
    use crate::providers::connection_factory::ConnectionSpec;

    let row: (
        String,
        String,
        String,
        Option<Vec<u8>>,
        Option<Vec<u8>>,
        Option<String>,
    ) = sqlx::query_as(
        r#"SELECT api_shape, base_url, auth_scheme, api_key_ciphertext, api_key_nonce, key_id
           FROM provider_connections WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .ok_or_else(|| ApiError::NotFound("connection not found".into()))?;

    let api_key = match (row.3, row.4, row.5) {
        (Some(ct), Some(nonce), Some(kid)) => {
            let env = edgequake_secrets::Envelope {
                ciphertext: ct,
                nonce,
                key_id: kid,
            };
            edgequake_secrets::decrypt_string(&env)
                .ok()
                .map(|s| s.expose().to_string())
        }
        _ => None,
    };

    Ok(ConnectionSpec {
        shape: row.0,
        base_url: row.1,
        api_key,
        model: model.to_string(),
        embedding_model: None,
        embedding_dimension: None,
    })
}

#[cfg(feature = "postgres")]
pub async fn llm_from_pool(
    pool: &sqlx::PgPool,
    id: &str,
    model: &str,
) -> Option<Arc<dyn LLMProvider>> {
    let uuid = parse_connection_id(id)?;
    let spec = load_connection_spec(pool, uuid, model).await.ok()?;
    crate::providers::connection_factory::llm_from_connection(&spec).ok()
}

#[cfg(feature = "postgres")]
pub async fn embedding_from_pool(
    pool: &sqlx::PgPool,
    id: &str,
    model: &str,
    dimension: usize,
) -> Option<Arc<dyn EmbeddingProvider>> {
    let uuid = parse_connection_id(id)?;
    let mut spec = load_connection_spec(pool, uuid, model).await.ok()?;
    spec.model = model.to_string();
    spec.embedding_model = Some(model.to_string());
    spec.embedding_dimension = Some(dimension);
    crate::providers::connection_factory::embedding_from_connection(&spec).ok()
}
