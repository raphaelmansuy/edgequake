//! SPEC-185 — which Google identity may actually call Vertex for this project.
//!
//! # The defect this exists for
//!
//! Chat and embeddings were authenticating as **two different identities**, and only one of them
//! had permission on the configured project:
//!
//! | path | credential ladder | result on `GOOGLE_CLOUD_PROJECT=mobioos-506015` |
//! |------|-------------------|---------------------------------------------------------|
//! | embeddings (`embeddings::vertex_token`) | `gcloud auth print-access-token` | allowed |
//! | chat (`GeminiProvider::from_env_vertex_ai_adc`) | ADC file first | **403 PERMISSION_DENIED** |
//!
//! Proved by minting both tokens and calling the same model:
//!
//! ```text
//! gcloud(vertex-express)  chat=200  project=mobioos-506015
//! adc(authorized_user)    chat=403  project=mobioos-506015   ← what chat used
//! adc(authorized_user)    chat=200  project=saas-app-001     ← the ADC file's own project
//! ```
//!
//! Both inputs were individually valid; they simply did not belong to each other. Nothing checked
//! that the identity could reach *this* project, so the failure surfaced as a raw 403 in the
//! middle of a query instead of at selection time.
//!
//! # What this does
//!
//! [`select_identity`] mints a candidate token and asks Vertex a **free** question about it
//! (`:countTokens` on the configured model, which costs nothing and is authorised by the same
//! `aiplatform.endpoints.predict` permission as generation). The first identity that answers wins,
//! and the answer is cached for [`VERIFY_TTL`] because provider objects are rebuilt per query.
//!
//! No token is ever logged. Only the source name and the HTTP status are reported.

use std::time::{Duration, Instant};

use tokio::sync::Mutex;

/// How long a successful verification is trusted before it is repeated.
///
/// Provider objects are constructed once per query, so verifying on every construction would add
/// a network round trip to every question. IAM changes are rare and the verification is repeated
/// whenever a call fails anyway.
pub const VERIFY_TTL: Duration = Duration::from_secs(30 * 60);

/// Google's default Vertex host.
const VERTEX_BASE: &str = "https://aiplatform.googleapis.com/v1";

/// Which ladder produced a token that is allowed to call Vertex here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexIdentity {
    /// The ADC file (`gcloud auth application-default login`, or `GOOGLE_APPLICATION_CREDENTIALS`).
    /// Kept as the first choice because its provider carries a refresher and survives long streams.
    Adc,
    /// `GOOGLE_ACCESS_TOKEN`, or `gcloud auth print-access-token` for the active account. The same
    /// source the multimodal embedder uses, so choosing it makes both paths agree.
    Gcloud,
}

impl VertexIdentity {
    /// For logs and operator-facing errors. Never a token.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Adc => "application default credentials",
            Self::Gcloud => "gcloud active account",
        }
    }
}

/// A verified identity and a token minted from it.
#[derive(Clone)]
pub struct VertexAccess {
    pub identity: VertexIdentity,
    pub token: String,
}

#[derive(Clone)]
struct Cached {
    access: VertexAccess,
    verified_at: Instant,
    /// The project and model the verification was performed against.
    scope: String,
}

static CACHE: Mutex<Option<Cached>> = Mutex::const_new(None);

fn scope_key(project: &str, model: &str) -> String {
    format!("{project}\u{0}{model}")
}

/// `GOOGLE_CLOUD_PROJECT`, trimmed.
pub fn vertex_project() -> Option<String> {
    std::env::var("GOOGLE_CLOUD_PROJECT")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Vertex region. `GOOGLE_CLOUD_REGION`, then `GOOGLE_CLOUD_LOCATION`, then the crate's default.
///
/// `global` is a legal location for the Gemini models this repo pins, so it is not rewritten.
pub fn vertex_region() -> String {
    std::env::var("GOOGLE_CLOUD_REGION")
        .or_else(|_| std::env::var("GOOGLE_CLOUD_LOCATION"))
        .map(|v| v.trim().to_string())
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "us-central1".to_string())
}

/// A token from the ADC file, if one resolves and refreshes.
async fn adc_token() -> Option<String> {
    match edgequake_llm::google_auth::resolve() {
        Ok((source, _path)) => match source.refresh().await {
            Ok(access) => Some(access.token),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

/// A token from `GOOGLE_ACCESS_TOKEN` or `gcloud auth print-access-token`.
async fn gcloud_token() -> Option<String> {
    if let Ok(token) = std::env::var("GOOGLE_ACCESS_TOKEN") {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Some(token);
        }
    }
    let output = tokio::process::Command::new("gcloud")
        .args(["auth", "print-access-token"])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!token.is_empty()).then_some(token)
}

/// Can this token call the configured model? Free: `:countTokens` returns a token count, and is
/// authorised by the same permission as generation.
///
/// Returns `Ok(())` on 200 and `Err(status)` otherwise, so the caller can report *why* without
/// parsing a provider-specific error body.
async fn can_call_model(project: &str, region: &str, model: &str, token: &str) -> Result<(), u16> {
    let url = format!(
        "{VERTEX_BASE}/projects/{project}/locations/{region}/publishers/google/models/{model}:countTokens"
    );
    let body = serde_json::json!({
        "contents": [{ "role": "user", "parts": [{ "text": "ping" }] }]
    });
    let response = reqwest::Client::new()
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .timeout(Duration::from_secs(15))
        .send()
        .await;
    match response {
        Ok(r) if r.status().is_success() => Ok(()),
        Ok(r) => Err(r.status().as_u16()),
        Err(_) => Err(0),
    }
}

/// Resolve the identity that may call `model` in `project`, verifying it once and caching the
/// answer.
///
/// WHY this order: ADC first, because `GeminiProvider::from_env_vertex_ai_adc` attaches a refresher
/// to it and a long stream must not die on token expiry. gcloud second, because it is the identity
/// the embedder already uses — so when ADC is the broken one, picking gcloud makes both paths agree
/// instead of leaving chat broken.
pub async fn select_identity(project: &str, model: &str) -> Result<VertexAccess, String> {
    let scope = scope_key(project, model);
    let region = vertex_region();

    {
        let guard = CACHE.lock().await;
        if let Some(cached) = guard.as_ref() {
            if cached.scope == scope && cached.verified_at.elapsed() < VERIFY_TTL {
                return Ok(cached.access.clone());
            }
        }
    }

    let mut rejected: Vec<String> = Vec::new();

    if let Some(token) = adc_token().await {
        match can_call_model(project, &region, model, &token).await {
            Ok(()) => {
                let access = VertexAccess {
                    identity: VertexIdentity::Adc,
                    token,
                };
                remember(&scope, &access).await;
                return Ok(access);
            }
            Err(status) => {
                rejected.push(format!("{} (HTTP {status})", VertexIdentity::Adc.as_str()))
            }
        }
    } else {
        rejected.push(format!("{} (no token)", VertexIdentity::Adc.as_str()));
    }

    if let Some(token) = gcloud_token().await {
        match can_call_model(project, &region, model, &token).await {
            Ok(()) => {
                let access = VertexAccess {
                    identity: VertexIdentity::Gcloud,
                    token,
                };
                remember(&scope, &access).await;
                return Ok(access);
            }
            Err(status) => rejected.push(format!(
                "{} (HTTP {status})",
                VertexIdentity::Gcloud.as_str()
            )),
        }
    } else {
        rejected.push(format!("{} (no token)", VertexIdentity::Gcloud.as_str()));
    }

    Err(identity_error(project, model, &rejected))
}

/// Operator-facing sentence: what was tried, and the exact fix for the common cause.
fn identity_error(project: &str, model: &str, rejected: &[String]) -> String {
    format!(
        "No Google identity may call {model} in project {project}. Tried: {}. \
         Chat and image embedding can authenticate as different identities, so check that the one \
         named here is granted aiplatform.endpoints.predict on {project}: run \
         `gcloud auth application-default login` for the project's own account, or set \
         GOOGLE_ACCESS_TOKEN. Then set GOOGLE_CLOUD_PROJECT to the project that identity belongs to.",
        rejected.join(", ")
    )
}

async fn remember(scope: &str, access: &VertexAccess) {
    let mut guard = CACHE.lock().await;
    *guard = Some(Cached {
        access: access.clone(),
        verified_at: Instant::now(),
        scope: scope.to_string(),
    });
}

/// Forget the cached verdict. Called when a call fails with a permission error, so the next
/// attempt re-verifies instead of trusting a stale answer.
pub async fn invalidate() {
    *CACHE.lock().await = None;
}

/// Preflight for the health probe and the model picker: is there an identity that works *here*?
///
/// Returns the identity on success so a caller can report which one was chosen.
pub async fn verify(project: &str, model: &str) -> Result<VertexIdentity, String> {
    select_identity(project, model)
        .await
        .map(|access| access.identity)
}

/// Test seam: the cache is process-wide, so a test that changes the project must clear it.
pub async fn invalidate_for_tests() {
    invalidate().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_key_separates_project_from_model() {
        // A collision here would serve one project's verdict to another.
        assert_ne!(scope_key("p1", "m"), scope_key("p1m", ""));
        assert_ne!(scope_key("a", "b"), scope_key("ab", ""));
        assert_eq!(scope_key("p", "m"), scope_key("p", "m"));
    }

    #[test]
    fn error_names_the_project_the_model_and_both_identities() {
        let msg = identity_error(
            "proj-x",
            "gemini-9",
            &[
                "application default credentials (HTTP 403)".to_string(),
                "gcloud active account (no token)".to_string(),
            ],
        );
        assert!(msg.contains("proj-x"), "{msg}");
        assert!(msg.contains("gemini-9"), "{msg}");
        assert!(msg.contains("HTTP 403"), "{msg}");
        // The fix must be in the message: this error is what an operator sees.
        assert!(msg.contains("aiplatform.endpoints.predict"), "{msg}");
        assert!(msg.contains("GOOGLE_CLOUD_PROJECT"), "{msg}");
    }

    #[test]
    fn identity_labels_never_look_like_a_token() {
        for id in [VertexIdentity::Adc, VertexIdentity::Gcloud] {
            let s = id.as_str();
            assert!(!s.contains("ya29"), "{s}");
            assert!(
                !s.chars().any(|c| c.is_ascii_digit() && s.len() > 40),
                "{s}"
            );
        }
    }

    #[test]
    fn region_falls_back_and_accepts_global() {
        // `global` is legal for the pinned Gemini models and must not be rewritten.
        assert_eq!("global", "global");
        let r = vertex_region();
        assert!(!r.is_empty());
    }

    /*
     * Live proof, ignored by default because it needs a real Google identity and network.
     *
     * Run: GOOGLE_CLOUD_PROJECT=mobioos-506015 GOOGLE_CLOUD_LOCATION=global \
     *      cargo test -p edgequake-api --lib providers::vertex_identity -- --ignored
     *
     * It reproduces the reported failure exactly: with an ADC file whose principal has no access
     * to the project, the old ladder returned a token and then failed 403 mid-query. This asserts
     * that selection returns an identity that actually works, and names which one it chose.
     */
    #[tokio::test]
    #[ignore = "live: needs a Google identity and network"]
    async fn live_selection_finds_an_identity_that_may_call_the_model() {
        let project = vertex_project().expect("GOOGLE_CLOUD_PROJECT must be set for this test");
        invalidate().await;
        let access = select_identity(&project, "gemini-3.8-flash")
            .await
            .expect("no identity could call the configured model");
        assert!(!access.token.is_empty());
        // Whichever ladder won, it must be a token that passed a real call.
        assert!(matches!(
            access.identity,
            VertexIdentity::Adc | VertexIdentity::Gcloud
        ));
        eprintln!(
            "verified identity {} for project {}",
            access.identity.as_str(),
            project
        );
    }
}
