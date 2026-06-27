//! Multimodal analyze subsystem (LightRAG parity, SOLID module boundaries).

mod analyzer;
mod api_views;
mod assets;
mod blocks;
mod cache;
mod chunk_budget;
mod chunks;
mod chunks_store;
mod context;
mod enrich;
mod gates;
mod item_record;
mod json_recovery;
mod manifest;
mod manifest_store;
mod metadata;
mod prompt_context;
mod prompts;
mod providers;
mod reanalyze;
mod sanitize;
mod scan;
mod sidecar;
mod stage;
mod standalone;
mod surrounding;

pub use analyzer::{analyze_image_bytes, analyze_multimodal_images, AnalyzeOutcome};
pub use api_views::{manifest_item_status_views, summary_from_metadata, MultimodalItemStatusView};
pub use blocks::{
    block_id_for_offset, blocks_map_from_sections, content_for_item, enrich_items_with_block_ids,
    load_content_rows_by_blockid, load_content_rows_by_blockid_jsonl, prepare_analyze_blocks,
    resolve_blocks_for_analyze, split_markdown_sections, virtual_block_map, MarkdownSection,
    VIRTUAL_BLOCK_ID,
};
pub use cache::{
    analysis_cache_enabled, analysis_cache_key, attach_cache_key, compute_args_hash,
    generate_cache_key, maybe_attach_cache_key,
};
pub use chunk_budget::{
    max_mm_chunk_tokens, min_mm_chunk_description_tokens, render_mm_chunk_with_budget,
};
pub use chunks::{
    append_mm_chunks_to_text, build_mm_chunks_from_manifest, collect_mm_chunks_from_manifest,
    enrich_processed_text_with_mm_chunks, mm_chunks_enabled, render_mm_chunk, MmChunkBuildError,
    MultimodalChunk,
};
pub use chunks_store::{load_mm_chunks, mm_chunks_key, persist_mm_chunks};
pub use context::{
    max_extract_input_chars, max_extract_input_tokens, trim_content_to_budget, SurroundingContext,
};
pub use enrich::enrich_markdown_with_vlm;
pub use gates::{should_run_image_analysis, vlm_process_enabled, MultimodalFailMode};
pub use item_record::{MultimodalItemRecord, MultimodalItemStatus, MultimodalSummary};
pub use json_recovery::{extract_json_object, parse_json_object};
pub use manifest::{ManifestItem, MultimodalManifest};
pub use manifest_store::{
    load_manifest, manifest_key, metadata_multimodal_patch, persist_manifest,
};
pub use metadata::{
    apply_process_options_to_metadata, resolve_process_options_from_metadata, METADATA_FIELD,
};
pub use prompt_context::{table_content_format_label, PromptContext};
pub use prompts::{
    equation_analysis_messages, image_analysis_messages, json_repair_user_message,
    table_analysis_messages,
};
pub use providers::MultimodalProviders;
pub use reanalyze::{
    reanalyze_document_multimodal, MultimodalReanalyzeOutcome, MultimodalReanalyzeParams,
};
pub use scan::{find_table_cite_span, scan_manifest_items, span_for_item};
pub use sidecar::{
    build_sidecar_block, MultimodalHeading, MultimodalSidecar, MultimodalSidecarRef,
};
pub use stage::{run_multimodal_analyze_stage, run_multimodal_analyze_stage_outcome};
pub use standalone::{analyze_standalone_image, StandaloneImageOutcome};
pub use surrounding::{
    build_surrounding, char_trim_trailing, find_target_span, load_chunk_separators,
    remove_table_tags, row_trim_table_trailing, strip_internal_multimodal_markup, SurroundingKind,
    SurroundingTokenCounter,
};
