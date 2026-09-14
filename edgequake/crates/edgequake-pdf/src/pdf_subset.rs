//! Drop encoding-artifact pages before pdfium walks ImageXObjects.
//!
//! pdf2md has no page filter: it calls `get_processed_image` on every page.
//! The honest adapter is lopdf `delete_pages` so storm pages never reach pdfium.

use lopdf::Document;

use crate::error::PdfConversionError;

/// Keep only `keep_pages` (1-indexed, sorted unique).
///
/// Returns `(subset_pdf_bytes, remap)` where `remap[subset_page_1based - 1]`
/// is the original page number.
pub fn subset_keeping_pages(
    pdf_bytes: &[u8],
    keep_pages: &[usize],
) -> Result<(Vec<u8>, Vec<usize>), PdfConversionError> {
    let mut keep: Vec<usize> = keep_pages.to_vec();
    keep.sort_unstable();
    keep.dedup();
    if keep.is_empty() {
        return Err(PdfConversionError::Backend(
            "subset_keeping_pages: keep_pages is empty".into(),
        ));
    }

    let mut doc = Document::load_mem(pdf_bytes)
        .map_err(|e| PdfConversionError::Backend(format!("subset load: {e}")))?;
    let total = doc.get_pages().len().max(1);
    let keep_set: std::collections::HashSet<usize> = keep.iter().copied().collect();
    let to_delete: Vec<u32> = (1..=total)
        .filter(|p| !keep_set.contains(p))
        .map(|p| p as u32)
        .collect();

    if to_delete.is_empty() {
        // Identity: subset page i == original page i for i in keep.
        return Ok((pdf_bytes.to_vec(), keep));
    }

    doc.delete_pages(&to_delete);
    let _ = doc.prune_objects();

    let mut out = Vec::new();
    doc.save_to(&mut out)
        .map_err(|e| PdfConversionError::Backend(format!("subset save: {e}")))?;

    // After delete, get_pages renumbers 1..N in document order = keep order.
    Ok((out, keep))
}

/// Map a 1-indexed page number from a subset PDF back to the original page.
pub fn remap_subset_page(subset_page_1based: usize, remap: &[usize]) -> Option<usize> {
    remap.get(subset_page_1based.checked_sub(1)?).copied()
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
    fn remap_is_one_based_index_into_keep() {
        let remap = vec![1, 2, 3, 4, 5, 7, 8, 9];
        assert_eq!(remap_subset_page(1, &remap), Some(1));
        assert_eq!(remap_subset_page(6, &remap), Some(7));
        assert_eq!(remap_subset_page(8, &remap), Some(9));
        assert_eq!(remap_subset_page(9, &remap), None);
        assert_eq!(remap_subset_page(0, &remap), None);
    }

    #[test]
    fn nt_drm_subset_drops_page6_and_remaps() {
        let bytes = std::fs::read(spec147("NT_DRM_01942_A.pdf")).expect("read NT_DRM");
        let keep: Vec<usize> = (1..=9).filter(|&p| p != 6).collect();
        let (subset, remap) = subset_keeping_pages(&bytes, &keep).expect("subset");
        assert_eq!(remap, keep);
        let doc = Document::load_mem(&subset).expect("load subset");
        assert_eq!(doc.get_pages().len(), 8);
        // Identity path when nothing deleted.
        let (same, id_remap) = subset_keeping_pages(&bytes, &(1..=9).collect::<Vec<_>>()).unwrap();
        assert_eq!(same.len(), bytes.len());
        assert_eq!(id_remap, (1..=9).collect::<Vec<_>>());
    }
}
