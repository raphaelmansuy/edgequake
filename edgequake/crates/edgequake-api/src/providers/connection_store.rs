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

/// Fleet LLM when an ambient pool was scoped on this task.
///
/// `Ok(None)` means no saved secret. A saved secret that cannot mint a token is an error.
pub async fn fleet_llm_from_ambient(
    provider: &str,
    model: &str,
) -> Result<Option<Arc<dyn LLMProvider>>, String> {
    #[cfg(feature = "postgres")]
    {
        let Ok(pool) = SAVED_CONNECTION_POOL.try_with(|pool| pool.clone()) else {
            return Ok(None);
        };
        return fleet_llm(&pool, provider, model).await;
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (provider, model);
        Ok(None)
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

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
struct StoredConnectionRow {
    tenant_id: Option<Uuid>,
    api_shape: String,
    base_url: String,
    api_key_ciphertext: Option<Vec<u8>>,
    api_key_nonce: Option<Vec<u8>>,
    key_id: Option<String>,
    secret_ref: Option<serde_json::Value>,
    google_project: Option<String>,
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

    let row = sqlx::query_as::<_, StoredConnectionRow>(
        r#"SELECT tenant_id, api_shape, base_url, api_key_ciphertext, api_key_nonce, key_id, secret_ref,
                  google_project
           FROM provider_connections WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .ok_or_else(|| ApiError::NotFound("connection not found".into()))?;

    let api_key = super::secret_manager::reveal_connection_key(
        pool,
        row.tenant_id,
        row.secret_ref,
        row.api_key_ciphertext,
        row.api_key_nonce,
        row.key_id,
    )
    .await?;

    Ok(ConnectionSpec {
        shape: row.api_shape,
        base_url: row.base_url,
        api_key,
        model: model.to_string(),
        embedding_model: None,
        embedding_dimension: None,
        google_project: row.google_project,
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
    crate::providers::connection_factory::llm_from_stored(&spec)
        .await
        .ok()
}

/// Build an LLM from `llm_roles.<role>.connection_id` when that id is set.
#[cfg(feature = "postgres")]
pub async fn llm_from_workspace_role(
    pool: &sqlx::PgPool,
    ws: &edgequake_core::Workspace,
    role: edgequake_core::LlmRole,
) -> Option<Arc<dyn LLMProvider>> {
    let resolved = edgequake_core::resolve_role_llm(ws, role);
    let id = resolved.connection_id.as_deref()?;
    llm_from_pool(pool, id, &resolved.model).await
}

/// Slugs that name the same saved connection. `vertex` and `vertexai` are one provider.
pub fn provider_match_names(provider: &str) -> Vec<String> {
    let name = provider.trim().to_ascii_lowercase();
    let mut names = vec![name.clone()];
    if matches!(name.as_str(), "vertex" | "vertexai" | "vertex-ai") {
        for alias in ["vertex", "vertexai", "vertex-ai"] {
            if !names.iter().any(|existing| existing == alias) {
                names.push(alias.to_string());
            }
        }
    }
    names
}

/// Fleet connection id for a provider slug, when a secret is stored.
#[cfg(feature = "postgres")]
pub async fn fleet_connection_id(pool: &sqlx::PgPool, provider: &str) -> Option<Uuid> {
    let names = provider_match_names(provider);
    sqlx::query_scalar(
        "SELECT id FROM provider_connections \
         WHERE tenant_id IS NULL \
           AND (lower(slug) = ANY($1) OR lower(api_shape) = ANY($1)) \
           AND (api_key_ciphertext IS NOT NULL OR secret_ref IS NOT NULL \
                OR NULLIF(btrim(google_project), '') IS NOT NULL) \
         ORDER BY (COALESCE(last_test_ok, false)) DESC \
         LIMIT 1",
    )
    .bind(&names)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

/// LLM built from the fleet connection for `provider`. `Ok(None)` means no saved secret.
#[cfg(feature = "postgres")]
pub async fn fleet_llm(
    pool: &sqlx::PgPool,
    provider: &str,
    model: &str,
) -> Result<Option<Arc<dyn LLMProvider>>, String> {
    let Some(id) = fleet_connection_id(pool, provider).await else {
        return Ok(None);
    };
    let spec = load_connection_spec(pool, id, model)
        .await
        .map_err(|err| err.to_string())?;
    crate::providers::connection_factory::llm_from_stored(&spec)
        .await
        .map(Some)
        .map_err(|err| err.to_string())
}

/// Image embedder for `gemini-embedding-2` from one saved Vertex connection.
///
/// `Ok(None)` means that row has no service account and no project, so the
/// caller may try another row or the process environment. A stored account
/// that cannot be opened is an error.
#[cfg(feature = "postgres")]
async fn embedder_from_connection(
    pool: &sqlx::PgPool,
    id: Uuid,
    model: &str,
    dimension: usize,
) -> Result<Option<Arc<crate::embeddings::vertex_gemini::VertexGeminiEmbedder>>, String> {
    let mut spec = load_connection_spec(pool, id, model)
        .await
        .map_err(|err| err.to_string())?;
    spec.embedding_model = Some(model.to_string());
    spec.embedding_dimension = Some(dimension);
    if let Some(embedder) = crate::providers::connection_factory::vertex_multimodal_embedder(&spec)
        .map_err(|err| err.to_string())?
    {
        return Ok(Some(embedder));
    }
    let Some(project) = spec
        .google_project
        .as_deref()
        .map(str::trim)
        .filter(|project| !project.is_empty())
    else {
        return Ok(None);
    };
    crate::embeddings::vertex_gemini::VertexGeminiEmbedder::for_saved_project(
        project, model, dimension,
    )
    .map(|embedder| Some(Arc::new(embedder)))
    .map_err(|err| err.to_string())
}

/// Image embedder for `gemini-embedding-2` from the saved Vertex service account.
///
/// `Ok(None)` means there is no stored service account and no saved project, so
/// the caller may use the process environment. A stored account that cannot be
/// opened is an error: falling through would report a missing
/// `GOOGLE_CLOUD_PROJECT`.
#[cfg(feature = "postgres")]
pub async fn fleet_vertex_embedder(
    pool: &sqlx::PgPool,
    model: &str,
    dimension: usize,
) -> Result<Option<Arc<crate::embeddings::vertex_gemini::VertexGeminiEmbedder>>, String> {
    let Some(id) = fleet_connection_id(pool, "vertexai").await else {
        return Ok(None);
    };
    embedder_from_connection(pool, id, model, dimension).await
}

/// Image embedder for a workspace: its own embedding connection, then the fleet card.
///
/// A saved project is enough. The process environment is not read here.
#[cfg(feature = "postgres")]
pub async fn vertex_embedder_for_workspace(
    pool: &sqlx::PgPool,
    workspace: &edgequake_core::Workspace,
    model: &str,
    dimension: usize,
) -> Result<Option<Arc<crate::embeddings::vertex_gemini::VertexGeminiEmbedder>>, String> {
    let mut saved_error = None;
    if let Some(raw) = edgequake_core::metadata_connection_id(workspace, "embedding") {
        if let Some(id) = parse_connection_id(&raw) {
            match embedder_from_connection(pool, id, model, dimension).await {
                Ok(Some(embedder)) => return Ok(Some(embedder)),
                Ok(None) => {}
                Err(error) => saved_error = Some(error),
            }
        }
    }
    match fleet_vertex_embedder(pool, model, dimension).await? {
        Some(embedder) => Ok(Some(embedder)),
        None => match saved_error {
            Some(error) => Err(error),
            None => Ok(None),
        },
    }
}

/// Embedding client built from the fleet connection for `provider`.
#[cfg(feature = "postgres")]
pub async fn fleet_embedding(
    pool: &sqlx::PgPool,
    provider: &str,
    model: &str,
    dimension: usize,
) -> Result<Option<Arc<dyn EmbeddingProvider>>, String> {
    let Some(id) = fleet_connection_id(pool, provider).await else {
        return Ok(None);
    };
    let mut spec = load_connection_spec(pool, id, model)
        .await
        .map_err(|err| err.to_string())?;
    spec.model = model.to_string();
    spec.embedding_model = Some(model.to_string());
    spec.embedding_dimension = Some(dimension);
    crate::providers::connection_factory::embedding_from_stored(&spec)
        .await
        .map(Some)
        .map_err(|err| err.to_string())
}

/// Vision LLM from the workspace role, else the fleet connection of the same slug.
///
/// `Ok(None)` means there is no saved secret, so the caller may use the process
/// environment. A saved secret that cannot mint a token is an error: falling
/// through would report a missing `GOOGLE_CLOUD_PROJECT` instead.
pub async fn vision_llm_from_saved(
    pool: AmbientPool,
    ws: Option<&edgequake_core::Workspace>,
    provider: &str,
    model: &str,
) -> Result<Option<Arc<dyn LLMProvider>>, String> {
    #[cfg(feature = "postgres")]
    {
        let Some(pool) = pool.as_ref() else {
            return Ok(None);
        };
        if let Some(ws) = ws {
            let resolved = edgequake_core::resolve_role_llm(ws, edgequake_core::LlmRole::Vlm);
            if resolved
                .connection_id
                .as_deref()
                .is_some_and(|id| !id.is_empty())
            {
                return match llm_from_workspace_role(pool, ws, edgequake_core::LlmRole::Vlm).await {
                    Some(provider) => Ok(Some(provider)),
                    None => Err(format!(
                        "The saved vision connection for '{provider}' could not be opened."
                    )),
                };
            }
        }
        return fleet_llm(pool, provider, model).await;
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (pool, ws, provider, model);
        Ok(None)
    }
}

/// Prefer a saved VLM connection; otherwise build from provider name + model.
pub async fn vision_llm_preferring_connection(
    pool: AmbientPool,
    ws: Option<&edgequake_core::Workspace>,
    provider: &str,
    model: &str,
) -> Option<Arc<dyn LLMProvider>> {
    match vision_llm_from_saved(pool, ws, provider, model).await {
        Ok(Some(provider)) => return Some(provider),
        Ok(None) => {}
        Err(error) => {
            tracing::warn!(
                %error,
                provider,
                "saved vision connection could not be opened"
            );
        }
    }
    edgequake_llm::ProviderFactory::create_llm_provider(provider, model).ok()
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
    crate::providers::connection_factory::embedding_from_stored(&spec)
        .await
        .ok()
}

#[cfg(test)]
mod tests {
    use super::provider_match_names;

    #[test]
    fn vertex_aliases_find_the_saved_slug() {
        let names = provider_match_names("VertexAI");
        assert!(names.iter().any(|name| name == "vertexai"));
        assert!(names.iter().any(|name| name == "vertex"));
        assert!(names.iter().any(|name| name == "vertex-ai"));
    }

    #[test]
    fn other_providers_match_their_own_slug() {
        assert_eq!(provider_match_names(" OpenAI "), vec!["openai".to_string()]);
    }

    #[tokio::test]
    async fn vision_without_a_database_does_not_invent_a_connection() {
        let saved = super::vision_llm_from_saved(
            #[cfg(feature = "postgres")]
            None,
            #[cfg(not(feature = "postgres"))]
            (),
            None,
            "vertexai",
            "gemini-3.8-flash",
        )
        .await
        .expect("no database is not an error");
        assert!(saved.is_none());
    }
}
