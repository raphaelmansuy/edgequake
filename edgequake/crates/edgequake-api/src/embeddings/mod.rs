//! SPEC-180 08: embedding clients that are not `EmbeddingProvider` text-only adapters.
//!
//! This module is also the one place that decides whether a (provider, model) pair is the
//! Vertex multimodal model, so the factory, the create check and the ingest path agree.

pub mod image_limits;
pub mod vertex_gemini;
pub mod vertex_token;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use edgequake_core::multimodal_embed::{EmbedError, MultimodalEmbedder};

pub use vertex_gemini::{VertexGeminiConfig, VertexGeminiEmbedder, VertexTextEmbedder};

/// The catalog id of the only model that embeds text and images in this pack.
pub const MULTIMODAL_MODEL: &str = "gemini-embedding-2";

/// True for the Vertex multimodal embedding model, whatever the provider spelling.
pub fn is_vertex_multimodal(provider: &str, model: &str) -> bool {
    let provider = provider.trim().to_ascii_lowercase();
    matches!(
        provider.as_str(),
        "vertexai" | "vertex" | "vertex-ai" | "vertex_ai"
    ) && model.trim().eq_ignore_ascii_case(MULTIMODAL_MODEL)
}

/// The API error of an embedding failure: 503 for credentials, 500 for a width the vendor
/// got wrong, 422 for everything the caller can fix (an image that is too large, animated or
/// corrupt) and 502 for a failed vendor call.
pub fn api_error(error: &EmbedError) -> crate::error::ApiError {
    use axum::http::StatusCode;
    use edgequake_core::multimodal_embed::embed_codes as c;
    let status = match error.code {
        c::AUTH => StatusCode::SERVICE_UNAVAILABLE,
        c::DIMENSION_MISMATCH => StatusCode::INTERNAL_SERVER_ERROR,
        c::REQUEST => StatusCode::BAD_GATEWAY,
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    };
    crate::error::ApiError::Coded {
        status,
        code: error.code,
        message: error.message.clone(),
    }
}

type ClientKey = (String, usize, String, String);

/// One client per (model, dimension, project, endpoint). It owns the HTTP pool.
pub fn shared_embedder(
    model: &str,
    dimension: usize,
) -> Result<Arc<VertexGeminiEmbedder>, EmbedError> {
    static CLIENTS: OnceLock<Mutex<HashMap<ClientKey, Arc<VertexGeminiEmbedder>>>> =
        OnceLock::new();
    let config = VertexGeminiConfig::from_env(model, dimension)?;
    let key = (
        model.to_string(),
        dimension,
        config.project.clone(),
        config.base_url.clone(),
    );
    let mut clients = CLIENTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(client) = clients.get(&key) {
        return Ok(Arc::clone(client));
    }
    let client = Arc::new(VertexGeminiEmbedder::new(
        config,
        vertex_token::VertexTokenProvider::from_env(),
    )?);
    clients.insert(key, Arc::clone(&client));
    Ok(client)
}

/// The multimodal client of a pair, or `None` when the pair is not the Vertex model.
pub fn multimodal_embedder_for(
    provider: &str,
    model: &str,
    dimension: usize,
) -> Option<Result<Arc<dyn MultimodalEmbedder>, EmbedError>> {
    is_vertex_multimodal(provider, model)
        .then(|| shared_embedder(model, dimension).map(|c| c as Arc<dyn MultimodalEmbedder>))
}

/// The text-only `EmbeddingProvider` view of the Vertex model, or `None` for other pairs.
pub fn text_provider_for(
    provider: &str,
    model: &str,
    dimension: usize,
) -> Option<Result<Arc<dyn edgequake_llm::EmbeddingProvider>, EmbedError>> {
    is_vertex_multimodal(provider, model).then(|| {
        shared_embedder(model, dimension).map(|client| {
            Arc::new(VertexTextEmbedder(client)) as Arc<dyn edgequake_llm::EmbeddingProvider>
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_vertex_pair_is_multimodal() {
        assert!(is_vertex_multimodal("vertexai", "gemini-embedding-2"));
        assert!(is_vertex_multimodal("Vertex", "Gemini-Embedding-2"));
        assert!(!is_vertex_multimodal("vertexai", "gemini-embedding-001"));
        assert!(!is_vertex_multimodal("openai", "gemini-embedding-2"));
    }
}
