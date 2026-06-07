//! Stage bridge — single conversion layer between progress hierarchies (SPEC-017 DRY-006).
//!
//! Three layers remain distinct by responsibility, but all mappings live here:
//!
//! | Layer | Type | Crate | Role |
//! |-------|------|-------|------|
//! | Tasks | `PipelinePhase` (wire: snake_case) | edgequake-tasks | Live PDF upload WebSocket |
//! | Unified | `UnifiedStage` (wire: lowercase) | ingestion_types | Frontend status badges |
//! | Internal | `PipelineStage` | progress | Async job tracker |
//!
//! Full struct collapse (`IngestionProgress` variants) is deferred — different serde shapes.

use crate::ingestion_types::UnifiedStage;
use crate::progress::PipelineStage;

/// Convert internal pipeline stage → frontend unified stage.
pub fn pipeline_stage_to_unified(stage: PipelineStage) -> UnifiedStage {
    match stage {
        PipelineStage::Preprocessing => UnifiedStage::Preprocessing,
        PipelineStage::Chunking => UnifiedStage::Chunking,
        PipelineStage::Extracting => UnifiedStage::Extracting,
        PipelineStage::Gleaning => UnifiedStage::Gleaning,
        PipelineStage::Merging => UnifiedStage::Merging,
        PipelineStage::Summarizing => UnifiedStage::Summarizing,
        PipelineStage::Embedding => UnifiedStage::Embedding,
        PipelineStage::Storing | PipelineStage::Finalizing => UnifiedStage::Storing,
    }
}

/// Convert unified stage → internal pipeline stage when applicable.
pub fn unified_to_pipeline_stage(stage: UnifiedStage) -> Option<PipelineStage> {
    match stage {
        UnifiedStage::Preprocessing => Some(PipelineStage::Preprocessing),
        UnifiedStage::Chunking => Some(PipelineStage::Chunking),
        UnifiedStage::Extracting => Some(PipelineStage::Extracting),
        UnifiedStage::Gleaning => Some(PipelineStage::Gleaning),
        UnifiedStage::Merging => Some(PipelineStage::Merging),
        UnifiedStage::Summarizing => Some(PipelineStage::Summarizing),
        UnifiedStage::Embedding => Some(PipelineStage::Embedding),
        UnifiedStage::Storing => Some(PipelineStage::Storing),
        UnifiedStage::Uploading
        | UnifiedStage::Converting
        | UnifiedStage::Completed
        | UnifiedStage::Failed => None,
    }
}

/// Map `edgequake_tasks::PipelinePhase` JSON slug → unified stage (no tasks crate dependency).
///
/// Slugs match `#[serde(rename_all = "snake_case")]` on `PipelinePhase`.
pub fn tasks_phase_slug_to_unified(slug: &str) -> Option<UnifiedStage> {
    match slug {
        "upload" => Some(UnifiedStage::Uploading),
        "pdf_conversion" => Some(UnifiedStage::Converting),
        "chunking" => Some(UnifiedStage::Chunking),
        "embedding" => Some(UnifiedStage::Embedding),
        "extraction" => Some(UnifiedStage::Extracting),
        "graph_storage" => Some(UnifiedStage::Storing),
        _ => None,
    }
}

/// Unified stage → tasks phase slug when a 1:1 mapping exists.
pub fn unified_to_tasks_phase_slug(stage: UnifiedStage) -> Option<&'static str> {
    match stage {
        UnifiedStage::Uploading => Some("upload"),
        UnifiedStage::Converting => Some("pdf_conversion"),
        UnifiedStage::Chunking => Some("chunking"),
        UnifiedStage::Embedding => Some("embedding"),
        UnifiedStage::Extracting => Some("extraction"),
        UnifiedStage::Storing => Some("graph_storage"),
        UnifiedStage::Preprocessing
        | UnifiedStage::Gleaning
        | UnifiedStage::Merging
        | UnifiedStage::Summarizing
        | UnifiedStage::Completed
        | UnifiedStage::Failed => None,
    }
}

/// Wire slug for unified stage (`#[serde(rename_all = "lowercase")]`).
pub fn unified_stage_slug(stage: UnifiedStage) -> &'static str {
    match stage {
        UnifiedStage::Uploading => "uploading",
        UnifiedStage::Converting => "converting",
        UnifiedStage::Preprocessing => "preprocessing",
        UnifiedStage::Chunking => "chunking",
        UnifiedStage::Extracting => "extracting",
        UnifiedStage::Gleaning => "gleaning",
        UnifiedStage::Merging => "merging",
        UnifiedStage::Summarizing => "summarizing",
        UnifiedStage::Embedding => "embedding",
        UnifiedStage::Storing => "storing",
        UnifiedStage::Completed => "completed",
        UnifiedStage::Failed => "failed",
    }
}

impl From<PipelineStage> for UnifiedStage {
    fn from(stage: PipelineStage) -> Self {
        pipeline_stage_to_unified(stage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_to_unified_roundtrip_for_processing_stages() {
        for internal in PipelineStage::all() {
            let unified = pipeline_stage_to_unified(internal);
            let back = unified_to_pipeline_stage(unified);
            if internal == PipelineStage::Finalizing {
                assert_eq!(back, Some(PipelineStage::Storing));
            } else {
                assert_eq!(back, Some(internal));
            }
        }
    }

    #[test]
    fn tasks_slug_maps_to_unified() {
        assert_eq!(
            tasks_phase_slug_to_unified("pdf_conversion"),
            Some(UnifiedStage::Converting)
        );
        assert_eq!(
            tasks_phase_slug_to_unified("extraction"),
            Some(UnifiedStage::Extracting)
        );
    }

    #[test]
    fn unified_slug_matches_frontend_contract() {
        assert_eq!(unified_stage_slug(UnifiedStage::Extracting), "extracting");
        assert_eq!(
            unified_to_tasks_phase_slug(UnifiedStage::Extracting),
            Some("extraction")
        );
    }
}
