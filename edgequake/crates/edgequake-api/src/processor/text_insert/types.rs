//! Shared state for text-insert pipeline phases (SPEC-025 6.6).

use std::sync::Arc;

use edgequake_pipeline::{ChunkProgressCallback, Pipeline, ProcessingResult, ProcessingStats};
use edgequake_tasks::TextInsertData;

use crate::processor::ProviderLineage;

pub(super) struct TextInsertPrepared {
    pub document_id: String,
    pub text_content: String,
    pub source_type: String,
    pub tenant_id: Option<String>,
    pub workspace_id: Option<String>,
    pub is_pdf_source: bool,
    pub track_id: String,
    pub data: TextInsertData,
    pub pipeline: Arc<Pipeline>,
    pub provider_lineage: ProviderLineage,
    pub processed_text: String,
    pub chunk_progress_callback: ChunkProgressCallback,
}

pub(super) struct TextInsertExtracted {
    pub prepared: TextInsertPrepared,
    pub result: ProcessingResult,
}

pub(super) struct TextInsertPersisted {
    pub prepared: TextInsertPrepared,
    pub result: ProcessingResult,
    pub workspace_id_meta: String,
    pub storage_errors: Vec<String>,
    pub stats_with_lineage: ProcessingStats,
    pub final_status: String,
}
