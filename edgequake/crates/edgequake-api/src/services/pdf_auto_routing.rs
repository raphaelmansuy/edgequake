//! Born-digital PDF auto-routing (SPEC-038).
//!
//! Thin adapter over [`LargeDocumentProfile`] — conversion logic lives in the SSOT module.

use edgequake_pdf::{PdfConversionConfig, PdfParserBackend};

use super::large_document_profile::LargeDocumentProfile;

/// Whether auto-routing may attempt EdgeParse before Vision.
pub fn should_try_edgeparse_before_vision(
    backend: PdfParserBackend,
    backend_explicit: bool,
) -> bool {
    LargeDocumentProfile::should_try_edgeparse_before_vision(backend, backend_explicit)
}

/// Attempt fast CPU parse; returns markdown when text density is sufficient.
pub async fn try_edgeparse_fast_path(
    pdf_data: &[u8],
    page_count: usize,
    filename: &str,
) -> Option<String> {
    let config = PdfConversionConfig {
        page_count_hint: Some(page_count),
        table_method: None,
        filename: Some(filename.to_string()),
        vision: None,
    };

    let converter = edgequake_pdf::create_pdf_converter(PdfParserBackend::EdgeParse, None);
    let markdown = converter.convert(pdf_data, &config).await.ok()?;
    if LargeDocumentProfile::markdown_has_text_layer(&markdown, page_count) {
        Some(markdown)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_route_only_for_default_vision() {
        assert!(should_try_edgeparse_before_vision(
            PdfParserBackend::Vision,
            false
        ));
        assert!(!should_try_edgeparse_before_vision(
            PdfParserBackend::Vision,
            true
        ));
        assert!(!should_try_edgeparse_before_vision(
            PdfParserBackend::EdgeParse,
            false
        ));
    }
}
