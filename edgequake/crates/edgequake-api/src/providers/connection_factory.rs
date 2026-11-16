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
        if let Some(embedder) = vertex_multimodal_embedder(spec)? {
            return Ok(Arc::new(
                crate::embeddings::vertex_gemini::VertexTextEmbedder(embedder),
            ));
        }
    }
    let provider = vertex_gemini(spec).await?;
    Ok(Arc::new(provider.with_embedding_model(&model)))
}

/// `gemini-embedding-2` over `:embedContent`, for any of the three Vertex credential kinds.
///
/// WHY three branches: Vertex accepts a service-account/authorized-user JSON, an API key
/// (Express or service-account-bound), and the server's own Google identity (ADC / metadata /
/// an active `gcloud` account). Before this, only the JSON branch existed, so a workspace
/// configured with an API key or with ADC silently lost image embedding — the multi-modal
/// embedder returned `None` and the caller fell back to text-only, which made image search
/// degrade without saying why.
pub fn vertex_multimodal_embedder(
    spec: &ConnectionSpec,
) -> Result<Option<Arc<crate::embeddings::vertex_gemini::VertexGeminiEmbedder>>, ApiError> {
    let model = spec
        .embedding_model
        .clone()
        .unwrap_or_else(|| spec.model.clone());
    let dimension = spec.embedding_dimension.unwrap_or(1536);

    // An explicit connection secret wins: JSON → project from the file; API key → project from env.
    if let Some(raw) = spec
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return match credential_material::open_secret(raw) {
            OpenedSecret::GoogleCredential { material, .. } => {
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
                build_multimodal_json(&project, &material, &model, dimension)
            }
            OpenedSecret::GoogleApiKey(key) | OpenedSecret::ApiKey(key) => {
                /*
                 * A Vertex **API key** cannot do image embedding, and no header spelling fixes it.
                 *
                 * Verified against the live API:
                 *   - `x-goog-api-key` on aiplatform           → 401 "API keys are not supported
                 *                                                 by this API. Expected OAuth2 …"
                 *   - `?key=` on aiplatform (Express mode)     → 401, same body
                 *   - `x-goog-api-key` on generativelanguage   → 400 API_KEY_INVALID
                 *                                                 (i.e. the key *is* understood,
                 *                                                  just not by Vertex)
                 *
                 * So an API key authenticates AI Studio, not Vertex. AI Studio's embedder is
                 * text-only, so returning `None` here is the honest answer: the workspace keeps
                 * text search and loses picture search. Pretending otherwise would surface as a
                 * 401 mid-query that reads like a bad key.
                 *
                 * To get picture embedding, save the service-account JSON, or leave the secret
                 * blank so the server's Google identity (ADC / metadata / active gcloud account)
                 * is used — both of those are OAuth2 and both work.
                 */
                let _ = key;
                tracing::warn!(
                    model = %model,
                    "Vertex API key cannot embed images (Vertex accepts OAuth2 only); \
                     image search is unavailable for this workspace. Save a service-account \
                     JSON or leave the secret blank to use the server identity."
                );
                Ok(None)
            }
        };
    }

    // No saved secret: the server's own identity (ADC / metadata / gcloud active account).
    let project = require_vertex_project()?;
    let embedder =
        crate::embeddings::vertex_gemini::VertexGeminiEmbedder::from_env(&model, dimension)
            .map_err(|error| crate::embeddings::api_error(&error))?;
    tracing::debug!(
        project = %project,
        model = %model,
        "Vertex multimodal embedder uses the server Google identity"
    );
    Ok(Some(Arc::new(embedder)))
}

/// `GOOGLE_CLOUD_PROJECT`, or the error that tells an operator what to set.
fn require_vertex_project() -> Result<String, ApiError> {
    std::env::var("GOOGLE_CLOUD_PROJECT")
        .ok()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .ok_or_else(|| {
            ApiError::BadRequest(
                "Vertex AI needs GOOGLE_CLOUD_PROJECT when the connection stores an API key or \
                 relies on the server's Google identity. Set it, or save a service-account JSON \
                 file, which carries its own project_id."
                    .into(),
            )
        })
}

fn build_multimodal_json(
    project: &str,
    material: &str,
    model: &str,
    dimension: usize,
) -> Result<Option<Arc<crate::embeddings::vertex_gemini::VertexGeminiEmbedder>>, ApiError> {
    let embedder = crate::embeddings::vertex_gemini::VertexGeminiEmbedder::from_service_account(
        project, material, model, dimension,
    )
    .map_err(|error| crate::embeddings::api_error(&error))?;
    Ok(Some(Arc::new(embedder)))
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
         * Vertex without an async token exchange.
         *
         * WHY `GeminiProvider::new` (AI Studio) is correct *here* but wrong for embeddings:
         * Vertex's endpoint rejects API keys outright (verified: 401 "API keys are not supported
         * by this API"), while AI Studio accepts them and serves chat. So an API key can chat but
         * cannot use Vertex's multimodal embedding. The embedding builder therefore refuses this
         * shape instead of detouring, since AI Studio has no image embedder.
         *
         * `llm_from_stored` handles the JSON and server-identity cases, which need an async token.
         * This sync arm only ever builds the key form, and that form really does mean AI Studio.
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
        /*
         * WHY this is an error and not `GeminiProvider::new(key)`:
         * `GeminiProvider::new` builds a **Google AI Studio** endpoint
         * (`generativelanguage.googleapis.com`), not Vertex. A `vertexai` shape reaching this
         * sync builder would silently embed through a different API than the one the operator
         * configured — different model names, different quota, different billing. That is the
         * same wrong-endpoint class of bug that made API keys look broken on Vertex.
         *
         * `embedding_from_stored` handles every real `vertexai` connection and never reaches
         * here; this arm exists so a future caller gets a clear error instead of a silent
         * detour through AI Studio.
         */
        "vertexai" | "vertex" => {
            return Err(ApiError::BadRequest(
                "Vertex embeddings need the stored-connection path, which selects the right \
                 Google identity (service-account JSON, ADC, or the active gcloud account). \
                 The synchronous builder would target AI Studio instead of Vertex."
                    .into(),
            ));
        }
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

    /// The synchronous embedding builder must refuse Vertex rather than detour to AI Studio.
    ///
    /// WHY: `GeminiProvider::new(key)` builds a `generativelanguage` client. A `vertexai` shape
    /// reaching that path would embed through a different API than the operator configured —
    /// different models, quota, and billing — and look like success. The stored-connection path
    /// handles every real Vertex workspace, so this arm should only ever produce an error.
    #[test]
    fn sync_embedding_builder_refuses_vertex_instead_of_targeting_ai_studio() {
        let spec = vertex_spec(Some("AIzaSyExampleKey"));
        let result = embedding_from_connection(&spec);

        let msg = match result {
            Err(err) => err.to_string(),
            Ok(_) => panic!(
                "a vertexai shape must not build through the sync path; \
                 that would silently target AI Studio"
            ),
        };
        assert!(
            msg.contains("stored-connection") || msg.contains("AI Studio"),
            "the error must say what to do instead: {msg}"
        );
    }

    // ── Multimodal embedding must accept all three Vertex credential kinds ───
    //
    // WHY these matter: this function used to return `None` for an API key or for the server
    // identity, so an image workspace configured that way silently fell back to text-only
    // embedding. Nothing errored; image search just quietly stopped working.

    fn vertex_spec(api_key: Option<&str>) -> ConnectionSpec {
        ConnectionSpec {
            shape: "vertexai".into(),
            base_url: "https://aiplatform.googleapis.com".into(),
            api_key: api_key.map(str::to_string),
            model: "gemini-3-8-flash".into(),
            embedding_model: Some("gemini-embedding-2".into()),
            embedding_dimension: Some(1536),
        }
    }

    fn with_project<T>(value: &str, body: impl FnOnce() -> T) -> T {
        let previous = std::env::var("GOOGLE_CLOUD_PROJECT").ok();
        std::env::set_var("GOOGLE_CLOUD_PROJECT", value);
        let out = body();
        match previous {
            Some(v) => std::env::set_var("GOOGLE_CLOUD_PROJECT", v),
            None => std::env::remove_var("GOOGLE_CLOUD_PROJECT"),
        }
        out
    }

    #[test]
    fn multimodal_embedder_accepts_a_service_account_json() {
        // Project comes from the file, so GOOGLE_CLOUD_PROJECT is deliberately absent.
        let json = r#"{"type":"service_account","client_email":"a@b.iam.gserviceaccount.com","private_key":"-----BEGIN PRIVATE KEY-----\nx\n-----END PRIVATE KEY-----","project_id":"file-project"}"#;
        let spec = vertex_spec(Some(json));
        let embedder = vertex_multimodal_embedder(&spec).expect("a JSON credential builds");
        assert!(
            embedder.is_some(),
            "a service-account file must yield a multimodal embedder"
        );
    }

    /// A Vertex API key cannot embed images: Vertex accepts OAuth2 only.
    ///
    /// WHY assert `None` rather than an error: image search is one capability among several. The
    /// workspace keeps text search; losing pictures is a degradation, not a failed request. The
    /// returned `None` is what makes the caller fall back to text-only.
    #[test]
    #[serial_test::serial]
    fn multimodal_embedder_reports_no_image_support_for_an_api_key() {
        with_project("env-project", || {
            let spec = vertex_spec(Some("AIzaSyExampleKey"));
            let embedder = vertex_multimodal_embedder(&spec)
                .expect("an API key is not an error, just limited");
            assert!(
                embedder.is_none(),
                "Vertex rejects API keys, so no multimodal embedder can be built from one"
            );
        });
    }

    #[test]
    #[serial_test::serial]
    fn multimodal_embedder_accepts_the_server_identity_when_no_secret_is_saved() {
        with_project("env-project", || {
            let spec = vertex_spec(None);
            let embedder = vertex_multimodal_embedder(&spec).expect("ADC builds");
            assert!(
                embedder.is_some(),
                "no saved secret must mean the server identity, not no embedder"
            );
        });
    }

    /// The server identity needs a project; the message must name the variable to set.
    ///
    /// `#[serial]` because it mutates `GOOGLE_CLOUD_PROJECT`, which the other Vertex tests in this
    /// module also read; cargo runs tests on threads, so without it this test would clear the
    /// project out from under them.
    #[test]
    #[serial_test::serial]
    fn the_server_identity_without_a_project_says_which_variable_to_set() {
        let previous = std::env::var("GOOGLE_CLOUD_PROJECT").ok();
        std::env::remove_var("GOOGLE_CLOUD_PROJECT");

        let result = vertex_multimodal_embedder(&vertex_spec(None));
        let msg = match result {
            Err(err) => err.to_string(),
            // An embedder could only be built if a project were somehow present.
            Ok(Some(_)) => panic!("a server-identity embedder needs a project"),
            Ok(None) => panic!("the server identity must not silently report no image support"),
        };
        assert!(msg.contains("GOOGLE_CLOUD_PROJECT"), "{msg}");

        if let Some(v) = previous {
            std::env::set_var("GOOGLE_CLOUD_PROJECT", v);
        }
    }
}
