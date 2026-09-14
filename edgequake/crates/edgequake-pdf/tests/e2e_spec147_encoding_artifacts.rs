//! SPEC-147 — encoding-artifact pages never become Pass-B figures.
//!
//! E2E of the Vision figure pass on the two live failure PDFs:
//! inventory → subset keep pages → decode (never storm pages) → empty/no
//! page-6 figs → assembled markdown has no storm `-fig-` drawings.
//!
//! Pass-A VLM (Mistral page OCR) is not invoked here; that hang is a separate
//! provider issue. This test proves the decode bomb is gone **per page**.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use edgequake_pdf::inline_images::scan_inline_image_refs;
use edgequake_pdf::{
    assemble_vision_markdown_with_figures, extract_embedded_figures,
    should_suppress_crop_for_analyze, skip_all_region_crops, CropDescriptor, PageModality,
    VisionPageSlice, XObjectInventory, ASSETS_SUBDIR,
};
use serial_test::serial;

fn spec147_pdf(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/147-fix-doc/data")
        .join(name);
    assert!(
        path.is_file(),
        "SPEC-147 fixture required at {path:?} (do not skip)"
    );
    path
}

fn fig_pngs(root: &Path) -> Vec<String> {
    let dir = root.join(ASSETS_SUBDIR);
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.contains("-fig-"))
        .collect();
    names.sort();
    names
}

fn fig_pngs_for_page(root: &Path, page: usize) -> Vec<String> {
    let needle = format!("page-{page:04}-fig-");
    fig_pngs(root)
        .into_iter()
        .filter(|n| n.contains(&needle))
        .collect()
}

#[tokio::test]
async fn e2e_nt_drm_skips_page6_table_border_xobjects() {
    let name = "NT_DRM_01942_A.pdf";
    let bytes = std::fs::read(spec147_pdf(name)).unwrap_or_else(|e| panic!("read {name}: {e}"));
    let inv = XObjectInventory::from_pdf_bytes(&bytes).expect("inventory");
    assert!(
        inv.artifact_pages().contains(&6),
        "page 6 must be artifact, pages={:?}",
        inv.pages
            .iter()
            .map(|p| (p.page_num, p.native_sizes.len(), p.artifact_kind))
            .collect::<Vec<_>>()
    );
    assert!(
        !inv.keep_pages().is_empty(),
        "non-storm pages must remain keepable"
    );
    assert!(!inv.keep_pages().contains(&6));

    let dir = tempfile::tempdir().expect("tmp");
    let started = Instant::now();
    let extract = extract_embedded_figures(&bytes, dir.path(), None)
        .await
        .expect("extract");
    assert!(
        extract.artifact_pages.contains(&6),
        "plan must list page 6 as artifact"
    );
    assert!(
        !extract.keep_pages.is_empty(),
        "NT_DRM must not whole-doc skip when other pages are clean"
    );
    assert!(!extract.keep_pages.contains(&6));
    assert!(
        !skip_all_region_crops(false, &extract),
        "partial keep must still allow captions on keep pages"
    );
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "subset extract must stay fast ({:?})",
        started.elapsed()
    );
    assert!(
        fig_pngs_for_page(dir.path(), 6).is_empty(),
        "page 6 must write no -fig- files, got {:?}",
        fig_pngs_for_page(dir.path(), 6)
    );

    let pages = vec![VisionPageSlice {
        page_num: 6,
        markdown: "Pass-A page text".into(),
    }];
    let map = extract.figure_map();
    assert!(map.get(&6).map(|v| v.is_empty()).unwrap_or(true));
    let md = assemble_vision_markdown_with_figures(
        &pages,
        true,
        true,
        Some("spec147"),
        None,
        Some(&map),
        None,
    );
    let refs = scan_inline_image_refs(&md);
    assert!(
        refs.iter().all(|r| !r
            .asset_path
            .as_deref()
            .is_some_and(|p| p.contains("page-0006-fig-"))),
        "page 6 markdown must not emit figure drawings: {md}"
    );
}

#[tokio::test]
async fn e2e_demande_essai_skips_ocr_chip_xobjects() {
    let name = "0004_Demande_d_essai_N_4440.pdf";
    let bytes = std::fs::read(spec147_pdf(name)).unwrap_or_else(|e| panic!("read {name}: {e}"));
    let inv = XObjectInventory::from_pdf_bytes(&bytes).expect("inventory");
    assert!(
        !inv.artifact_pages().is_empty(),
        "0004 must mark artifact pages"
    );

    let dir = tempfile::tempdir().expect("tmp");
    let started = Instant::now();
    let extract = extract_embedded_figures(&bytes, dir.path(), None)
        .await
        .expect("extract");
    assert!(
        extract.keep_pages.is_empty() || extract.written.is_empty(),
        "0004 OCR chips must not become figures: keep={:?} written={}",
        extract.keep_pages,
        extract.written.len()
    );
    if extract.keep_pages.is_empty() {
        assert!(skip_all_region_crops(false, &extract));
    }
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "0004 skip/subset must stay fast ({:?})",
        started.elapsed()
    );
    // Prefer zero fig files; if keep_pages non-empty, none should be OCR-chip sized leftovers.
    for fig in &extract.written {
        assert!(
            fig.width >= 24 && fig.height >= 24,
            "surviving fig must pass native gate: {}x{}",
            fig.width,
            fig.height
        );
    }
}

#[tokio::test]
#[serial]
async fn e2e_real_embedded_figure_is_not_skipped() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data/embedded_figure_sample.pdf");
    assert!(path.is_file(), "sample fixture missing: {path:?}");
    let bytes = std::fs::read(&path).expect("read sample");
    let inv = XObjectInventory::from_pdf_bytes(&bytes).expect("inventory");
    assert!(
        inv.artifact_pages().is_empty(),
        "a single real figure must not look like an encoding storm: {:?}",
        inv.pages
            .iter()
            .map(|p| (p.page_num, p.native_sizes.len(), p.artifact_kind))
            .collect::<Vec<_>>()
    );
    let dir = tempfile::tempdir().expect("tmp");
    let extract = match extract_embedded_figures(&bytes, dir.path(), None).await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("pdfium unavailable: {e}");
            return;
        }
    };
    assert!(!extract.keep_pages.is_empty());
    assert!(!skip_all_region_crops(false, &extract));
    assert!(
        !extract.written.is_empty(),
        "sample PDF must still extract a figure"
    );
}

#[test]
fn e2e_pass_b_print_suppresses_hairline_crop() {
    let crop = CropDescriptor {
        area_frac: 0.001,
        ink_frac: 0.5,
        aspect_ratio: 40.0,
        is_chart_fragment: false,
    };
    assert!(should_suppress_crop_for_analyze(PageModality::Print, &crop));
}
