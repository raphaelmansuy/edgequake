//! SPEC-180 08: can this deployment *really* embed an image with this pair, right now?
//!
//! A static catalog card is a promise about a **model**. It says nothing about whether this
//! project and this workload identity may call it. The picker used to trust the card, so it
//! offered `vertexai/gemini-embedding-2` on a deployment whose identity had no
//! `aiplatform.endpoints.predict` on the configured project: the user configured six wizard
//! steps, reached Review, and only then got a raw Vertex 403.
//!
//! One probe answers the question for every caller, so the answer cannot drift:
//! - the catalog (`/models/eligible`), which stops offering a pair that cannot work;
//! - create / update / rebuild, which refuse with the same code and words;
//! - the wizard, which shows the reason on the model step instead of on the last one.
//!
//! Results are cached per pair for [`CACHE_TTL`]; a picker read is not a vendor call.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use edgequake_core::multimodal_embed::{embed_codes, EmbedError, EmbedInput};

/// How long one probe answer, success or failure, is reused.
pub const CACHE_TTL: Duration = Duration::from_secs(300);

/// Ceiling for a picker-time probe, so a hung network cannot stall the model step.
pub const PICKER_TIMEOUT: Duration = Duration::from_secs(10);

/// The smallest valid PNG (1×1): the vendor answers, and the call stays cheap.
pub fn probe_png() -> &'static [u8] {
    PROBE_PNG
}

const PROBE_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

type PairKey = (String, String);

fn pair_key(provider: &str, model: &str) -> PairKey {
    (
        provider.trim().to_ascii_lowercase(),
        model.trim().to_string(),
    )
}

#[derive(Clone)]
struct Cached {
    outcome: Result<(), EmbedError>,
    at: Instant,
}

fn cache() -> &'static Mutex<HashMap<PairKey, Cached>> {
    static CACHE: OnceLock<Mutex<HashMap<PairKey, Cached>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock() -> std::sync::MutexGuard<'static, HashMap<PairKey, Cached>> {
    cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The last probe of this pair while it is fresh.
pub fn cached(provider: &str, model: &str) -> Option<Result<(), EmbedError>> {
    let key = pair_key(provider, model);
    let mut guard = lock();
    let hit = guard.get(&key)?;
    if hit.at.elapsed() >= CACHE_TTL {
        guard.remove(&key);
        return None;
    }
    Some(hit.outcome.clone())
}

/// Remember an answer so the picker and the wizard agree without a second vendor call.
pub fn remember(provider: &str, model: &str, outcome: &Result<(), EmbedError>) {
    lock().insert(
        pair_key(provider, model),
        Cached {
            outcome: outcome.clone(),
            at: Instant::now(),
        },
    );
}

/// Drop every cached answer (credential change, tests).
pub fn forget_all() {
    lock().clear();
}

/// Embed one image with the pair. `None` means this module cannot judge the pair: it is not
/// an image embedder this build knows, so the caller keeps its catalog-only rule.
pub async fn probe_image(
    provider: &str,
    model: &str,
    dimension: usize,
) -> Option<Result<(), EmbedError>> {
    if let Some(hit) = cached(provider, model) {
        return Some(hit);
    }
    let client = crate::embeddings::multimodal_embedder_for(provider, model, dimension)?;
    let outcome = match client {
        Ok(client) => client
            .embed(&[EmbedInput::image("image/png", PROBE_PNG.to_vec())])
            .await
            .map(|_| ()),
        Err(error) => Err(error),
    };
    remember(provider, model, &outcome);
    Some(outcome)
}

/// Picker-time probe: the same call under [`PICKER_TIMEOUT`]. A timeout is a failure here,
/// because a deployment that cannot answer in ten seconds cannot serve an ingest either.
pub async fn probe_image_for_picker(
    provider: &str,
    model: &str,
    dimension: usize,
) -> Option<Result<(), EmbedError>> {
    if let Some(hit) = cached(provider, model) {
        return Some(hit);
    }
    let pending = probe_image(provider, model, dimension);
    match tokio::time::timeout(PICKER_TIMEOUT, pending).await {
        Ok(found) => found,
        Err(_) => {
            let outcome = Err(EmbedError::new(
                embed_codes::REQUEST,
                format!(
                    "the image embedding probe did not answer within {}s",
                    PICKER_TIMEOUT.as_secs()
                ),
            ));
            remember(provider, model, &outcome);
            Some(outcome)
        }
    }
}

/// Code a failed image probe raises. The vendor's own code is kept when the caller can act
/// on it (`image_too_large`, `image_decode_failed`); auth collapses to `embedding_auth`
/// because "no credential" is the one failure a deployment admin fixes.
pub fn failure_code(error: &EmbedError) -> &'static str {
    error.code
}

/// Why the pair cannot embed an image here, in words a deployment admin can act on.
///
/// The vendor text is kept but trimmed: a toast is not a log file. A permission failure
/// names the project the server actually called, because that is the thing to change.
pub fn action_message(error: &EmbedError) -> String {
    let detail = trim_vendor_text(&error.message);
    match error.code {
        embed_codes::AUTH => format!(
            "Vertex refused the image embedding call for project {project}: {detail}. \
             Grant this identity aiplatform.endpoints.predict, or set GOOGLE_CLOUD_PROJECT to \
             the project that grants it.",
            project = vertex_project_label()
        ),
        _ => detail,
    }
}

/// The project the last Vertex call used, so the message names the real one.
fn vertex_project_label() -> String {
    std::env::var("GOOGLE_CLOUD_PROJECT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "(GOOGLE_CLOUD_PROJECT is not set)".to_string())
}

/// First line of a vendor body, cut at a sentence end, so a 403 does not flood the toast.
pub fn trim_vendor_text(message: &str) -> String {
    let flat = message.split_whitespace().collect::<Vec<_>>().join(" ");
    let flat = flat
        .split("Remediate access")
        .next()
        .unwrap_or(&flat)
        .trim()
        .to_string();
    const MAX: usize = 240;
    if flat.chars().count() <= MAX {
        return flat;
    }
    let cut: String = flat.chars().take(MAX).collect();
    format!("{cut}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_probe_image_is_a_real_png() {
        assert!(crate::embeddings::image_limits::looks_like_image(PROBE_PNG));
    }

    #[test]
    fn a_refusal_names_the_project_and_the_fix() {
        let error = EmbedError::new(
            embed_codes::AUTH,
            "embedContent returned 403 Forbidden: {\"error\":{\"code\":403,\
             \"message\":\"Permission 'aiplatform.endpoints.predict' denied\"}}",
        );
        let message = action_message(&error);
        assert!(message.contains("GOOGLE_CLOUD_PROJECT"), "{message}");
        assert!(
            message.contains("aiplatform.endpoints.predict"),
            "{message}"
        );
        assert!(message.contains("403"), "{message}");
    }

    #[test]
    fn vendor_text_is_flat_and_short() {
        let noisy = format!("line one\n  line two {}", "x".repeat(600));
        let trimmed = trim_vendor_text(&noisy);
        assert!(!trimmed.contains('\n'));
        assert!(trimmed.chars().count() <= 241);
    }

    #[test]
    fn a_cached_answer_is_reused_until_forgotten() {
        forget_all();
        remember("vertexai", "gemini-embedding-2", &Ok(()));
        assert_eq!(cached("VertexAI", " gemini-embedding-2 "), Some(Ok(())));
        forget_all();
        assert_eq!(cached("vertexai", "gemini-embedding-2"), None);
    }
}
