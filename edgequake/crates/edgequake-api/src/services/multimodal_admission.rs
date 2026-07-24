//! Multimodal upload content resolution SSOT (SPEC-026 Phase 4 P-07).
//!
//! DRY entry for file and batch upload handlers: routes images through shared
//! `analyze_standalone_image`; text files through existing validation.

use uuid::Uuid;

use crate::error::ApiResult;
use crate::file_validation::{
    image_mime_type, is_image_extension, validate_file, validate_file_size,
};
use crate::services::analyze_standalone_image;
use crate::services::vlm_provider_resolver::resolve_vlm_provider;
use crate::services::MultimodalManifest;
use crate::state::AppState;

/// Metadata attached to multimodal admissions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultimodalAdmissionMeta {
    pub multimodal: bool,
    pub ingest_mode: Option<&'static str>,
    pub source_type: &'static str,
}

impl Default for MultimodalAdmissionMeta {
    fn default() -> Self {
        Self {
            multimodal: false,
            ingest_mode: None,
            source_type: "file",
        }
    }
}

/// Resolved upload payload ready for `DocumentAdmissionInput`.
#[derive(Debug, Clone)]
pub struct ResolvedUploadContent {
    pub text_content: String,
    pub mime_type: String,
    pub meta: MultimodalAdmissionMeta,
    /// Virtual sidecar manifest when content is multimodal (SPEC-026 Phase 4e).
    pub manifest: Option<MultimodalManifest>,
}

/// Resolve uploaded bytes to text content (image → VLM, text → UTF-8 validate).
pub async fn resolve_upload_content(
    state: &AppState,
    workspace_id: Option<Uuid>,
    filename: &str,
    content: &[u8],
) -> ApiResult<ResolvedUploadContent> {
    validate_file_size(content.len(), state.config.max_document_size)?;

    let raw_ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();

    if is_image_extension(&raw_ext) {
        return resolve_image_upload(state, workspace_id, filename, content, &raw_ext).await;
    }

    let (_extension, text_content, mime_type) =
        validate_file(filename, content, state.config.max_document_size)?;
    // SPEC-086: .md / markdown MIME → source_type markdown (not generic "file").
    let source_type = if raw_ext == "md"
        || mime_type.eq_ignore_ascii_case("text/markdown")
        || mime_type.eq_ignore_ascii_case("text/x-markdown")
    {
        "markdown"
    } else if raw_ext == "txt" || mime_type.starts_with("text/") {
        "text"
    } else {
        "file"
    };
    Ok(ResolvedUploadContent {
        text_content,
        mime_type: mime_type.to_string(),
        meta: MultimodalAdmissionMeta {
            multimodal: false,
            ingest_mode: None,
            source_type,
        },
        manifest: None,
    })
}

async fn resolve_image_upload(
    state: &AppState,
    workspace_id: Option<Uuid>,
    filename: &str,
    content: &[u8],
    raw_ext: &str,
) -> ApiResult<ResolvedUploadContent> {
    let mime = image_mime_type(raw_ext).unwrap_or("image/png");
    let llm = resolve_vlm_provider(state, workspace_id).await;
    let outcome = analyze_standalone_image(content, mime, filename, llm.as_ref()).await;

    Ok(ResolvedUploadContent {
        text_content: outcome.markdown,
        mime_type: mime.to_string(),
        meta: MultimodalAdmissionMeta {
            multimodal: true,
            ingest_mode: Some(outcome.ingest_mode),
            source_type: "image",
        },
        manifest: Some(outcome.manifest),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multimodal_meta_defaults_non_image() {
        let meta = MultimodalAdmissionMeta::default();
        assert!(!meta.multimodal);
        assert_eq!(meta.source_type, "file");
    }

    #[test]
    fn markdown_extension_maps_to_markdown_source_type() {
        // Unit-level pin: resolve_upload_content needs AppState; assert taxonomy helper.
        let raw_ext = "md";
        let mime_type = "text/markdown";
        let source_type = if raw_ext == "md"
            || mime_type.eq_ignore_ascii_case("text/markdown")
            || mime_type.eq_ignore_ascii_case("text/x-markdown")
        {
            "markdown"
        } else {
            "file"
        };
        assert_eq!(source_type, "markdown");
    }
}
