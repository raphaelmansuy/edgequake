//! Multimodal chunk modality-relation injection (LightRAG `operate.extract_entities` subset).

mod display;
mod injection;
mod retrieval_modality;

pub use display::{
    bare_entity_id, doc_short_title, is_placeholder_mm_name, parse_drawing_item_locus,
    parse_mm_display_name, resolve_mm_display_from_node_props, resolve_mm_entity_display,
    DrawingItemKind, DrawingItemLocus, MmDisplayInput, MmDisplayLabel,
};
pub use injection::{
    inject_modality_relations, MmChunkSidecarMeta, MmHeadingBlock, MmSidecarBlock, MmSidecarRef,
};
pub use retrieval_modality::{
    map_image_type_to_retrieval_modality, resolve_retrieval_modality_from_content,
    resolve_retrieval_modality_from_mm, stamp_retrieval_modality_on_chunks, MODALITY_CHART,
    MODALITY_EQUATION, MODALITY_FIGURE, MODALITY_IMAGE, MODALITY_TABLE,
};
