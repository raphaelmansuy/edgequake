//! Construct LLM/embedding providers from an explicit Connection (no env mutation).
//!
//! Uses public constructors from `edgequake-llm` 0.10.9.

use std::sync::Arc;

use edgequake_llm::google_auth::GoogleCredentials;
use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use edgequake_llm::{AnthropicProvider, GeminiProvider, OllamaProvider, OpenAIProvider};

use super::credential_material::{self, OpenedSecret};

use crate::error::ApiError;
use crate::providers::connection_shapes::{is_openai_wire, requires_server_credentials};
use crate::safety_limits::{
    SafetyLimitedEmbeddingProviderWrapper, SafetyLimitedProviderWrapper, SafetyLimitsConfig,
};

#[derive(Clone)]
pub struct ConnectionSpec {
    pub shape: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub embedding_model: Option<String>,
    pub embedding_dimension: Option<usize>,
}

/// Build a provider, minting a Vertex token from a stored service-account file when one is saved.
pub async fn llm_from_stored(spec: &ConnectionSpec) -> Result<Arc<dyn LLMProvider>, ApiError> {
    if credential_material::is_vertex(&spec.shape) {
        let inner = vertex_llm(spec).await?;
        return Ok(wrap_llm("vertexai", inner));
    }
    llm_from_connection(spec)
}

/// Embedding provider for a stored connection. Vertex uses the same credential as chat.
pub async fn embedding_from_stored(
    spec: &ConnectionSpec,
) -> Result<Arc<dyn EmbeddingProvider>, ApiError> {
    if credential_material::is_vertex(&spec.shape) {
        let inner = vertex_embed(spec).await?;
        return Ok(wrap_embed("vertexai", inner));
    }
    embedding_from_connection(spec)
}

async fn vertex_llm(spec: &ConnectionSpec) -> Result<Arc<dyn LLMProvider>, ApiError> {
    let provider = vertex_gemini(spec).await?;
    Ok(Arc::new(provider.with_model(&spec.model)))
}

async fn vertex_embed(spec: &ConnectionSpec) -> Result<Arc<dyn EmbeddingProvider>, ApiError> {
    let model = spec
        .embedding_model
        .clone()
        .unwrap_or_else(|| spec.model.clone());
    if crate::embeddings::is_vertex_multimodal(&spec.shape, &model) {
        if let Some(provider) = multimodal_from_stored(spec, &model)? {
            return Ok(provider);
        }
    }
    let provider = vertex_gemini(spec).await?;
    Ok(Arc::new(provider.with_embedding_model(&model)))
}

/// `gemini-embedding-2` is served by `:embedContent`, not the chat client's `:predict`.
fn multimodal_from_stored(
    spec: &ConnectionSpec,
    model: &str,
) -> Result<Option<Arc<dyn EmbeddingProvider>>, ApiError> {
    let Some(raw) = spec
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return Ok(None);
    };
    let OpenedSecret::GoogleCredential { material, .. } = credential_material::open_secret(raw)
    else {
        return Ok(None);
    };
    let creds: GoogleCredentials = serde_json::from_str(&material).map_err(|_| {
        ApiError::BadRequest(
            "The stored Google credential is not a service-account or authorized-user file.".into(),
        )
    })?;
    let project = creds.project_id().unwrap_or("").trim().to_string();
    if project.is_empty() {
        return Err(ApiError::BadRequest(
            "The stored Google credential has no project_id.".into(),
        ));
    }
    let dimension = spec.embedding_dimension.unwrap_or(1536);
    let embedder = crate::embeddings::vertex_gemini::VertexGeminiEmbedder::from_service_account(
        &project, &material, model, dimension,
    )
    .map_err(|error| crate::embeddings::api_error(&error))?;
    Ok(Some(Arc::new(
        crate::embeddings::vertex_gemini::VertexTextEmbedder(Arc::new(embedder)),
    )))
}

async fn vertex_gemini(spec: &ConnectionSpec) -> Result<GeminiProvider, ApiError> {
    let Some(raw) = spec
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return GeminiProvider::from_env_vertex_ai_adc()
            .await
            .map_err(|err| ApiError::BadRequest(err.to_string()));
    };
    match credential_material::open_secret(raw) {
        OpenedSecret::GoogleCredential { region, material } => {
            let creds: GoogleCredentials = serde_json::from_str(&material).map_err(|_| {
                ApiError::BadRequest(
                    "The stored Google credential is not a service-account or authorized-user file."
                        .into(),
                )
            })?;
            let project = creds.project_id().unwrap_or("").trim().to_string();
            if project.is_empty() {
                return Err(ApiError::BadRequest(
                    "The stored Google credential has no project_id.".into(),
                ));
            }
            let principal = creds.principal().to_string();
            let token = creds.into_source().refresh().await.map_err(|_| {
                ApiError::BadRequest(format!(
                    "Could not mint a Google access token for {principal}. The key may have been revoked."
                ))
            })?;
            Ok(GeminiProvider::vertex_ai(project, region, token.token))
        }
        OpenedSecret::GoogleApiKey(material) | OpenedSecret::ApiKey(material) => {
            Ok(GeminiProvider::new(material))
        }
    }
}

pub fn llm_from_connection(spec: &ConnectionSpec) -> Result<Arc<dyn LLMProvider>, ApiError> {
    let shape = spec.shape.to_ascii_lowercase();
    if requires_server_credentials(&shape) {
        return Err(server_credential_error(&shape));
    }
    let key = spec.api_key.clone().unwrap_or_default();
    let inner: Arc<dyn LLMProvider> = match shape.as_str() {
        /*
         * Vertex without an async token exchange. A saved Vertex secret is one of three things:
         * nothing (the server identity), an API key (Express or service-account-bound), or a
         * service-account JSON that must be traded for an access token over the network.
         * `llm_from_stored` handles the last case; this sync path can only build the key one.
         */
        "vertexai" | "vertex" => match credential_material::open_secret(&key) {
            OpenedSecret::GoogleApiKey(material) | OpenedSecret::ApiKey(material) => {
                Arc::new(GeminiProvider::new(material).with_model(&spec.model))
            }
            OpenedSecret::GoogleCredential { .. } => {
                return Err(ApiError::BadRequest(
                    "A Vertex service-account file needs an async token exchange. Use the stored-connection path, which mints a token before it builds the client."
                        .into(),
                ));
            }
        },
        "ollama" => Arc::new(build_ollama(spec)?),
        "anthropic_messages" | "anthropic" | "claude" => Arc::new(
            AnthropicProvider::new(key)
                .with_base_url(&spec.base_url)
                .with_model(&spec.model),
        ),
        "gemini" | "google" => {
            if key.is_empty() {
                return Err(ApiError::BadRequest(
                    "Gemini requires an API key on the connection".into(),
                ));
            }
            Arc::new(GeminiProvider::new(key).with_model(&spec.model))
        }
        /*
         * Azure is not plain OpenAI wire. Its model field is a *deployment*, and a saved base may
         * or may not include the Foundry `/openai/v1` suffix. Normalising here means one stored
         * URL works either way instead of producing `/openai/v1/openai/v1`.
         */
        "azure" | "azure-openai" | "azure_openai" => {
            if key.is_empty() {
                return Err(ApiError::BadRequest(
                    "Azure OpenAI requires an API key on the connection".into(),
                ));
            }
            let endpoint = azure_endpoint(&spec.base_url);
            Arc::new(edgequake_llm::AzureOpenAIProvider::new(
                endpoint,
                key,
                spec.model.clone(),
            ))
        }
        other if is_openai_wire(other) => {
            Arc::new(OpenAIProvider::compatible(key, spec.base_url.clone()).with_model(&spec.model))
        }
        other => {
            return Err(ApiError::BadRequest(format!(
                "provider shape '{other}' is not supported"
            )));
        }
    };
    Ok(wrap_llm(&shape, inner))
}

pub fn embedding_from_connection(
    spec: &ConnectionSpec,
) -> Result<Arc<dyn EmbeddingProvider>, ApiError> {
    let shape = spec.shape.to_ascii_lowercase();
    if requires_server_credentials(&shape) {
        return Err(server_credential_error(&shape));
    }
    let key = spec.api_key.clone().unwrap_or_default();
    let model = spec
        .embedding_model
        .clone()
        .unwrap_or_else(|| spec.model.clone());
    let inner: Arc<dyn EmbeddingProvider> = match shape.as_str() {
        "vertexai" | "vertex" => match credential_material::open_secret(&key) {
            OpenedSecret::GoogleApiKey(material) | OpenedSecret::ApiKey(material) => {
                Arc::new(GeminiProvider::new(material).with_embedding_model(&model))
            }
            OpenedSecret::GoogleCredential { .. } => {
                return Err(ApiError::BadRequest(
                    "A Vertex service-account file needs an async token exchange. Use the stored-connection path, which mints a token before it builds the client."
                        .into(),
                ));
            }
        },
        "ollama" => {
            let mut adjusted = spec.clone();
            adjusted.embedding_model = Some(model.clone());
            Arc::new(build_ollama(&adjusted)?)
        }
        "gemini" | "google" => {
            if key.is_empty() {
                return Err(ApiError::BadRequest(
                    "Gemini requires an API key on the connection".into(),
                ));
            }
            Arc::new(GeminiProvider::new(key).with_embedding_model(&model))
        }
        "azure" | "azure-openai" | "azure_openai" => {
            if key.is_empty() {
                return Err(ApiError::BadRequest(
                    "Azure OpenAI requires an API key on the connection".into(),
                ));
            }
            let endpoint = azure_endpoint(&spec.base_url);
            Arc::new(
                edgequake_llm::AzureOpenAIProvider::new(endpoint, key, spec.model.clone())
                    .with_embedding_deployment(&model),
            )
        }
        other if is_openai_wire(other) => Arc::new(
            OpenAIProvider::compatible(key, spec.base_url.clone()).with_embedding_model(&model),
        ),
        other => {
            return Err(ApiError::BadRequest(format!(
                "provider shape '{other}' is not supported for embeddings"
            )));
        }
    };
    Ok(wrap_embed(&shape, inner))
}

/// The Foundry v1 endpoint: the resource host, with exactly one `/openai/v1` suffix.
///
/// Operators paste either `https://r.openai.azure.com` or `.../openai/v1`, and the provider
/// appends `/openai/v1` itself. Passing the already-suffixed form through yields
/// `/openai/v1/openai/v1`. `/openai/deployments/...` is the legacy shape and is left alone.
fn azure_endpoint(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.contains("/openai/deployments/") {
        return trimmed.to_string();
    }
    trimmed
        .strip_suffix("/openai/v1")
        .unwrap_or(trimmed)
        .trim_end_matches('/')
        .to_string()
}

fn server_credential_error(shape: &str) -> ApiError {
    ApiError::BadRequest(format!(
        "{shape} is available to add, but a saved URL and key cannot build it. Configure server credentials for this provider."
    ))
}

fn build_ollama(spec: &ConnectionSpec) -> Result<OllamaProvider, ApiError> {
    let mut builder = OllamaProvider::builder()
        .host(spec.base_url.clone())
        .model(spec.model.clone());
    if let Some(key) = spec.api_key.as_deref().filter(|s| !s.is_empty()) {
        builder = builder.api_key(key);
    }
    if let Some(model) = spec.embedding_model.as_deref().filter(|s| !s.is_empty()) {
        builder = builder.embedding_model(model);
    }
    if let Some(dim) = spec.embedding_dimension.filter(|d| *d > 0) {
        builder = builder.embedding_dimension(dim);
    }
    builder
        .build()
        .map_err(|e| ApiError::BadRequest(e.to_string()))
}

fn wrap_llm(shape: &str, inner: Arc<dyn LLMProvider>) -> Arc<dyn LLMProvider> {
    let key = if crate::locality::is_slow_local_provider(shape) {
        shape
    } else {
        "openai"
    };
    Arc::new(SafetyLimitedProviderWrapper::new(
        inner,
        SafetyLimitsConfig::from_env_for_extraction(key),
    ))
}

fn wrap_embed(shape: &str, inner: Arc<dyn EmbeddingProvider>) -> Arc<dyn EmbeddingProvider> {
    let key = if crate::locality::is_slow_local_provider(shape) {
        shape
    } else {
        "openai"
    };
    Arc::new(SafetyLimitedEmbeddingProviderWrapper::new(
        inner,
        SafetyLimitsConfig::from_env_for_extraction(key),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /*
     * Endpoint facts that go stale. Each of these was a real 2026 breakage: Hugging Face retired
     * its host, Cohere's `/v1` is not OpenAI-shaped, and Azure's Foundry route ends in `/openai/v1`.
     */
    #[test]
    fn azure_endpoint_gets_exactly_one_foundry_suffix() {
        assert_eq!(
            azure_endpoint("https://r.openai.azure.com"),
            "https://r.openai.azure.com"
        );
        assert_eq!(
            azure_endpoint("https://r.openai.azure.com/"),
            "https://r.openai.azure.com"
        );
        assert_eq!(
            azure_endpoint("https://r.openai.azure.com/openai/v1"),
            "https://r.openai.azure.com"
        );
        // A doubled suffix is the failure this prevents.
        assert!(!azure_endpoint("https://r.openai.azure.com/openai/v1/").contains("/openai/v1"));
        // The legacy deployment route is not ours to rewrite.
        assert_eq!(
            azure_endpoint("https://r.openai.azure.com/openai/deployments/gpt-4o"),
            "https://r.openai.azure.com/openai/deployments/gpt-4o"
        );
    }

    #[test]
    fn azure_builds_a_provider_and_still_requires_a_key() {
        let spec = ConnectionSpec {
            shape: "azure".into(),
            base_url: "https://r.openai.azure.com/openai/v1".into(),
            api_key: Some("k".into()),
            model: "gpt-4o".into(),
            embedding_model: None,
            embedding_dimension: None,
        };
        assert!(llm_from_connection(&spec).is_ok());
        let keyless = ConnectionSpec {
            api_key: None,
            ..spec.clone()
        };
        assert!(llm_from_connection(&keyless).is_err());
    }

    #[test]
    fn shape_defaults_point_at_the_surfaces_that_still_exist() {
        use crate::providers::connection_shapes::CONNECTION_SHAPES;
        let base = |id: &str| {
            CONNECTION_SHAPES
                .iter()
                .find(|s| s.id == id)
                .unwrap_or_else(|| panic!("no shape {id}"))
                .default_base_url
        };
        // Retired in 2026 and returns 410 Gone.
        assert!(!base("huggingface").contains("api-inference.huggingface.co"));
        assert!(base("huggingface").contains("router.huggingface.co/v1"));
        // Cohere's /v1 is the native shape; the Compatibility API is the OpenAI-shaped one.
        assert!(base("cohere").contains("/compatibility/v1"));
        // Foundry v1 needs the suffix and no dated api-version.
        assert!(base("azure").ends_with("/openai/v1"));
    }

    #[test]
    fn openai_compatible_constructs() {
        let spec = ConnectionSpec {
            shape: "openai_chat".into(),
            base_url: "http://127.0.0.1:9050".into(),
            api_key: Some("k".into()),
            model: "default".into(),
            embedding_model: None,
            embedding_dimension: None,
        };
        assert!(llm_from_connection(&spec).is_ok());
        assert!(embedding_from_connection(&spec).is_ok());
    }

    #[test]
    fn ollama_uses_the_saved_host() {
        let spec = ConnectionSpec {
            shape: "ollama".into(),
            base_url: "http://127.0.0.1:19999".into(),
            api_key: None,
            model: "custom-chat".into(),
            embedding_model: Some("custom-embed".into()),
            embedding_dimension: Some(32),
        };
        let built = build_ollama(&spec).expect("builder");
        assert_eq!(built.host(), "http://127.0.0.1:19999");
        assert_eq!(LLMProvider::model(&built), "custom-chat");
        assert!(llm_from_connection(&spec).is_ok());
        assert!(embedding_from_connection(&spec).is_ok());
    }

    #[test]
    fn mistral_and_openrouter_use_the_saved_base_url() {
        for shape in ["mistral", "openrouter"] {
            let spec = ConnectionSpec {
                shape: shape.into(),
                base_url: "https://example.test/v1".into(),
                api_key: Some("k".into()),
                model: "default".into(),
                embedding_model: None,
                embedding_dimension: None,
            };
            assert!(llm_from_connection(&spec).is_ok(), "{shape}");
            assert!(embedding_from_connection(&spec).is_ok(), "{shape}");
        }
    }

    #[test]
    fn vertex_builds_from_a_saved_api_key_and_explains_a_service_account_file() {
        // An Express key or a service-account-bound key is a plain Gemini-API key: sync build works.
        let key_spec = ConnectionSpec {
            shape: "vertexai".into(),
            base_url: "https://aiplatform.googleapis.com".into(),
            api_key: Some("AIzaSyDaGmWKa4JsXZ-HjGw7ISLn_3namBGewQe".into()),
            model: "gemini-3-8-flash".into(),
            embedding_model: None,
            embedding_dimension: None,
        };
        assert!(llm_from_connection(&key_spec).is_ok());

        // A service-account file needs an async token exchange, so the sync builder must say so
        // rather than silently building the wrong client.
        let json_spec = ConnectionSpec {
            api_key: Some(
                r#"{"type":"service_account","client_email":"a@b.iam.gserviceaccount.com","private_key":"-----BEGIN PRIVATE KEY-----\nx\n-----END PRIVATE KEY-----","project_id":"p"}"#
                    .into(),
            ),
            ..key_spec.clone()
        };
        match llm_from_connection(&json_spec) {
            Err(err) => {
                let msg = err.to_string();
                assert!(
                    msg.contains("service-account") || msg.contains("service account"),
                    "{msg}"
                );
            }
            Ok(_) => panic!("a service-account file must not build synchronously"),
        }
    }
}
