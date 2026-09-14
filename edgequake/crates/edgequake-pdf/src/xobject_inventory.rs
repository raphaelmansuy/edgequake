//! Cheap Image XObject inventory (lopdf, no pdfium decode).
//!
//! Used to omit encoding-artifact **pages** from pdfium extract: HTML
//! table-border storms and OCR-searchable scan chip dumps. Walks every page
//! (per-page storm abort still stops counting mid-page).

use edgeparse_core::pdf::loader::load_pdf_from_bytes;

use crate::error::PdfConversionError;
use crate::figure_keep::{
    is_decode_storm, page_artifact_kind, placement_area_pt2, PageArtifactKind,
};
use crate::page_walk::PageWalkSignals;

/// Per-page Image XObject counts from the PDF dict (no pixel decode).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageXObjectInventory {
    pub page_num: usize,
    /// Native `/Width`×`/Height` for every Image `Do` on the page.
    pub native_sizes: Vec<(u32, u32)>,
    /// Why extract must be skipped for this page (`None` = extract).
    pub artifact_kind: Option<PageArtifactKind>,
}

impl PageXObjectInventory {
    fn from_signals(
        page_num: usize,
        native_sizes: Vec<(u32, u32)>,
        bboxes: &[(f64, f64, f64, f64)],
    ) -> Self {
        let areas: Vec<f64> = bboxes.iter().copied().map(placement_area_pt2).collect();
        let artifact_kind = page_artifact_kind(&native_sizes, &areas);
        Self {
            page_num,
            native_sizes,
            artifact_kind,
        }
    }

    pub fn encoding_artifact(&self) -> bool {
        self.artifact_kind.is_some()
    }

    pub fn is_decode_storm(&self) -> bool {
        is_decode_storm(&self.native_sizes)
    }
}

/// Document-level inventory (all pages).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XObjectInventory {
    pub pages: Vec<PageXObjectInventory>,
}

impl XObjectInventory {
    pub fn from_pdf_bytes(pdf_bytes: &[u8]) -> Result<Self, PdfConversionError> {
        let raw = load_pdf_from_bytes(pdf_bytes, None)
            .map_err(|e| PdfConversionError::Backend(format!("lopdf load: {e}")))?;
        let n = raw.document.get_pages().len().max(1);
        let mut pages = Vec::with_capacity(n);
        for pn in 1..=n {
            let walked = crate::page_walk::walk_page_signals_abort_storm(&raw.document, &[pn]);
            let w = walked.into_iter().next();
            let native_sizes = w
                .as_ref()
                .map(|s| s.image_native.clone())
                .unwrap_or_default();
            let bboxes = w
                .as_ref()
                .map(|s| s.image_bboxes.clone())
                .unwrap_or_default();
            pages.push(PageXObjectInventory::from_signals(
                pn,
                native_sizes,
                &bboxes,
            ));
        }
        Ok(Self { pages })
    }

    pub fn from_walked(walked: &[PageWalkSignals]) -> Self {
        Self {
            pages: walked
                .iter()
                .map(|w| {
                    PageXObjectInventory::from_signals(
                        w.page_num,
                        w.image_native.clone(),
                        &w.image_bboxes,
                    )
                })
                .collect(),
        }
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn total_images(&self) -> usize {
        self.pages.iter().map(|p| p.native_sizes.len()).sum()
    }

    pub fn keep_native_count(&self) -> usize {
        self.pages
            .iter()
            .flat_map(|p| p.native_sizes.iter())
            .filter(|(w, h)| crate::figure_keep::keep_native_pixels(*w, *h))
            .count()
    }

    pub fn storm_pages(&self) -> Vec<usize> {
        self.pages
            .iter()
            .filter(|p| p.is_decode_storm())
            .map(|p| p.page_num)
            .collect()
    }

    pub fn artifact_pages(&self) -> Vec<usize> {
        self.pages
            .iter()
            .filter(|p| p.encoding_artifact())
            .map(|p| p.page_num)
            .collect()
    }

    /// 1-indexed pages that are safe to hand to pdfium ImageXObject extract.
    pub fn keep_pages(&self) -> Vec<usize> {
        self.pages
            .iter()
            .filter(|p| !p.encoding_artifact())
            .map(|p| p.page_num)
            .collect()
    }

    /// True when **every** inventoried page is an encoding artifact.
    pub fn all_pages_are_artifacts(&self) -> bool {
        !self.pages.is_empty()
            && self
                .pages
                .iter()
                .all(PageXObjectInventory::encoding_artifact)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn spec147(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../specs/147-fix-doc/data")
            .join(name)
    }

    #[test]
    fn nt_drm_page6_is_decode_storm_other_pages_kept() {
        let path = spec147("NT_DRM_01942_A.pdf");
        let bytes = std::fs::read(&path).expect("read NT_DRM PDF");
        let inv = XObjectInventory::from_pdf_bytes(&bytes).expect("inventory");
        assert_eq!(inv.page_count(), 9, "must walk every page");
        let p6 = inv.pages.iter().find(|p| p.page_num == 6).expect("page 6");
        assert!(
            p6.native_sizes.len() >= 50,
            "page 6 xobjects={}",
            p6.native_sizes.len()
        );
        assert_eq!(p6.artifact_kind, Some(PageArtifactKind::DecodeStorm));
        assert!(inv.artifact_pages().contains(&6));
        let keep = inv.keep_pages();
        assert!(!keep.contains(&6));
        assert!(
            !keep.is_empty(),
            "non-storm pages must remain keepable: {keep:?}"
        );
        assert!(!inv.all_pages_are_artifacts());
    }

    #[test]
    fn demande_essai_ocr_tiles_skip_extract() {
        let path = spec147("0004_Demande_d_essai_N_4440.pdf");
        let bytes = std::fs::read(&path).expect("read 0004 PDF");
        let inv = XObjectInventory::from_pdf_bytes(&bytes).expect("inventory");
        assert!(
            inv.pages
                .iter()
                .any(PageXObjectInventory::encoding_artifact),
            "0004 OCR-searchable scan must mark artifact pages, pages={:?}",
            inv.pages
                .iter()
                .map(|p| (p.page_num, p.native_sizes.len(), p.artifact_kind))
                .collect::<Vec<_>>()
        );
        // Scanned form: expect most/all pages artifact; keep may be empty.
        assert!(
            inv.all_pages_are_artifacts() || inv.keep_pages().is_empty(),
            "0004 should yield empty keep_pages (artifact scan), keep={:?}",
            inv.keep_pages()
        );
    }
}
