pub mod backend;
pub mod chart_crop;
pub mod document_language;
pub mod drawing_tags;
pub mod embedded_images;
pub mod error;
pub mod fallback;
pub mod figure_filter;
pub mod image_guard;
pub mod inline_images;
pub mod manuscript_profile;
pub mod page_assets;
pub mod page_convert_plan;
pub mod page_count;
pub mod page_layout;
pub mod page_marker;
pub mod page_modality;
pub mod page_selection;
pub mod page_signals;
pub mod page_walk;
pub mod pdfium_ready;
pub mod reasoning_effort_inject;
pub mod region_assets;
pub mod vision_extract;
pub mod vision_markdown;
pub mod vision_prompts;

pub use backend::{
    create_pdf_converter, resolve_pdf_parser_choice, PageDrawingAssetsConfig, PdfConversionConfig,
    PdfConverter, PdfParserBackend, PdfParserResolutionSource, ResolvedPdfParser,
    VisionConversionConfig, VisionStatusHook,
};
pub use chart_crop::{
    chart_residual_alongside_fig_pages, chart_residual_candidate_pages, crop_png_to_ink_bbox,
    encode_png, filter_chart_pages_by_page_png_ink, ink_content_bbox, ink_fraction,
    ink_fraction_from_bytes, maybe_chart_specialize_bytes, page_markdown_suggests_chart,
    page_png_has_ink_residual, promote_fig_as_chart_when_ink_empty, text_suggests_chart,
    write_chart_crop_assets, CropCoverageReport, CHART_CROP_RENDER,
};
pub use document_language::detect_document_language;
pub use drawing_tags::{
    asset_id_from_rel_path, asset_url_matches_rel, bind_figure_images_to_page_asset,
    caption_with_page_context, count_markdown_images_for_asset, dedupe_markdown_asset_images,
    finalize_page_asset_images, format_drawing_block, format_drawing_tag,
    format_inline_asset_image, inject_figure_local_images, insert_drawing_tag_after_first_image,
    is_drawing_eligible_asset_rel_path, is_full_page_asset_rel_path,
    markdown_has_durable_asset_image, page_asset_rel_path, page_chart_crop_rel_path,
    page_chart_drawing_item_id, page_drawing_item_id, page_figure_asset_rel_path,
    page_figure_drawing_item_id, page_num_from_asset_rel_path, page_table_asset_rel_path,
    ASSETS_SUBDIR, EMPTY_VISION_PAGE_PLACEHOLDER,
};
pub use embedded_images::{
    figures_by_page, is_scan_tiling_page, write_embedded_figure_assets, WrittenFigureAsset,
};
pub use error::PdfConversionError;
pub use fallback::{
    build_edgeparse_fallback_message, should_fallback_to_edgeparse, VisionFailureKind,
};
pub use figure_filter::{
    apply_filter_result_or_keep, apply_filter_to_figure_map, collect_filter_candidates,
    crop_descriptor_from_asset, delete_discarded_pngs, discarded_rel_paths,
    discarded_rel_paths_from_manifest, figure_filter_env_enabled, inject_kept_descriptions,
    load_manifest, prune_chart_crop_paths, prune_figure_map, prune_figure_map_using_manifest,
    should_suppress_crop_manuscript, strip_discarded_asset_lines, write_manifest, CropDescriptor,
    CropGeometryCache, FigureCandidate, FigureFilter, FigureFilterResult, FigureKind,
    FIGURE_FILTER_MANIFEST,
};
pub use inline_images::{
    scan_inline_image_refs, InlineImageAnalysis, InlineImageAnalyzer, NoopInlineImageAnalyzer,
};
pub use manuscript_profile::{
    ManuscriptProfile, DEFAULT_MANUSCRIPT_DPI, DEFAULT_MANUSCRIPT_MAX_PIXELS,
};
pub use page_assets::{write_page_png_assets, PageAssetRenderConfig};
pub use page_convert_plan::{classifications_from_signals, PageConvertPlan};
pub use page_count::{count_pdf_pages, resolve_pdf_page_count};
pub use page_layout::{
    load_page_layout_sidecar, sidecar_exists, write_sidecar_from_assets, BBoxPdf,
    PageLayoutPageSidecar, PageLayoutRegionSidecar, PageLayoutSidecar, PAGE_LAYOUT_SIDECAR,
};
pub use page_marker::{PageMarkerWriter, PAGE_MARKER_PREFIX, PAGE_MARKER_SUFFIX};
pub use page_modality::{
    classify_document_majority, classify_page_heuristic, PageClassResult, PageClassification,
    PageModality,
};
pub use page_selection::parse_page_selection;
pub use page_signals::{
    analyze_modality_blocking, classify_document_from_bytes, classify_document_from_signals,
    classify_pages_from_bytes, compute_page_signals, compute_page_signals_blocking,
    orientation_mixed, ModalityAnalysis, PageSignals,
};
pub use page_walk::{walk_page_signals, PageWalkSignals};
pub use pdfium_ready::{prime_pdfium, PdfPrimeError};
pub use region_assets::{
    should_write_region_figure, tables_by_page, write_caption_region_assets, WrittenTableAsset,
};
pub use vision_extract::{
    VisionAssetWritePlan, VisionExtractConfig, VisionExtractOverlay, DOC_META_VISION_EXTRACT,
    META_CHART_SYSTEM_PROMPT, META_EXTRACT_CHARTS, META_EXTRACT_FIGURES, META_EXTRACT_IMAGES,
    META_FIGURE_SYSTEM_PROMPT, META_IMAGE_SYSTEM_PROMPT, META_PAGE_SYSTEM_PROMPT,
    VISION_PROMPT_MAX_BYTES,
};
pub use vision_markdown::{
    assemble_vision_markdown, assemble_vision_markdown_with_figures,
    assemble_vision_markdown_with_options, assemble_vision_markdown_with_overrides,
    assemble_vision_markdown_with_policy, enrich_markdown_with_viewer_assets,
    inject_on_disk_region_assets, normalize_selected_vision_pages, normalize_vision_pages,
    page_numbers_from_markdown, stitch_page_markdown_in_order, VisionPageSlice,
};
pub use vision_prompts::{
    grounding_judge_user_prompt, grounding_refine_user_prompt, pass_a_system_prompt_for,
    EMPTY_PAGE_ESCALATION_USER_PROMPT, GROUNDING_JUDGE_SYSTEM,
    RAG_PAGE_MANUSCRIPT_VISION_SYSTEM_PROMPT, RAG_PAGE_VISION_SYSTEM_PROMPT,
};
