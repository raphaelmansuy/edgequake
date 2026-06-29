use async_trait::async_trait;
use edgeparse_core::{
    api::config::{ProcessingConfig, TableMethod},
    convert_bytes,
    models::document::PdfDocument,
    output::markdown,
};
use tracing::info;

use super::{PdfConversionConfig, PdfConverter};
use crate::error::PdfConversionError;

/// The page marker format injected between pages in the markdown output.
/// Parsed by `PageAwareChunking` to enforce page-boundary chunk splits.
const PAGE_MARKER_PREFIX: &str = "<!-- edgequake-page:";
const PAGE_MARKER_SUFFIX: &str = " -->";

fn page_marker(n: u32) -> String {
    format!("{}{}{}", PAGE_MARKER_PREFIX, n, PAGE_MARKER_SUFFIX)
}

/// Fast CPU-only PDF converter powered by EdgeParse.
///
/// # Page marker injection (SPEC-032 W-09)
///
/// EdgeParse produces a `PdfDocument` where each `ContentElement` in
/// `doc.kids` carries a `page_number()` from its bounding box. We group kids
/// by page, build one mini-`PdfDocument` per page (cloning metadata), call
/// `to_markdown` on each, and join the results with
/// `<!-- edgequake-page:N -->` markers.
///
/// This ensures the `PageAwareChunking` strategy can split at hard page
/// boundaries so **no chunk ever spans two pages**.
#[derive(Debug, Default)]
pub struct EdgeParsePdfConverter;

#[async_trait]
impl PdfConverter for EdgeParsePdfConverter {
    async fn convert(
        &self,
        pdf_bytes: &[u8],
        config: &PdfConversionConfig,
    ) -> Result<String, PdfConversionError> {
        let pdf_bytes = pdf_bytes.to_vec();
        let filename = config
            .filename
            .clone()
            .unwrap_or_else(|| "document.pdf".to_string());
        let table_method = config.table_method.clone();

        tokio::task::spawn_blocking(move || {
            let processing = ProcessingConfig {
                table_method: match table_method.as_deref() {
                    Some("cluster") => TableMethod::Cluster,
                    _ => TableMethod::Default,
                },
                ..Default::default()
            };

            let document = convert_bytes(&pdf_bytes, &filename, &processing)
                .map_err(|error| PdfConversionError::Backend(error.to_string()))?;

            let total_pages = document.number_of_pages;

            info!(
                pages = total_pages,
                "EdgeParse conversion completed, injecting page markers"
            );

            let markdown_with_markers =
                build_page_marked_markdown(&document).map_err(PdfConversionError::Backend)?;

            if markdown_with_markers.trim().is_empty() {
                return Err(PdfConversionError::EmptyOutput(
                    "edgeparse returned no markdown",
                ));
            }

            info!(
                pages = total_pages,
                markdown_len = markdown_with_markers.len(),
                "EdgeParse: page-marked markdown built"
            );

            Ok(markdown_with_markers)
        })
        .await
        .map_err(|error| PdfConversionError::Internal(error.to_string()))?
    }

    fn backend_name(&self) -> &'static str {
        "edgeparse"
    }
}

/// Build markdown with `<!-- edgequake-page:N -->` markers by grouping
/// `doc.kids` (flattened content elements) by their `page_number()`.
///
/// # Algorithm
///
/// 1. Walk `doc.kids` in reading order.
/// 2. Group consecutive elements that share the same `page_number()`.
/// 3. For each group, build a mini `PdfDocument` (single page) and call
///    `edgeparse_core::output::markdown::to_markdown`.
/// 4. Prepend each page's markdown with its page marker.
///
/// # Fallback
///
/// If any element has `page_number() == None`, it is attributed to the
/// current page (reading-order heuristic). Single-page documents get a
/// single `<!-- edgequake-page:1 -->` marker at the top.
fn build_page_marked_markdown(document: &PdfDocument) -> Result<String, String> {
    if document.kids.is_empty() {
        return Ok(String::new());
    }

    // Group content elements by page number (preserving reading order)
    let mut page_groups: Vec<(u32, Vec<_>)> = Vec::new();
    let mut current_page: u32 = 1;

    for kid in &document.kids {
        let kid_page = kid.page_number().unwrap_or(current_page);
        // Only advance the page counter forward (defensive: some elements
        // can have None page_number and should stay on the current page)
        if kid_page >= current_page {
            current_page = kid_page;
        }
        if let Some(last) = page_groups.last_mut() {
            if last.0 == current_page {
                last.1.push(kid.clone());
                continue;
            }
        }
        page_groups.push((current_page, vec![kid.clone()]));
    }

    if page_groups.is_empty() {
        return Ok(String::new());
    }

    // Single-page document: no grouping needed, just prepend page 1 marker
    if page_groups.len() == 1 && document.number_of_pages <= 1 {
        let page_md = markdown::to_markdown(document).map_err(|e| e.to_string())?;
        if page_md.trim().is_empty() {
            return Ok(String::new());
        }
        return Ok(format!("{}\n{}", page_marker(1), page_md));
    }

    let mut parts: Vec<String> = Vec::with_capacity(page_groups.len());

    for (page_num, kids) in page_groups {
        if kids.is_empty() {
            continue;
        }
        // Build a mini single-page PdfDocument for this page's kids
        let page_doc = PdfDocument {
            file_name: document.file_name.clone(),
            source_path: document.source_path.clone(),
            number_of_pages: 1,
            author: document.author.clone(),
            title: document.title.clone(),
            creation_date: document.creation_date.clone(),
            modification_date: document.modification_date.clone(),
            producer: document.producer.clone(),
            creator: document.creator.clone(),
            subject: document.subject.clone(),
            keywords: document.keywords.clone(),
            kids,
        };

        let page_md = markdown::to_markdown(&page_doc).map_err(|e| e.to_string())?;
        if !page_md.trim().is_empty() {
            parts.push(format!("{}\n{}", page_marker(page_num), page_md));
        }
    }

    Ok(parts.join("\n\n"))
}
