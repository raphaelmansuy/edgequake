//! SPEC-134 P0 contract: the belief-store admission gate is wired on the
//! production path.
//!
//! Three leaks were measured on the live assessment document (2026-08-20):
//! 32 `ASSETS/*.PNG` entities from scan-tiling fragment links, a
//! `TRACTION_TEST_RESULTS` entity from a `grounding:low score=0.00` page, and
//! gear-sketch entities from Pass-B narration of a tiling fragment. These
//! tests pin the fixes so a future refactor cannot silently reopen the gate.

/// PDF conversion processor (modality → conversion config wiring).
fn prod_pdf_src() -> &'static str {
    include_str!("../src/processor/pdf_processing.rs")
}

/// Vision backend (figure_map population — the fragment choke point).
fn vision_backend_src() -> &'static str {
    include_str!("../../edgequake-pdf/src/backend/vision.rs")
}

/// Insert-task prepare stage (chunking/extraction input construction).
fn prepare_src() -> &'static str {
    include_str!("../src/processor/text_insert/prepare.rs")
}

#[test]
fn modality_fed_to_conversion_config_on_production_path() {
    let src = prod_pdf_src();
    assert!(
        src.contains("cfg.page_modality = Some(page_modality)"),
        "the resolved page modality must reach PageDrawingAssetsConfig so \
         scan-tiling suppression applies during conversion"
    );
}

#[test]
fn tiling_suppression_runs_at_figure_map_source() {
    let src = vision_backend_src();
    assert!(
        src.contains("extract_embedded_figures"),
        "vision backend must use the figure-extract facade (inventory skip + prune)"
    );
    assert!(
        src.contains("skip_all_region_crops") || src.contains("omit_page"),
        "caption and chart crops must share page-level omit (Pass-A raster is the unit)"
    );
    assert!(
        src.contains("keep_pages") || src.contains("artifact_pages"),
        "figure plan must expose keep_pages / artifact_pages"
    );
    assert!(
        src.contains("prune_artifact_figures"),
        "figure_map must be pruned after writers so fragments cannot re-enter"
    );
    assert!(
        src.contains("page_modality") && src.contains("is_manuscript_like()"),
        "LAW-134-20 page-as-unit must still gate manuscript fragment inject"
    );
}

#[test]
fn quarantine_lane_filters_index_input_not_display() {
    let src = prepare_src();
    assert!(
        src.contains("strip_low_grounding_sections(&text_content)"),
        "chunking/extraction input must pass through the quarantine lane"
    );
    // Display != Index (LAW-134-4): the quarantined text feeds only the
    // processed_text (index) construction; text_content (display SSOT) must
    // remain the unfiltered resolved content.
    let quarantine_pos = src.find("strip_low_grounding_sections").unwrap();
    let processed_pos = src.find("let processed_text = {").unwrap();
    assert!(
        quarantine_pos > processed_pos,
        "quarantine filter must live inside the processed_text (index) block"
    );
}

#[test]
fn marker_format_has_single_owner() {
    // DRY: the grounding:low marker is written by the verify pass and read by
    // the quarantine lane — both must live in manuscript_verify.rs so the
    // format can never drift.
    let src = include_str!("../src/services/manuscript_verify.rs");
    assert!(src.contains("GROUNDING_LOW_MARKER_PREFIX"));
    assert!(src.contains("GROUNDING_QUARANTINED_MARKER"));
    assert!(src.contains("pub fn strip_low_grounding_sections"));
}
