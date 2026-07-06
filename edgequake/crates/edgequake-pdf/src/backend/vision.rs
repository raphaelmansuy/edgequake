use std::sync::Arc;

use async_trait::async_trait;
use edgequake_pdf2md::{convert_from_bytes, ConversionConfig, FileCheckpointStore};
use tracing::info;

use super::{PdfConversionConfig, PdfConverter};
use crate::error::PdfConversionError;

/// Vision-based PDF converter backed by `edgequake-pdf2md`.
///
/// Uses `provider_name` + `model` factory resolution inside pdf2md instead of
/// injecting `Arc<dyn LLMProvider>` — avoids dual edgequake-llm versions until
/// pdf2md@0.9.3 aligns on 0.10.0 (SPEC-043 P0).
pub struct VisionPdfConverter;

impl std::fmt::Debug for VisionPdfConverter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisionPdfConverter").finish()
    }
}

impl Default for VisionPdfConverter {
    fn default() -> Self {
        Self
    }
}

impl VisionPdfConverter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PdfConverter for VisionPdfConverter {
    async fn convert(
        &self,
        pdf_bytes: &[u8],
        config: &PdfConversionConfig,
    ) -> Result<String, PdfConversionError> {
        let vision = config
            .vision
            .as_ref()
            .ok_or(PdfConversionError::BackendNotConfigured("vision config"))?;
        let provider_name = vision
            .provider_name
            .clone()
            .ok_or(PdfConversionError::BackendNotConfigured("vision provider"))?;
        let model = vision
            .model
            .clone()
            .ok_or(PdfConversionError::BackendNotConfigured("vision model"))?;

        let mut builder = ConversionConfig::builder()
            .provider_name(provider_name)
            .model(model.clone());

        if let Some(concurrency) = vision.concurrency {
            builder = builder.concurrency(concurrency);
        }
        if let Some(dpi) = vision.dpi {
            builder = builder.dpi(dpi);
        }
        if let Some(progress_callback) = vision.progress_callback.clone() {
            builder = builder.progress_callback(progress_callback);
        }
        if let Some(checkpoint_dir) = vision.checkpoint_dir.clone() {
            builder = builder.checkpoint_store(Arc::new(FileCheckpointStore::new(&checkpoint_dir)));
        }
        if vision.no_resume {
            builder = builder.no_resume(true);
        }

        let conversion_config = builder
            .build()
            .map_err(|error| PdfConversionError::Backend(error.to_string()))?;
        let output = convert_from_bytes(pdf_bytes, &conversion_config)
            .await
            .map_err(|error| PdfConversionError::Backend(error.to_string()))?;

        if output.markdown.trim().is_empty() {
            return Err(PdfConversionError::EmptyOutput(
                "vision returned no markdown",
            ));
        }

        info!(
            pages = output.stats.total_pages,
            processed_pages = output.stats.processed_pages,
            markdown_len = output.markdown.len(),
            "Vision conversion completed"
        );

        let markdown = if output.pages.len() > 1 {
            let mut parts: Vec<String> = Vec::with_capacity(output.pages.len());
            for page in &output.pages {
                if !page.markdown.trim().is_empty() {
                    parts.push(format!(
                        "<!-- edgequake-page:{} -->\n{}",
                        page.page_num,
                        page.markdown.trim()
                    ));
                }
            }
            if parts.is_empty() {
                output.markdown
            } else {
                parts.join("\n\n")
            }
        } else {
            format!("<!-- edgequake-page:1 -->\n{}", output.markdown.trim())
        };

        Ok(markdown)
    }

    fn backend_name(&self) -> &'static str {
        "vision"
    }
}
