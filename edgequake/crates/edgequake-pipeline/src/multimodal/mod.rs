//! Multimodal chunk modality-relation injection (LightRAG `operate.extract_entities` subset).

mod injection;

pub use injection::{
    inject_modality_relations, parse_mm_display_name, MmChunkSidecarMeta, MmHeadingBlock,
    MmSidecarBlock, MmSidecarRef,
};
