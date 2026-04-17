//! PDF upload handler module.
//!
//! Split from monolithic `pdf_upload.rs` into focused sub-modules:
//! - `types`: DTOs and request/response structs
//! - `upload`: Main upload handler
//! - `status`: Status, listing, deletion, progress handlers
//! - `content`: Download and content retrieval handlers
//! - `helpers`: Internal utilities (storage access, task creation, page counting)
//! - `operations`: Retry and cancel handlers

pub mod content;
mod helpers;
pub mod operations;
pub mod status;
pub mod types;
pub mod upload;

// Re-export all public items for flat access via `handlers::*`
pub use content::*;
pub use operations::*;
pub use status::*;
pub use types::*;
pub use upload::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_processing_time() {
        // Small PDF, few pages
        let data = vec![0u8; 100_000]; // 100KB
        let time = helpers::estimate_processing_time(&data, Some(5));
        assert!((15..=30).contains(&time)); // 5 pages * 3s + 0.1MB * 0.5

        // Large PDF, many pages
        let data = vec![0u8; 10_000_000]; // 10MB
        let time = helpers::estimate_processing_time(&data, Some(50));
        assert!((150..=200).contains(&time)); // 50 pages * 3s + 10MB * 0.5
    }

    #[test]
    fn test_pdf_upload_options_vision_model() {
        let mut opts = PdfUploadOptions {
            vision_provider: Some("openai".to_string()),
            ..Default::default()
        };
        // OODA-04: Updated from gpt-4o-mini to gpt-4.1-nano per mission directive
        assert_eq!(opts.vision_model(), "gpt-4.1-nano");

        opts.vision_provider = Some("ollama".to_string());
        assert_eq!(opts.vision_model(), "gemma4:latest");

        opts.vision_model = Some("custom-model".to_string());
        assert_eq!(opts.vision_model(), "custom-model");

        // Test default (None provider = env var EDGEQUAKE_LLM_PROVIDER or "ollama")
        // WHY: Should NOT default to "openai" — that requires an API key and breaks
        // Ollama-only deployments. Default to "ollama" (safe, no API key needed).
        let default_opts = PdfUploadOptions::default();
        // In test environments EDGEQUAKE_LLM_PROVIDER is unset, so defaults to "ollama"
        // which gives a safe local default model for vision extraction.
        let resolved_provider = default_opts.resolved_vision_provider();
        assert_ne!(
            resolved_provider, "openai",
            "Must not hardcode openai as default vision provider"
        );
    }

    #[test]
    fn test_pdf_upload_options_backend_resolution_defaults_to_vision() {
        let opts = PdfUploadOptions::default();
        assert_eq!(opts.resolved_backend(None).as_str(), "vision");
    }

    /// OODA-17: Test PdfOperationResponse serialization
    #[test]
    fn test_pdf_operation_response_serialization() {
        // With task_id
        let response = PdfOperationResponse {
            success: true,
            pdf_id: "abc123".to_string(),
            message: "Retry initiated".to_string(),
            task_id: Some("task-456".to_string()),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"pdf_id\":\"abc123\""));
        assert!(json.contains("\"task_id\":\"task-456\""));

        // Without task_id (skip_serializing_if)
        let response = PdfOperationResponse {
            success: true,
            pdf_id: "abc123".to_string(),
            message: "Cancelled".to_string(),
            task_id: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("task_id"));
    }
}
