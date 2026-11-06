//! SPEC-180 08: a workspace that embeds images needs an embedding model that does.
//!
//! One check for create, update and rebuild (DRY). The pure part judges a catalog; the
//! async part loads the catalog and probes one small image with the real client, so a
//! deployment that cannot embed an image fails on the model step and not on the first PDF.
//! The probe itself lives in [`super::embed_capability_probe`], because the member picker
//! asks the same question and must get the same answer.

use edgequake_core::model_policy::CatalogEntry;
use edgequake_core::rag_profile::{codes, EmbeddingModalities, ProfileError};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Set to `1` to skip the create-time probe (tests without a credential).
pub const SKIP_PROBE_ENV: &str = "EDGEQUAKE_SKIP_EMBED_PROBE";

/// 422 `embedding_model_not_multimodal` unless the pair embeds images.
pub fn judge_catalog(
    entries: &[CatalogEntry],
    provider: &str,
    model: &str,
) -> Result<(), ProfileError> {
    let provider = provider.trim().to_lowercase();
    let embeds_image = entries
        .iter()
        .find(|e| e.provider == provider && e.model == model.trim())
        .is_some_and(|e| e.facts.embeds_image());
    if embeds_image {
        return Ok(());
    }
    Err(ProfileError::new(
        codes::EMBEDDING_MODEL_NOT_MULTIMODAL,
        format!("{provider}/{model} cannot embed images; pick an image-capable embedding model or use 'text' only"),
    ))
}

/// True when the deployment has opted out of the create-time probe (tests, offline fleets).
pub fn probe_skipped() -> bool {
    std::env::var(SKIP_PROBE_ENV).is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// Embed one small image with the real client. The workspace is asking it to read images for
/// the life of the workspace, so an image is what the probe must send: a text embed passes on
/// a deployment the image endpoint refuses, and the wizard then fails on the last step.
async fn probe(state: &AppState, provider: &str, model: &str, dimension: usize) -> ApiResult<()> {
    let Some(outcome) = probe_image_embedding(state, provider, model, dimension).await else {
        // A pair this build cannot probe keeps the catalog-only answer from `judge_catalog`.
        return Ok(());
    };
    outcome.map_err(|error| {
        ApiError::unprocessable_coded(
            super::embed_capability_probe::failure_code(&error),
            super::embed_capability_probe::action_message(&error),
        )
    })
}

/// Live image probe. A saved Vertex service account is used before the process environment,
/// so a deployment without `GOOGLE_CLOUD_PROJECT` can still prove `gemini-embedding-2`.
pub async fn probe_image_embedding(
    state: &AppState,
    provider: &str,
    model: &str,
    dimension: usize,
) -> Option<Result<(), edgequake_core::multimodal_embed::EmbedError>> {
    use edgequake_core::multimodal_embed::{
        embed_codes, EmbedError, EmbedInput, MultimodalEmbedder,
    };

    #[cfg(feature = "postgres")]
    if crate::embeddings::is_vertex_multimodal(provider, model) {
        if let Some(pool) = state.optional_pg_pool() {
            match crate::providers::connection_store::fleet_vertex_embedder(pool, model, dimension)
                .await
            {
                Ok(Some(client)) => {
                    let outcome = client
                        .embed(&[EmbedInput::image(
                            "image/png",
                            super::embed_capability_probe::probe_png().to_vec(),
                        )])
                        .await
                        .map(|_| ());
                    super::embed_capability_probe::remember(provider, model, &outcome);
                    return Some(outcome);
                }
                Ok(None) => {}
                Err(message) => {
                    let outcome = Err(EmbedError::new(embed_codes::AUTH, message));
                    super::embed_capability_probe::remember(provider, model, &outcome);
                    return Some(outcome);
                }
            }
        }
    }
    super::embed_capability_probe::probe_image(provider, model, dimension).await
}

/// Judge the embedding pair of a workspace against its modalities.
///
/// `text` workspaces pass untouched (the text path has no extra rule). A `text,image`
/// workspace must name an image-capable pair, and the Vertex model is probed unless
/// [`SKIP_PROBE_ENV`] is set.
pub async fn ensure_embedding_matches_modalities(
    state: &AppState,
    modalities: EmbeddingModalities,
    provider: &str,
    model: &str,
    dimension: usize,
) -> ApiResult<()> {
    if !modalities.includes_image() {
        return Ok(());
    }
    let mut catalog = crate::services::model_policy_catalog::load_catalog(state).await;
    crate::services::model_policy_catalog::append_connection_models(state, &mut catalog).await?;
    let entries: Vec<CatalogEntry> = catalog.into_iter().map(|item| item.entry).collect();
    judge_catalog(&entries, provider, model)?;
    if !probe_skipped() {
        probe(state, provider, model, dimension).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_core::model_policy::ModelFacts;

    fn entry(provider: &str, model: &str, vision: bool) -> CatalogEntry {
        CatalogEntry {
            provider: provider.into(),
            model: model.into(),
            facts: ModelFacts {
                embedding: true,
                vision,
                registered: true,
                ..ModelFacts::default()
            },
        }
    }

    #[test]
    fn an_image_embedding_card_passes() {
        let entries = [entry("vertexai", "gemini-embedding-2", true)];
        assert!(judge_catalog(&entries, "VertexAI", "gemini-embedding-2").is_ok());
    }

    #[test]
    fn a_text_only_card_is_not_multimodal() {
        let entries = [entry("openai", "text-embedding-3-small", false)];
        let err = judge_catalog(&entries, "openai", "text-embedding-3-small").unwrap_err();
        assert_eq!(err.code, codes::EMBEDDING_MODEL_NOT_MULTIMODAL);
    }

    #[test]
    fn an_unknown_pair_is_not_multimodal() {
        let err = judge_catalog(&[], "vertexai", "gemini-embedding-2").unwrap_err();
        assert_eq!(err.code, codes::EMBEDDING_MODEL_NOT_MULTIMODAL);
    }
}
