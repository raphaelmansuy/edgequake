//! SPEC-049 — e2e visual region extract → assemble → analyze edge cases.

mod common;

use common::spec026_multimodal::{
    allow_tiny_images_in_tests, mock_chart_vlm_responses, restore_vlm_image_limits,
    vision_page_markdown, vision_table_page_markdown, write_figure_png_asset, write_page_png_asset,
    write_table_png_asset, TINY_PNG,
};
use edgequake_api::services::run_multimodal_analyze_stage_outcome;
use edgequake_llm::MockProvider;
use edgequake_pdf::scan_inline_image_refs;
use edgequake_pdf2md::{
    extract_visual_regions_from_bytes, extract_visual_regions_from_path, RegionKind, RegionSource,
};
use serial_test::serial;
use std::path::PathBuf;
use std::sync::Arc;

fn vector_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../edgequake-pdf2md/test_cases/vector_figure_table_sample.pdf")
}

fn embedded_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../edgequake-pdf2md/test_cases/embedded_figure_sample.pdf")
}

/// Real multi-page arXiv papers under `specs/048-improve-ux/e2e/`.
fn e2e_pdf_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/048-improve-ux/e2e")
        .join(name)
}

fn ideas_fixture() -> PathBuf {
    e2e_pdf_fixture("ideas_2607.08758v1.pdf")
}

fn hierar_fixture() -> PathBuf {
    e2e_pdf_fixture("hierar_2607.02980v1.pdf")
}

fn lightrad_fixture() -> PathBuf {
    e2e_pdf_fixture("lighrad_2410.05779v3.pdf")
}

fn assert_corpus_regions(
    path: &std::path::Path,
    min_figs: usize,
    required_figure_nums: &[u32],
    max_page: usize,
) {
    let regions = match extract_visual_regions_from_path(path, None) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("skip pdfium: {e}");
            return;
        }
    };
    let fig_count = regions
        .iter()
        .filter(|r| r.kind == RegionKind::Figure)
        .count();
    assert!(
        fig_count >= min_figs,
        "{}: expected ≥{min_figs} figures, got {fig_count}",
        path.display()
    );
    for n in required_figure_nums {
        let label = format!("Figure {n}");
        assert!(
            regions.iter().any(|r| r.label == label),
            "{}: missing {label}",
            path.display()
        );
    }
    for r in &regions {
        let area = (r.width as u64) * (r.height as u64);
        assert!(
            area < 1_200_000,
            "G3 near-full: {}x{} p{} {}",
            r.width,
            r.height,
            r.page_num,
            r.label
        );
        assert!(r.page_num >= 1 && r.page_num <= max_page);
        assert!(matches!(r.source, RegionSource::ObjectCluster));
    }
    assert_eq!(
        regions
            .iter()
            .filter(|r| r.kind == RegionKind::Table)
            .count(),
        0,
        "{}: text-native tables must not invent crops",
        path.display()
    );
}

async fn assert_corpus_writes_figs(path: &std::path::Path, min_figs: usize) {
    use edgequake_pdf::write_caption_region_assets;
    use std::collections::HashMap;

    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("skip read: {e}");
            return;
        }
    };
    let tmp = tempfile::tempdir().unwrap();
    let (figs, tables) =
        match write_caption_region_assets(&bytes, tmp.path(), &HashMap::new(), None).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("skip write: {e}");
                return;
            }
        };
    assert!(
        figs.len() >= min_figs,
        "{}: expected ≥{min_figs} written figs, got {}",
        path.display(),
        figs.len()
    );
    assert!(
        tables.is_empty(),
        "{}: text-native tables must not invent -table- crops, got {}",
        path.display(),
        tables.len()
    );
    for f in &figs {
        let full = tmp.path().join(&f.rel_path);
        assert!(full.is_file(), "missing {}", f.rel_path);
        assert!(f.rel_path.contains("-fig-"));
        let area = (f.width as u64) * (f.height as u64);
        assert!(area < 1_200_000, "near-full written {}", f.rel_path);
    }
}

#[test]
fn e2_e3_vector_fixture_regions_not_full_page() {
    let path = vector_fixture();
    if !path.exists() {
        eprintln!("skip: missing {path:?}");
        return;
    }
    let regions = match extract_visual_regions_from_path(&path, None) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("skip pdfium: {e}");
            return;
        }
    };
    assert!(
        !regions.is_empty(),
        "expected visual regions from vector sample"
    );
    for r in &regions {
        let area = (r.width as u64) * (r.height as u64);
        assert!(
            area < 1_200_000,
            "E3/G3 near-full rejected: {}x{} {:?}",
            r.width,
            r.height,
            r.source
        );
        assert!(matches!(
            r.source,
            RegionSource::ObjectCluster | RegionSource::StructTree
        ));
    }
    assert!(
        regions
            .iter()
            .any(|r| r.kind == RegionKind::Figure || r.kind == RegionKind::Table),
        "expected figure or table kind"
    );
}

#[test]
fn e1_embedded_fixture_yields_figure() {
    let path = embedded_fixture();
    if !path.exists() {
        eprintln!("skip: missing {path:?}");
        return;
    }
    let regions = match extract_visual_regions_from_path(&path, None) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("skip pdfium: {e}");
            return;
        }
    };
    assert!(
        regions.iter().any(|r| r.kind == RegionKind::Figure),
        "E1 ImageXObject → figure"
    );
}

#[test]
fn e11_invalid_pdf_errors() {
    assert!(extract_visual_regions_from_bytes(b"%PDF-not-really", None).is_err());
}

/// E14 — Ideas arXiv PDF: untagged 22-page paper with Figure 1–10 vector diagrams.
#[test]
fn e14_ideas_pdf_figures_stable_not_full_page() {
    let path = ideas_fixture();
    if !path.exists() {
        eprintln!("skip: missing Ideas fixture {path:?}");
        return;
    }
    assert_corpus_regions(&path, 10, &(1..=10).collect::<Vec<_>>(), 22);
}

/// E14b — write fig assets for Ideas PDF; files exist and stay under G3.
#[tokio::test]
async fn e14b_ideas_pdf_writes_figure_assets() {
    let path = ideas_fixture();
    if !path.exists() {
        eprintln!("skip: missing Ideas fixture {path:?}");
        return;
    }
    assert_corpus_writes_figs(&path, 10).await;
}

/// E15 — Hierarchical Sparse Attention (arXiv 2607.02980).
#[test]
fn e15_hierar_pdf_figures_stable_not_full_page() {
    let path = hierar_fixture();
    if !path.exists() {
        eprintln!("skip: missing hierar fixture {path:?}");
        return;
    }
    assert_corpus_regions(&path, 7, &(1..=7).collect::<Vec<_>>(), 27);
}

#[tokio::test]
async fn e15b_hierar_pdf_writes_figure_assets() {
    let path = hierar_fixture();
    if !path.exists() {
        eprintln!("skip: missing hierar fixture {path:?}");
        return;
    }
    assert_corpus_writes_figs(&path, 7).await;
}

/// E16 — LightRAG (arXiv 2410.05779); Figure 2 may be absent from object clusters.
#[test]
fn e16_lightrad_pdf_figures_stable_not_full_page() {
    let path = lightrad_fixture();
    if !path.exists() {
        eprintln!("skip: missing LightRAG fixture {path:?}");
        return;
    }
    assert_corpus_regions(&path, 5, &[1, 3, 4, 5, 6, 7], 16);
}

#[tokio::test]
async fn e16b_lightrad_pdf_writes_figure_assets() {
    let path = lightrad_fixture();
    if !path.exists() {
        eprintln!("skip: missing LightRAG fixture {path:?}");
        return;
    }
    assert_corpus_writes_figs(&path, 5).await;
}

#[tokio::test]
#[serial]
async fn e4_table_drawing_analyze_no_full_page() {
    allow_tiny_images_in_tests();
    std::env::set_var("EDGEQUAKE_VISION_PROVIDER", "__test_no_vision__");

    let assets_root = tempfile::tempdir().unwrap();
    write_page_png_asset(assets_root.path(), 6);
    write_table_png_asset(assets_root.path(), 6);

    let raw = vision_table_page_markdown("spec049-e4", 6, "## Table 1: Rates\n\nBody.");
    let refs = scan_inline_image_refs(&raw);
    assert_eq!(refs.len(), 1);
    assert!(refs[0]
        .asset_path
        .as_deref()
        .is_some_and(|p| p.contains("-table-")));

    let mock = Arc::new(MockProvider::new());
    mock_chart_vlm_responses(mock.as_ref()).await;
    let outcome = run_multimodal_analyze_stage_outcome(
        raw,
        Some("ite"),
        "spec049-table.pdf",
        None,
        uuid::Uuid::nil(),
        mock,
        Some(assets_root.path()),
        Some("spec049-e4"),
        None,
    )
    .await;
    assert!(outcome.summary.success >= 1);
    assert!(!outcome.markdown.contains("](assets/page-0006.png)"));

    restore_vlm_image_limits();
    std::env::remove_var("EDGEQUAKE_VISION_PROVIDER");
}

#[tokio::test]
#[serial]
async fn e10_fig_page_drawing_not_chart_override() {
    allow_tiny_images_in_tests();
    std::env::set_var("EDGEQUAKE_VISION_PROVIDER", "__test_no_vision__");

    let assets_root = tempfile::tempdir().unwrap();
    write_page_png_asset(assets_root.path(), 1);
    write_figure_png_asset(assets_root.path(), 1);
    // Stale chart must not become the drawing target when fig exists in markdown.
    std::fs::write(
        assets_root.path().join("assets/page-0001-chart.png"),
        TINY_PNG,
    )
    .unwrap();

    let raw = vision_page_markdown("spec049-e10", &[(1, "Figure overview.")]);
    assert!(raw.contains("-fig-01.png"));
    assert!(!raw.contains("-chart.png"));

    let mock = Arc::new(MockProvider::new());
    mock_chart_vlm_responses(mock.as_ref()).await;
    let outcome = run_multimodal_analyze_stage_outcome(
        raw,
        Some("ite"),
        "spec049-fig.pdf",
        None,
        uuid::Uuid::nil(),
        mock,
        Some(assets_root.path()),
        Some("spec049-e10"),
        None,
    )
    .await;
    assert!(outcome.markdown.contains("page-0001-fig-01"));
    assert!(!outcome.markdown.contains("](assets/page-0001.png)"));

    restore_vlm_image_limits();
    std::env::remove_var("EDGEQUAKE_VISION_PROVIDER");
}

/// E20 — ink residual propose is keyword-free; W1-crop-expand allows fig pages.
#[test]
fn e20_ink_residual_propose_allows_fig_pages() {
    use edgequake_pdf::{chart_residual_candidate_pages, WrittenFigureAsset, WrittenTableAsset};
    use std::collections::HashMap;

    let mut figs = HashMap::new();
    figs.insert(
        1,
        vec![WrittenFigureAsset {
            page_num: 1,
            index: 1,
            rel_path: "assets/page-0001-fig-01.png".into(),
            width: 40,
            height: 30,
            bbox: None,
        }],
    );
    let tables: HashMap<usize, Vec<WrittenTableAsset>> = HashMap::new();
    assert_eq!(
        chart_residual_candidate_pages(&[1, 5], &figs, &tables),
        vec![1, 5]
    );
}
