//! Standalone image upload analyze path (DRY with PDF inline analyzer).

use edgequake_llm::traits::LLMProvider;

use super::super::vision_content::image_analysis_to_markdown;
use super::super::vlm_limits::{probe_image_dimensions, validate_image_for_vlm};
use super::analyzer::analyze_image_bytes;
use super::context::SurroundingContext;
use super::item_record::MultimodalItemRecord;
use super::prompt_context::PromptContext;
use super::scan::standalone_image_manifest;

/// Result of standalone image VLM describe-to-text admission.
#[derive(Debug, Clone)]
pub struct StandaloneImageOutcome {
    pub markdown: String,
    pub manifest: super::manifest::MultimodalManifest,
    pub ingest_mode: &'static str,
}

/// Analyze uploaded image bytes for standalone admission (same core as inline PDF path).
pub async fn analyze_standalone_image(
    content: &[u8],
    mime: &str,
    filename: &str,
    llm: &dyn LLMProvider,
) -> StandaloneImageOutcome {
    let item_id = format!("upload_{}", filename.replace(['/', '\\'], "_"));

    let (width, height) = probe_image_dimensions(content, mime).unwrap_or((0, 0));
    if let Err(reason) = validate_image_for_vlm(content, width, height) {
        let record = MultimodalItemRecord::skipped(&item_id, "drawing", reason);
        return StandaloneImageOutcome {
            markdown: format!(
                "# Image: {filename}\n\n*VLM skipped: {}*",
                record.message.as_deref().unwrap_or("")
            ),
            manifest: standalone_image_manifest(record),
            ingest_mode: "vlm_skipped",
        };
    }

    let ctx = PromptContext::from_parts(None, None, &SurroundingContext::default());
    match analyze_image_bytes(&item_id, content, mime, llm, &ctx, None).await {
        Ok((record, _replacement)) => {
            let analysis = super::super::vision_content::ImageAnalysisResult {
                name: record.name.clone().unwrap_or_default(),
                image_type: record.item_type.clone().unwrap_or_default(),
                description: record.description.clone().unwrap_or_default(),
            };
            StandaloneImageOutcome {
                markdown: image_analysis_to_markdown(&analysis),
                manifest: standalone_image_manifest(record),
                ingest_mode: "vlm_describe",
            }
        }
        Err(record) => StandaloneImageOutcome {
            markdown: format!(
                "# Image Document: {filename}\n\n*Automatic text extraction failed: {}*",
                record.message.as_deref().unwrap_or("unknown error")
            ),
            manifest: standalone_image_manifest(record),
            ingest_mode: "vlm_describe",
        },
    }
}
