use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// PDF upload options.
#[derive(Debug, Clone, Default)]
pub struct PdfUploadOptions {
    /// Enable vision LLM processing (default: true).
    pub enable_vision: bool,
    /// Vision provider to use. None = use workspace config then server default.
    /// Explicitly set by form field `vision_provider`.
    pub vision_provider: Option<String>,
    /// Vision model override. None = use workspace config then provider default.
    /// Explicitly set by form field `vision_model`.
    pub vision_model: Option<String>,
    /// Document title (optional).
    pub title: Option<String>,
    /// Custom metadata (optional).
    pub metadata: Option<serde_json::Value>,
    /// Batch tracking ID (optional).
    pub track_id: Option<String>,
    /// Force re-indexing of duplicate PDF (default: false).
    /// WHY (OODA-08): When true, existing graph/vector data is cleared
    /// and the document is re-processed with current LLM/config.
    pub force_reindex: bool,
}

impl PdfUploadOptions {
    /// Get the resolved vision provider (with fallback to server default).
    ///
    /// WHY: When no vision provider is explicitly configured, fall back to the
    /// server's main LLM provider (EDGEQUAKE_LLM_PROVIDER env var) rather than
    /// hardcoding "openai". This ensures PDF vision extraction works out-of-the-box
    /// for Ollama deployments that have no OPENAI_API_KEY.
    pub fn resolved_vision_provider(&self) -> String {
        // WHY filter empty strings: same Docker Compose ${VAR:-} → "" issue as above.
        self.vision_provider
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                std::env::var("EDGEQUAKE_LLM_PROVIDER")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "ollama".to_string())
            })
    }

    /// Get the vision model to use (with fallback from provider).
    ///
    /// WHY filter empty strings: if workspace stored an empty model string,
    /// treat it the same as "not configured" and fall back to the provider default.
    pub fn vision_model(&self) -> String {
        self.vision_model
            .clone()
            .filter(|s| !s.is_empty()) // treat "" same as None
            .unwrap_or_else(|| default_vision_model_for_provider(&self.resolved_vision_provider()))
    }
}

/// Return a sensible default vision model for the given provider.
///
/// WHY: Different providers have different default multimodal models.
/// For Ollama, use the configured LLM model if set (multimodal models
/// like gemma4 support vision natively). For OpenAI, use gpt-4.1-nano.
///
/// WHY filter empty strings: Docker Compose `${VAR:-}` maps an unset host
/// variable to the empty string "" inside the container. `std::env::var`
/// returns `Ok("")` for that case, so the `.or_else` chain never fires and
/// the caller receives an empty model name which Ollama rejects with
/// `{"error":"model is required"}`. We treat "" the same as "unset".
pub(crate) fn default_vision_model_for_provider(provider: &str) -> String {
    // Filter empty strings: Docker Compose ${VAR:-} maps unset vars to "" in containers.
    let env_vision = std::env::var("EDGEQUAKE_VISION_MODEL")
        .ok()
        .filter(|s| !s.is_empty());
    let env_llm = std::env::var("EDGEQUAKE_LLM_MODEL")
        .ok()
        .filter(|s| !s.is_empty());
    match provider {
        "openai" => env_vision.unwrap_or_else(|| "gpt-4.1-nano".to_string()),
        _ => env_vision
            .or(env_llm)
            .unwrap_or_else(|| "gemma3:latest".to_string()),
    }
}

/// PDF upload response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfUploadResponse {
    /// Generated PDF ID.
    pub pdf_id: String,

    /// Associated document ID (null during processing).
    pub document_id: Option<String>,

    /// Processing status.
    pub status: String,

    /// Background task ID.
    pub task_id: String,

    /// Batch tracking ID (if provided).
    pub track_id: Option<String>,

    /// Human-readable message.
    pub message: String,

    /// Estimated processing time in seconds.
    pub estimated_time_seconds: u64,

    /// PDF metadata.
    pub metadata: PdfMetadata,

    /// ID of the existing duplicate PDF, present when status is "duplicate".
    /// WHY: Frontend uses this field to show the DuplicateUploadDialog and
    /// offer the user a choice to reprocess or skip the duplicate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_of: Option<String>,
}

/// PDF metadata in response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfMetadata {
    /// Original filename.
    pub filename: String,

    /// File size in bytes.
    pub file_size_bytes: i64,

    /// Number of pages (if detected).
    pub page_count: Option<i32>,

    /// SHA-256 checksum.
    pub sha256_checksum: String,

    /// Vision enabled flag.
    pub vision_enabled: bool,

    /// Vision model to use.
    pub vision_model: Option<String>,
}

/// PDF status response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfStatusResponse {
    /// PDF ID.
    pub pdf_id: String,

    /// Associated document ID (if completed).
    pub document_id: Option<String>,

    /// Processing status.
    pub status: String,

    /// Processing duration in milliseconds (if completed).
    pub processing_duration_ms: Option<i64>,

    /// PDF metadata.
    pub metadata: PdfStatusMetadata,

    /// Extraction errors (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<serde_json::Value>,
}

/// PDF status metadata.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfStatusMetadata {
    /// Original filename.
    pub filename: String,

    /// Number of pages.
    pub page_count: Option<i32>,

    /// Extraction method used (if completed).
    pub extraction_method: Option<String>,

    /// Vision model used (if applicable).
    pub vision_model: Option<String>,

    /// When processing completed.
    pub processed_at: Option<String>,
}

/// PDF list query parameters.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListPdfsQuery {
    /// Filter by status.
    #[serde(default)]
    pub status: Option<String>,

    /// Page number (1-indexed).
    #[serde(default = "default_page")]
    pub page: usize,

    /// Page size.
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    20
}

/// PDF list response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListPdfsResponse {
    /// PDF items.
    pub items: Vec<PdfListItem>,

    /// Pagination info.
    pub pagination: PdfPaginationInfo,
}

/// PDF list item.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfListItem {
    /// PDF ID.
    pub pdf_id: String,

    /// Original filename.
    pub filename: String,

    /// Processing status.
    pub status: String,

    /// File size in bytes.
    pub file_size_bytes: i64,

    /// Number of pages.
    pub page_count: Option<i32>,

    /// When uploaded.
    pub created_at: String,

    /// When processed.
    pub processed_at: Option<String>,
}

/// Pagination information for PDF listing.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfPaginationInfo {
    /// Current page (1-indexed).
    pub page: usize,

    /// Page size.
    pub page_size: usize,

    /// Total item count.
    pub total_count: i64,

    /// Total pages.
    pub total_pages: usize,
}

// ============================================================================
// Unit tests — vision model/provider resolution
// ============================================================================
//
// WHY: The provider/model mapping is the source of the "gpt-4.1-nano sent to
// Ollama" bug (SPEC-040 mismatch).  These tests lock the invariant that the
// resolved model is always compatible with the resolved provider.

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Helper that clears the relevant env vars, runs `f`, then restores them.
    // WHY: env vars are process-global; isolating them prevents test pollution.
    fn with_clean_vision_env<F: FnOnce()>(f: F) {
        let prev_vision = env::var("EDGEQUAKE_VISION_MODEL").ok();
        let prev_llm = env::var("EDGEQUAKE_LLM_MODEL").ok();
        let prev_provider = env::var("EDGEQUAKE_LLM_PROVIDER").ok();

        env::remove_var("EDGEQUAKE_VISION_MODEL");
        env::remove_var("EDGEQUAKE_LLM_MODEL");
        env::remove_var("EDGEQUAKE_LLM_PROVIDER");

        f();

        // Restore
        match prev_vision {
            Some(v) => env::set_var("EDGEQUAKE_VISION_MODEL", v),
            None => env::remove_var("EDGEQUAKE_VISION_MODEL"),
        }
        match prev_llm {
            Some(v) => env::set_var("EDGEQUAKE_LLM_MODEL", v),
            None => env::remove_var("EDGEQUAKE_LLM_MODEL"),
        }
        match prev_provider {
            Some(v) => env::set_var("EDGEQUAKE_LLM_PROVIDER", v),
            None => env::remove_var("EDGEQUAKE_LLM_PROVIDER"),
        }
    }

    // -------------------------------------------------------------------------
    // default_vision_model_for_provider
    // -------------------------------------------------------------------------

    #[test]
    fn openai_provider_returns_gpt_nano_by_default() {
        with_clean_vision_env(|| {
            assert_eq!(
                default_vision_model_for_provider("openai"),
                "gpt-4.1-nano",
                "OpenAI default should be gpt-4.1-nano"
            );
        });
    }

    #[test]
    fn ollama_provider_returns_gemma_by_default() {
        with_clean_vision_env(|| {
            let model = default_vision_model_for_provider("ollama");
            // KEY INVARIANT: must never be an OpenAI model name.
            assert_ne!(
                model, "gpt-4.1-nano",
                "Ollama default must NOT be gpt-4.1-nano (an OpenAI model); got '{model}'"
            );
            assert_ne!(
                model, "gpt-4o",
                "Ollama default must NOT be gpt-4o (an OpenAI model); got '{model}'"
            );
            assert!(
                !model.is_empty(),
                "Model must not be empty for ollama provider"
            );
        });
    }

    #[test]
    fn empty_string_vision_model_env_is_ignored() {
        // WHY: Docker Compose ${VAR:-} maps unset vars to ""; treat "" as unset.
        // The model must NOT be empty AND must NOT be an OpenAI-specific model.
        with_clean_vision_env(|| {
            env::set_var("EDGEQUAKE_VISION_MODEL", "");
            let model = default_vision_model_for_provider("ollama");
            assert!(
                !model.is_empty(),
                "Empty EDGEQUAKE_VISION_MODEL env must not produce an empty model name"
            );
            assert_ne!(
                model, "gpt-4.1-nano",
                "Empty EDGEQUAKE_VISION_MODEL for Ollama provider must never yield gpt-4.1-nano"
            );
        });
    }

    #[test]
    fn env_vision_model_overrides_provider_default_for_openai() {
        with_clean_vision_env(|| {
            env::set_var("EDGEQUAKE_VISION_MODEL", "gpt-4o");
            assert_eq!(default_vision_model_for_provider("openai"), "gpt-4o");
        });
    }

    #[test]
    fn env_vision_model_overrides_provider_default_for_ollama() {
        with_clean_vision_env(|| {
            env::set_var("EDGEQUAKE_VISION_MODEL", "llava:latest");
            assert_eq!(default_vision_model_for_provider("ollama"), "llava:latest");
        });
    }

    #[test]
    fn env_llm_model_used_as_fallback_when_vision_not_set() {
        with_clean_vision_env(|| {
            env::set_var("EDGEQUAKE_LLM_MODEL", "gemma4:e4b");
            assert_eq!(
                default_vision_model_for_provider("ollama"),
                "gemma4:e4b",
                "LLM model env must be used as fallback for Ollama when VISION_MODEL is absent"
            );
        });
    }

    // -------------------------------------------------------------------------
    // KEY INVARIANT: gpt-4.1-nano must NEVER be returned for Ollama provider
    // -------------------------------------------------------------------------

    #[test]
    fn gpt_nano_is_never_returned_for_ollama_provider() {
        with_clean_vision_env(|| {
            let model = default_vision_model_for_provider("ollama");
            assert_ne!(
                model, "gpt-4.1-nano",
                "CRITICAL: gpt-4.1-nano is an OpenAI model and must never be sent to Ollama"
            );
        });
    }

    #[test]
    fn gpt_nano_env_sent_to_openai_only() {
        // An operator who sets EDGEQUAKE_VISION_MODEL="gpt-4.1-nano" intends it
        // for OpenAI.  Verify it's NOT the default for Ollama.
        with_clean_vision_env(|| {
            // Without the env var, Ollama should NOT return gpt-4.1-nano.
            assert_ne!(default_vision_model_for_provider("ollama"), "gpt-4.1-nano");
        });
    }

    // -------------------------------------------------------------------------
    // PdfUploadOptions::resolved_vision_provider
    // -------------------------------------------------------------------------

    #[test]
    fn resolved_provider_falls_back_to_ollama_when_env_unset() {
        with_clean_vision_env(|| {
            let opts = PdfUploadOptions::default();
            assert_eq!(
                opts.resolved_vision_provider(),
                "ollama",
                "Default provider should be ollama when nothing is configured"
            );
        });
    }

    #[test]
    fn explicit_vision_provider_takes_priority() {
        with_clean_vision_env(|| {
            let opts = PdfUploadOptions {
                vision_provider: Some("openai".to_string()),
                ..Default::default()
            };
            assert_eq!(opts.resolved_vision_provider(), "openai");
        });
    }

    #[test]
    fn empty_string_vision_provider_falls_back_to_env() {
        with_clean_vision_env(|| {
            env::set_var("EDGEQUAKE_LLM_PROVIDER", "lmstudio");
            let opts = PdfUploadOptions {
                vision_provider: Some("".to_string()),
                ..Default::default()
            };
            assert_eq!(
                opts.resolved_vision_provider(),
                "lmstudio",
                "Empty string vision_provider must not override env var"
            );
        });
    }

    // -------------------------------------------------------------------------
    // PdfUploadOptions::vision_model
    // -------------------------------------------------------------------------

    #[test]
    fn ollama_provider_vision_model_is_provider_compatible() {
        with_clean_vision_env(|| {
            let opts = PdfUploadOptions {
                vision_provider: Some("ollama".to_string()),
                ..Default::default()
            };
            let model = opts.vision_model();
            assert_ne!(
                model, "gpt-4.1-nano",
                "Provider=ollama must never yield model=gpt-4.1-nano (OpenAI model)"
            );
            assert!(
                !model.is_empty(),
                "Model must not be empty for ollama provider"
            );
        });
    }

    #[test]
    fn openai_provider_vision_model_defaults_to_gpt_nano() {
        with_clean_vision_env(|| {
            let opts = PdfUploadOptions {
                vision_provider: Some("openai".to_string()),
                ..Default::default()
            };
            assert_eq!(opts.vision_model(), "gpt-4.1-nano");
        });
    }

    #[test]
    fn explicit_vision_model_always_wins() {
        with_clean_vision_env(|| {
            let opts = PdfUploadOptions {
                vision_provider: Some("ollama".to_string()),
                vision_model: Some("llava:34b".to_string()),
                ..Default::default()
            };
            assert_eq!(opts.vision_model(), "llava:34b");
        });
    }

    #[test]
    fn empty_string_vision_model_falls_back_to_provider_default() {
        with_clean_vision_env(|| {
            let opts = PdfUploadOptions {
                vision_provider: Some("ollama".to_string()),
                vision_model: Some("".to_string()), // empty → treated as None
                ..Default::default()
            };
            let model = opts.vision_model();
            assert_ne!(
                model, "gpt-4.1-nano",
                "Empty vision_model + ollama provider must never yield gpt-4.1-nano"
            );
        });
    }
}
