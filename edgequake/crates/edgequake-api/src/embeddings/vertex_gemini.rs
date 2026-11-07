//! SPEC-180 08: Vertex AI `gemini-embedding-2` over `:embedContent`.
//!
//! The crates.io client calls `:predict`, which this model does not serve, so a workspace
//! that selects it uses this client even when it embeds text only. One HTTP call per input,
//! fanned out under a concurrency cap. No `taskType` is sent; the text prefix carries the
//! task (R-180-5). Failure is a coded error; nothing falls back to a mock (LAW-180-15).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use edgequake_core::multimodal_embed::{
    document_prefix, embed_codes, EmbedError, EmbedInput, EmbedPart, MultimodalEmbedder,
};
use serde_json::{json, Value};

use super::image_limits::{estimate_tokens, prepare_image, MAX_IMAGES_PER_INPUT};
use super::vertex_token::VertexTokenProvider;

/// Dimensions the model serves (SPEC-180 LAW-180-5).
pub const SUPPORTED_DIMENSIONS: [usize; 3] = [768, 1536, 3072];
/// The model's input budget in tokens.
pub const MAX_INPUT_TOKENS: usize = 8192;
const DEFAULT_BASE_URL: &str = "https://aiplatform.googleapis.com";
/// `gemini-embedding-2` is served from the global location (W0 spike, verified live).
const LOCATION: &str = "global";
const DEFAULT_CONCURRENCY: usize = 4;
const MAX_ATTEMPTS: u32 = 3;
const NORM_TOLERANCE: f32 = 1e-3;

/// Connection settings of one client.
#[derive(Debug, Clone)]
pub struct VertexGeminiConfig {
    pub project: String,
    pub base_url: String,
    pub model: String,
    pub dimension: usize,
    pub concurrency: usize,
    /// Per-image byte budget; `0` disables it.
    pub max_image_bytes: usize,
}

impl VertexGeminiConfig {
    /// Settings from the environment. A missing project is `embedding_auth`, never a mock.
    pub fn from_env(model: &str, dimension: usize) -> Result<Self, EmbedError> {
        let project = std::env::var("GOOGLE_CLOUD_PROJECT")
            .ok()
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .ok_or_else(|| EmbedError::new(embed_codes::AUTH, "GOOGLE_CLOUD_PROJECT is not set"))?;
        let base_url = std::env::var("EDGEQUAKE_VERTEX_BASE_URL")
            .ok()
            .map(|u| u.trim().trim_end_matches('/').to_string())
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let concurrency = std::env::var("EDGEQUAKE_EMBED_CONCURRENCY")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .filter(|n| *n > 0)
            .unwrap_or(DEFAULT_CONCURRENCY);
        let max_image_bytes = edgequake_pdf::image_guard::max_image_bytes_from_env();
        Self::new(
            project,
            base_url,
            model,
            dimension,
            concurrency,
            max_image_bytes,
        )
    }

    pub fn new(
        project: impl Into<String>,
        base_url: impl Into<String>,
        model: &str,
        dimension: usize,
        concurrency: usize,
        max_image_bytes: usize,
    ) -> Result<Self, EmbedError> {
        if !SUPPORTED_DIMENSIONS.contains(&dimension) {
            return Err(EmbedError::new(
                embed_codes::DIMENSION_MISMATCH,
                format!("dimension must be one of 768, 1536, 3072; got {dimension}"),
            ));
        }
        Ok(Self {
            project: project.into(),
            base_url: base_url.into(),
            model: model.to_string(),
            dimension,
            concurrency: concurrency.max(1),
            max_image_bytes,
        })
    }

    fn endpoint(&self) -> String {
        format!(
            "{}/v1/projects/{}/locations/{LOCATION}/publishers/google/models/{}:embedContent",
            self.base_url, self.project, self.model
        )
    }
}

/// The client. Cheap to clone behind an `Arc`.
pub struct VertexGeminiEmbedder {
    config: VertexGeminiConfig,
    http: reqwest::Client,
    tokens: VertexTokenProvider,
}

impl VertexGeminiEmbedder {
    pub fn new(
        config: VertexGeminiConfig,
        tokens: VertexTokenProvider,
    ) -> Result<Self, EmbedError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| EmbedError::new(embed_codes::REQUEST, e.to_string()))?;
        Ok(Self {
            config,
            http,
            tokens,
        })
    }

    /// A client whose token is minted from a stored service-account JSON.
    ///
    /// The project comes from that file, so the process does not need
    /// `GOOGLE_CLOUD_PROJECT`.
    pub fn from_service_account(
        project: &str,
        material: &str,
        model: &str,
        dimension: usize,
    ) -> Result<Self, EmbedError> {
        let base_url = std::env::var("EDGEQUAKE_VERTEX_BASE_URL")
            .ok()
            .map(|u| u.trim().trim_end_matches('/').to_string())
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let max_image_bytes = edgequake_pdf::image_guard::max_image_bytes_from_env();
        Self::new(
            VertexGeminiConfig::new(
                project,
                base_url,
                model,
                dimension,
                DEFAULT_CONCURRENCY,
                max_image_bytes,
            )?,
            VertexTokenProvider::new(Arc::new(super::vertex_token::StoredServiceAccount::new(
                material,
            ))),
        )
    }

    /// A client for a project saved on the connection.
    ///
    /// The token is this server's Google identity, checked against `project`.
    /// `GOOGLE_CLOUD_PROJECT` is not read.
    pub fn for_saved_project(
        project: &str,
        model: &str,
        dimension: usize,
    ) -> Result<Self, EmbedError> {
        let base_url = std::env::var("EDGEQUAKE_VERTEX_BASE_URL")
            .ok()
            .map(|u| u.trim().trim_end_matches('/').to_string())
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let max_image_bytes = edgequake_pdf::image_guard::max_image_bytes_from_env();
        Self::new(
            VertexGeminiConfig::new(
                project,
                base_url,
                model,
                dimension,
                DEFAULT_CONCURRENCY,
                max_image_bytes,
            )?,
            VertexTokenProvider::for_project(project),
        )
    }

    /// Project this client calls. The query path uses it to reuse one HTTP pool.
    pub fn project(&self) -> &str {
        &self.config.project
    }

    /// Identity of this client in the process-wide pool.
    pub(crate) fn shared_key(&self) -> (String, usize, String, String) {
        (
            self.config.model.clone(),
            self.config.dimension,
            self.config.project.clone(),
            self.config.base_url.clone(),
        )
    }

    /// A client for `model` at `dimension`, with credentials from the environment.
    pub fn from_env(model: &str, dimension: usize) -> Result<Self, EmbedError> {
        Self::new(
            VertexGeminiConfig::from_env(model, dimension)?,
            VertexTokenProvider::from_env(),
        )
    }

    /// Request body of one input; images are checked and re-encoded first.
    pub fn request_body(&self, input: &EmbedInput) -> Result<Value, EmbedError> {
        if input.image_count() > MAX_IMAGES_PER_INPUT {
            return Err(EmbedError::new(
                embed_codes::INPUT_TOO_LARGE,
                format!("an input carries at most {MAX_IMAGES_PER_INPUT} images"),
            ));
        }
        let text_chars: usize = input
            .parts
            .iter()
            .map(|p| match p {
                EmbedPart::Text(t) => t.chars().count(),
                EmbedPart::Image { .. } => 0,
            })
            .sum();
        if estimate_tokens(input.image_count(), text_chars) > MAX_INPUT_TOKENS {
            return Err(EmbedError::new(
                embed_codes::INPUT_TOO_LARGE,
                format!("input is estimated above {MAX_INPUT_TOKENS} tokens; split it"),
            ));
        }
        let mut parts = Vec::with_capacity(input.parts.len());
        for part in &input.parts {
            parts.push(match part {
                EmbedPart::Text(text) => json!({ "text": text }),
                EmbedPart::Image { bytes, .. } => {
                    let prepared = prepare_image(bytes, self.config.max_image_bytes)?;
                    json!({ "inlineData": { "mimeType": prepared.mime, "data": B64.encode(prepared.bytes) } })
                }
            });
        }
        if parts.is_empty() {
            return Err(EmbedError::new(
                embed_codes::REQUEST,
                "an input needs at least one part",
            ));
        }
        Ok(json!({
            "content": { "parts": parts },
            "embedContentConfig": { "outputDimensionality": self.config.dimension },
        }))
    }

    async fn post_once(&self, body: &Value, token: &str) -> Result<reqwest::Response, EmbedError> {
        self.http
            .post(self.config.endpoint())
            .bearer_auth(token)
            .json(body)
            .send()
            .await
            .map_err(|e| {
                EmbedError::new(
                    embed_codes::REQUEST,
                    format!("embedContent request failed: {e}"),
                )
            })
    }

    /// POST with one 401 refresh and a bounded retry on 429 / 5xx.
    async fn send(&self, body: &Value) -> Result<Value, EmbedError> {
        let mut token = self.tokens.token().await?;
        let mut refreshed = false;
        for attempt in 0..MAX_ATTEMPTS {
            let response = self.post_once(body, &token).await?;
            let status = response.status();
            if status.is_success() {
                return response.json::<Value>().await.map_err(|e| {
                    EmbedError::new(embed_codes::REQUEST, format!("bad response body: {e}"))
                });
            }
            if status.as_u16() == 401 && !refreshed {
                token = self.tokens.refresh().await?;
                refreshed = true;
                continue;
            }
            let retryable = status.as_u16() == 429 || status.is_server_error();
            if retryable && attempt + 1 < MAX_ATTEMPTS {
                tokio::time::sleep(Duration::from_millis(200 * 2u64.pow(attempt))).await;
                continue;
            }
            let code = if status.as_u16() == 401 || status.as_u16() == 403 {
                embed_codes::AUTH
            } else {
                embed_codes::REQUEST
            };
            let detail = response.text().await.unwrap_or_default();
            let detail: String = detail.chars().take(300).collect();
            return Err(EmbedError::new(
                code,
                format!("embedContent returned {status}: {detail}"),
            ));
        }
        Err(EmbedError::new(
            embed_codes::REQUEST,
            "embedContent retries exhausted",
        ))
    }

    fn vector_of(&self, response: &Value) -> Result<Vec<f32>, EmbedError> {
        let values = response
            .pointer("/embedding/values")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                EmbedError::new(embed_codes::REQUEST, "response has no embedding.values")
            })?;
        let mut vector: Vec<f32> = values
            .iter()
            .map(|v| v.as_f64().map(|f| f as f32))
            .collect::<Option<_>>()
            .ok_or_else(|| {
                EmbedError::new(embed_codes::REQUEST, "embedding.values holds a non-number")
            })?;
        if vector.len() != self.config.dimension {
            return Err(EmbedError::new(
                embed_codes::DIMENSION_MISMATCH,
                format!(
                    "expected {} values, got {}",
                    self.config.dimension,
                    vector.len()
                ),
            ));
        }
        normalize(&mut vector);
        Ok(vector)
    }

    async fn embed_one(&self, input: &EmbedInput) -> Result<Vec<f32>, EmbedError> {
        let body = self.request_body(input)?;
        let response = self.send(&body).await?;
        self.vector_of(&response)
    }
}

/// L2-normalise unless the norm is already 1 within tolerance.
fn normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 && (norm - 1.0).abs() > NORM_TOLERANCE {
        vector.iter_mut().for_each(|v| *v /= norm);
    }
}

#[async_trait]
impl MultimodalEmbedder for VertexGeminiEmbedder {
    fn model(&self) -> &str {
        &self.config.model
    }

    fn dimension(&self) -> usize {
        self.config.dimension
    }

    async fn embed(&self, inputs: &[EmbedInput]) -> Result<Vec<Vec<f32>>, EmbedError> {
        let mut vectors = Vec::with_capacity(inputs.len());
        for batch in inputs.chunks(self.config.concurrency) {
            let calls = batch.iter().map(|input| self.embed_one(input));
            for result in futures::future::join_all(calls).await {
                vectors.push(result?);
            }
        }
        Ok(vectors)
    }
}

/// Text-only `EmbeddingProvider` view, for the chunk pipeline and workspaces that embed
/// text only. Chunks carry the document prefix with no title (`title: none`); a query
/// vector comes from [`MultimodalEmbedder`] with the query prefix instead.
pub struct VertexTextEmbedder(pub Arc<VertexGeminiEmbedder>);

#[async_trait]
impl edgequake_llm::traits::EmbeddingProvider for VertexTextEmbedder {
    fn name(&self) -> &str {
        "vertexai"
    }

    fn model(&self) -> &str {
        MultimodalEmbedder::model(self.0.as_ref())
    }

    fn dimension(&self) -> usize {
        self.0.dimension()
    }

    fn max_tokens(&self) -> usize {
        MAX_INPUT_TOKENS
    }

    async fn embed(&self, texts: &[String]) -> edgequake_llm::Result<Vec<Vec<f32>>> {
        let inputs: Vec<EmbedInput> = texts
            .iter()
            .map(|t| EmbedInput::text(document_prefix("", t)))
            .collect();
        self.0
            .embed(&inputs)
            .await
            .map_err(|e| edgequake_llm::LlmError::ApiError(format!("{}: {}", e.code, e.message)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embeddings::vertex_token::TokenSource;
    use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn a_stored_service_account_does_not_need_google_cloud_project() {
        let client = VertexGeminiEmbedder::from_service_account(
            "demo-project",
            r#"{"type":"service_account","project_id":"demo-project"}"#,
            "gemini-embedding-2",
            1536,
        );
        assert!(client.is_ok());
    }

    struct Fixed;

    #[async_trait]
    impl TokenSource for Fixed {
        async fn mint(&self) -> Result<String, EmbedError> {
            Ok("test-token".into())
        }
    }

    #[derive(Clone)]
    struct Fake {
        dimension: usize,
        bodies: Arc<std::sync::Mutex<Vec<Value>>>,
        fail_first_with: Arc<AtomicUsize>,
    }

    async fn handler(
        State(fake): State<Fake>,
        Json(body): Json<Value>,
    ) -> (StatusCode, Json<Value>) {
        fake.bodies.lock().unwrap().push(body);
        if fake.fail_first_with.load(Ordering::SeqCst) > 0 {
            let code = fake.fail_first_with.swap(0, Ordering::SeqCst);
            return (
                StatusCode::from_u16(code as u16).unwrap(),
                Json(json!({"error": "x"})),
            );
        }
        let values: Vec<f32> = (0..fake.dimension).map(|i| (i % 7) as f32 + 1.0).collect();
        (
            StatusCode::OK,
            Json(json!({ "embedding": { "values": values } })),
        )
    }

    async fn serve(fake: Fake) -> String {
        let app = Router::new()
            .route(
                "/v1/projects/{p}/locations/{l}/publishers/google/models/{m}",
                post(handler),
            )
            .with_state(fake);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        format!("http://{addr}")
    }

    fn fake(dimension: usize, fail: usize) -> Fake {
        Fake {
            dimension,
            bodies: Arc::default(),
            fail_first_with: Arc::new(AtomicUsize::new(fail)),
        }
    }

    fn embedder(base: &str, dimension: usize) -> VertexGeminiEmbedder {
        let config =
            VertexGeminiConfig::new("proj", base, "gemini-embedding-2", dimension, 2, 3_500_000)
                .unwrap();
        VertexGeminiEmbedder::new(config, VertexTokenProvider::new(Arc::new(Fixed))).unwrap()
    }

    fn png() -> Vec<u8> {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            8,
            8,
            image::Rgb([9, 99, 199]),
        ));
        let mut out = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn unsupported_dimension_is_refused_at_construction() {
        let e = VertexGeminiConfig::new("p", "http://x", "m", 1408, 4, 0).unwrap_err();
        assert_eq!(e.code, embed_codes::DIMENSION_MISMATCH);
    }

    #[tokio::test]
    async fn body_carries_output_dimensionality_and_no_task_type() {
        let e = embedder("http://unused", 1536);
        let body = e.request_body(&EmbedInput::text("hello")).unwrap();
        assert_eq!(body["embedContentConfig"]["outputDimensionality"], 1536);
        assert!(body.get("taskType").is_none());
        assert_eq!(body["content"]["parts"][0]["text"], "hello");
    }

    #[tokio::test]
    async fn image_part_is_inline_data() {
        let e = embedder("http://unused", 768);
        let body = e
            .request_body(&EmbedInput::image_with_text("image/png", png(), "cap"))
            .unwrap();
        assert_eq!(
            body["content"]["parts"][0]["inlineData"]["mimeType"],
            "image/png"
        );
        assert_eq!(body["content"]["parts"][1]["text"], "cap");
    }

    #[tokio::test]
    async fn more_than_six_images_is_refused() {
        let e = embedder("http://unused", 768);
        let parts = (0..7)
            .map(|_| EmbedPart::Image {
                mime: "image/png".into(),
                bytes: png(),
            })
            .collect();
        let err = e.request_body(&EmbedInput { parts }).unwrap_err();
        assert_eq!(err.code, embed_codes::INPUT_TOO_LARGE);
    }

    #[tokio::test]
    async fn an_input_over_the_token_budget_is_refused_before_http() {
        let e = embedder("http://unused", 768);
        let err = e
            .request_body(&EmbedInput::text("x".repeat(40_000)))
            .unwrap_err();
        assert_eq!(err.code, embed_codes::INPUT_TOO_LARGE);
    }

    #[tokio::test]
    async fn vectors_come_back_normalised_and_in_order() {
        let f = fake(768, 0);
        let base = serve(f.clone()).await;
        let e = embedder(&base, 768);
        let out = e
            .embed(&[
                EmbedInput::text("a"),
                EmbedInput::text("b"),
                EmbedInput::text("c"),
            ])
            .await
            .unwrap();
        assert_eq!(out.len(), 3);
        for v in &out {
            assert_eq!(v.len(), 768);
            let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!((norm - 1.0).abs() < 1e-3, "norm {norm}");
        }
        assert_eq!(f.bodies.lock().unwrap().len(), 3, "one call per input");
    }

    #[tokio::test]
    async fn wrong_width_is_a_dimension_mismatch() {
        let base = serve(fake(1536, 0)).await;
        let e = embedder(&base, 768);
        let err = e.embed(&[EmbedInput::text("a")]).await.unwrap_err();
        assert_eq!(err.code, embed_codes::DIMENSION_MISMATCH);
    }

    #[tokio::test]
    async fn a_503_is_retried() {
        let f = fake(768, 503);
        let base = serve(f.clone()).await;
        let e = embedder(&base, 768);
        e.embed(&[EmbedInput::text("a")])
            .await
            .expect("retry succeeds");
        assert_eq!(f.bodies.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn a_401_refreshes_the_token_once() {
        let f = fake(768, 401);
        let base = serve(f.clone()).await;
        let e = embedder(&base, 768);
        e.embed(&[EmbedInput::text("a")])
            .await
            .expect("second call succeeds");
        assert_eq!(f.bodies.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn a_403_is_an_auth_error() {
        let base = serve(fake(768, 403)).await;
        let e = embedder(&base, 768);
        let err = e.embed(&[EmbedInput::text("a")]).await.unwrap_err();
        assert_eq!(err.code, embed_codes::AUTH);
    }

    #[test]
    fn missing_project_is_embedding_auth_not_a_mock() {
        let saved = std::env::var("GOOGLE_CLOUD_PROJECT").ok();
        std::env::remove_var("GOOGLE_CLOUD_PROJECT");
        let err = VertexGeminiConfig::from_env("gemini-embedding-2", 1536).unwrap_err();
        assert_eq!(err.code, embed_codes::AUTH);
        if let Some(p) = saved {
            std::env::set_var("GOOGLE_CLOUD_PROJECT", p);
        }
    }

    #[test]
    fn a_saved_project_builds_the_client_without_the_environment() {
        let saved = std::env::var("GOOGLE_CLOUD_PROJECT").ok();
        std::env::remove_var("GOOGLE_CLOUD_PROJECT");
        let client =
            VertexGeminiEmbedder::for_saved_project("mobioos-506015", "gemini-embedding-2", 1536)
                .expect("saved project");
        assert!(client
            .config
            .endpoint()
            .contains("/projects/mobioos-506015/"));
        assert!(VertexGeminiConfig::from_env("gemini-embedding-2", 1536).is_err());
        if let Some(project) = saved {
            std::env::set_var("GOOGLE_CLOUD_PROJECT", project);
        }
    }

    #[test]
    fn endpoint_is_global_embed_content() {
        let c =
            VertexGeminiConfig::new("proj", "http://h", "gemini-embedding-2", 768, 1, 0).unwrap();
        assert_eq!(
            c.endpoint(),
            "http://h/v1/projects/proj/locations/global/publishers/google/models/gemini-embedding-2:embedContent"
        );
    }
}
