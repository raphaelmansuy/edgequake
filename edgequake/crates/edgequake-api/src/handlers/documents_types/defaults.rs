//! Default value functions for serde deserialization.
//!
//! These functions provide default values for request fields when
//! not specified in the incoming JSON. Used via `#[serde(default = "...")]`.

/// Default: enable gleaning for higher quality entity extraction.
pub fn default_enable_gleaning() -> bool {
    true
}

/// Default: 1 gleaning pass (hard-capped at [`edgequake_pipeline::MAX_GLEANING_CAP`]).
pub fn default_max_gleaning() -> usize {
    edgequake_pipeline::clamp_max_gleaning(1)
}

/// Clamp request `max_gleaning` to the OPS-P1.6 safety cap (pure helper).
pub fn clamp_request_max_gleaning(raw: usize) -> usize {
    edgequake_pipeline::clamp_max_gleaning(raw)
}

/// Default: enable LLM-powered description summarization.
pub fn default_use_llm_summarization() -> bool {
    true
}

/// Default: page 1 (1-indexed).
pub fn default_page() -> usize {
    1
}

/// Default: 20 items per page.
pub fn default_page_size() -> usize {
    20
}

/// Default: scan subdirectories recursively.
pub fn default_recursive() -> bool {
    true
}

/// Default: max 1000 files per scan.
pub fn default_max_files() -> usize {
    1000
}

/// Default true value for document-related boolean fields.
pub fn documents_default_true() -> bool {
    true
}

/// Default: max 100 documents to reprocess.
pub fn default_max_reprocess() -> usize {
    100
}

/// Default: 15 minutes before considering a document stuck (aligned with INV-07).
pub fn default_stuck_threshold_minutes() -> u64 {
    15
}

/// Default maximum retry attempts for chunks.
pub fn default_max_chunk_retries() -> usize {
    3
}
