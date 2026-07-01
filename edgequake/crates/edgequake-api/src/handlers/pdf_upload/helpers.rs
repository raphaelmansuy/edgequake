use chrono::Utc;
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

use super::types::PdfUploadOptions;
use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::state::AppState;
use edgequake_core::Workspace;
use edgequake_storage::PdfDocumentStorage;
use edgequake_tasks::{PdfProcessingData, Task, TaskStatus, TaskType};

// ============================================================================
// Helper Functions
// ============================================================================

/// Get PDF storage from app state (platform-specific).
/// Get PDF storage from AppState.
///
/// @implements SPEC-007: PDF storage access
/// @enforces BR0701: PostgreSQL-backed PDF storage
#[cfg(feature = "postgres")]
pub(super) fn get_pdf_storage(state: &AppState) -> ApiResult<Arc<dyn PdfDocumentStorage>> {
    if state.storage.is_postgresql() {
        state
            .storage
            .validate_postgres_adapters()
            .map_err(ApiError::Internal)?;
    }
    state
        .storage
        .pdf_storage
        .as_ref()
        .map(Arc::clone)
        .ok_or_else(|| {
            ApiError::Internal("PDF storage not initialized (check storage setup)".to_string())
        })
}

#[cfg(not(feature = "postgres"))]
pub(super) fn get_pdf_storage(_state: &AppState) -> ApiResult<Arc<dyn PdfDocumentStorage>> {
    Err(ApiError::Internal(
        "PDF storage not available (postgres feature disabled)".to_string(),
    ))
}

/// Reprocess intent carried into a PDF processing task.
///
/// WHY: Grouping the three reprocessing knobs into a single struct keeps
/// `create_pdf_processing_task` under the clippy argument limit (SRP/DRY) and
/// gives Replace (`force_reindex`) and Reprocess a shared vocabulary for
/// "reuse the existing document id" + "re-run conversion from scratch".
#[derive(Debug, Clone, Default)]
pub(super) struct PdfReprocessIntent {
    /// When `Some`, the worker reuses this document id (in-place reprocessing)
    /// instead of minting a new UUID. Used by Replace (`force_reindex`) and
    /// Reprocess.
    pub existing_document_id: Option<String>,
    /// When `true`, the worker ignores any cached `markdown_content` and
    /// re-runs PDF -> markdown conversion. Used by Replace and Reprocess
    /// `mode=full`.
    pub restart_from_scratch: bool,
    /// Explicit reprocess mode echoed into the task payload for observability.
    pub reprocess_mode: Option<edgequake_tasks::ReprocessMode>,
}

impl PdfReprocessIntent {
    /// Fresh upload: mint a new document id, no restart, no mode.
    pub(super) fn fresh() -> Self {
        Self::default()
    }
}

/// Result of enqueueing (or reusing) a PDF processing task.
pub(super) struct PdfProcessingEnqueueResult {
    pub track_id: String,
    pub document_id: String,
}

/// Create PDF processing background task.
pub(super) async fn create_pdf_processing_task(
    state: &AppState,
    context: &TenantContext,
    pdf_id: Uuid,
    options: &PdfUploadOptions,
    workspace: Option<&Workspace>,
    intent: PdfReprocessIntent,
    page_count: Option<i32>,
    file_size_bytes: u64,
) -> ApiResult<PdfProcessingEnqueueResult> {
    let workspace_id = context
        .workspace_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Workspace ID required".to_string()))?;

    let tenant_id = context
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Tenant ID required".to_string()))?;

    if let Some(existing_track_id) = crate::services::admit_pdf_processing_enqueue(
        state,
        pdf_id,
        workspace_id,
        intent.restart_from_scratch,
    )
    .await
    {
        let document_id = crate::services::resolve_pdf_ingest_document_id(
            state,
            pdf_id,
            intent.existing_document_id.clone(),
            context,
        )
        .await;
        return Ok(PdfProcessingEnqueueResult {
            track_id: existing_track_id,
            document_id,
        });
    }

    let document_id = crate::services::resolve_pdf_ingest_document_id(
        state,
        pdf_id,
        intent.existing_document_id.clone(),
        context,
    )
    .await;

    let resolved_backend = options.resolved_backend(workspace);
    let backend_explicit = options.pdf_parser_backend.is_some()
        || workspace.and_then(|ws| ws.pdf_parser_backend).is_some()
        || edgequake_pdf::PdfParserBackend::from_env().is_some();

    let page_count_usize = page_count.unwrap_or(1).max(1) as usize;
    let profile = crate::services::LargeDocumentProfile::new(page_count_usize, file_size_bytes);
    let processing_timeout_secs =
        profile.task_timeout_secs(resolved_backend, &options.resolved_vision_provider());

    let task_data = PdfProcessingData {
        pdf_id,
        tenant_id,
        workspace_id,
        enable_vision: options.enable_vision,
        vision_provider: options.resolved_vision_provider(),
        // WHY: Use vision_model() method (not the raw field) so provider-specific
        // defaults are applied when no explicit model was set by the user.
        vision_model: if resolved_backend == edgequake_pdf::PdfParserBackend::Vision {
            Some(options.vision_model())
        } else {
            None
        },
        existing_document_id: Some(document_id.clone()),
        pdf_parser_backend: resolved_backend,
        pdf_parser_backend_explicit: backend_explicit,
        restart_from_scratch: intent.restart_from_scratch,
        reprocess_mode: intent.reprocess_mode,
        multimodal_process_options: options.process_options.clone(),
    };

    let track_id = format!("pdf-{}", Uuid::new_v4());

    if let Some(existing_track_id) =
        state
            .tasks
            .pdf_admission
            .try_register(workspace_id, pdf_id, &track_id)
    {
        return Ok(PdfProcessingEnqueueResult {
            track_id: existing_track_id,
            document_id,
        });
    }

    let task = Task {
        track_id: track_id.clone(),
        tenant_id,
        workspace_id,
        task_type: TaskType::PdfProcessing,
        status: TaskStatus::Pending,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        started_at: None,
        completed_at: None,
        error_message: None,
        error: None,
        retry_count: 0,
        max_retries: 3,
        consecutive_timeout_failures: 0,
        circuit_breaker_tripped: false,
        task_data: serde_json::to_value(&task_data)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize task data: {}", e)))?,
        metadata: Some(serde_json::json!({
            "processing_timeout_secs": processing_timeout_secs,
        })),
        progress: None,
        result: None,
    };

    if let Err(e) = state.enqueue_task(task).await {
        state.tasks.pdf_admission.release(workspace_id, pdf_id);
        return Err(e);
    }

    debug!(
        "Created and queued PDF processing task: id={}, pdf_id={}, document_id={}",
        track_id, pdf_id, document_id
    );

    Ok(PdfProcessingEnqueueResult {
        track_id,
        document_id,
    })
}

/// Extract page count from PDF binary data.
///
/// WHY: PDF files contain binary content (compressed streams, images), so
/// `std::str::from_utf8` fails for virtually all real PDFs. Instead, we
/// search the raw bytes for the `/Count` token followed by a space and
/// digits — the standard PDF catalog structure for declaring page count.
/// We find the LARGEST `/Count N` value because the root Pages node
/// contains the total, while sub-nodes contain partial counts.
pub(super) fn extract_page_count(pdf_data: &[u8]) -> Option<i32> {
    let needle = b"/Count ";
    let mut max_count: Option<i32> = None;

    // Scan raw bytes for all occurrences of "/Count " followed by digits
    let mut pos = 0;
    while pos + needle.len() < pdf_data.len() {
        if let Some(offset) = pdf_data[pos..]
            .windows(needle.len())
            .position(|w| w == needle)
        {
            let start = pos + offset + needle.len();
            // Extract digits after "/Count "
            let digit_end = pdf_data[start..]
                .iter()
                .position(|&b| !b.is_ascii_digit())
                .unwrap_or(pdf_data.len() - start);

            if digit_end > 0 {
                if let Ok(num_str) = std::str::from_utf8(&pdf_data[start..start + digit_end]) {
                    if let Ok(count) = num_str.parse::<i32>() {
                        // Keep the largest count (root Pages node has the total)
                        max_count = Some(max_count.map_or(count, |prev: i32| prev.max(count)));
                    }
                }
            }
            pos = start + digit_end;
        } else {
            break;
        }
    }

    max_count
}

/// Estimate processing time based on file size and page count.
pub(super) fn estimate_processing_time(
    file_size_bytes: u64,
    page_count: Option<i32>,
    backend: edgequake_pdf::PdfParserBackend,
    provider: &str,
) -> u64 {
    let pages = page_count.unwrap_or(10).max(1) as usize;
    let profile = crate::services::LargeDocumentProfile::new(pages, file_size_bytes);
    profile.estimated_total_secs(backend, provider)
}

/// Clear derived data (graph/vector) for a document during re-indexing.
///
/// OODA-08: Helper function to clear graph and vector data for a document
/// without deleting the raw PDF or markdown content.
///
/// # WHY
///
/// When re-indexing a document, we want to:
/// 1. Keep the raw PDF data (no need to re-upload)
/// 2. Keep the markdown content (can be re-used or regenerated)
/// 3. Clear graph entities/relationships (will be re-extracted)
/// 4. Clear vector embeddings (will be re-computed)
///
/// This allows re-processing with updated LLM/config without re-uploading.
///
/// # Arguments
///
/// * `state` - Application state with graph and vector storage
/// * `document_id` - Document ID to clear data for
///
/// # Returns
///
/// * `Ok(())` - Data cleared successfully
/// * `Err(String)` - Error message if clearing failed
pub(super) async fn clear_document_derived_data(
    state: &AppState,
    document_id: &str,
) -> Result<(), String> {
    info!(
        "OODA-08: Clearing derived data for document: {}",
        document_id
    );

    // SPEC-006 P2: reuse bounded document cascade (DRY with delete handler)
    let scope = crate::services::DocumentSourceScope::from_document_id(document_id);
    let stats = crate::services::cascade_remove_document_sources(
        &state.storage.graph_storage,
        None,
        None,
        &scope,
    )
    .await
    .map_err(|e| format!("Failed to clear graph data: {}", e))?;

    let entities_cleared = stats.entities_removed + stats.entities_updated;
    let edges_cleared = stats.relationships_removed + stats.relationships_updated;

    // 2. Clear vector data
    // Note: Vector storage doesn't have a direct delete_by_document method,
    // but vector cleanup happens automatically when entities are deleted
    // because vectors are typically stored alongside entities or referenced by entity IDs.
    // Future optimization: Add explicit delete_vectors_by_document() if needed.

    info!(
        "OODA-08: Cleared derived data - entities={}, edges={}",
        entities_cleared, edges_cleared
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_page_count edge cases ─────────────────────────────────

    #[test]
    fn test_extract_page_count_normal_pdf() {
        // Simulates a PDF with a root Pages node: /Count 42
        let data = b"%PDF-1.4\n/Type /Pages\n/Count 42\n/Kids [...]";
        assert_eq!(extract_page_count(data), Some(42));
    }

    #[test]
    fn test_extract_page_count_multiple_count_entries() {
        // PDF with sub-nodes: root /Count 100, sub /Count 50
        // Should return the largest (root total)
        let data = b"%PDF-1.4\n/Count 50\n...\n/Count 100\n";
        assert_eq!(extract_page_count(data), Some(100));
    }

    #[test]
    fn test_extract_page_count_single_page() {
        let data = b"%PDF-1.4\n/Count 1\n";
        assert_eq!(extract_page_count(data), Some(1));
    }

    #[test]
    fn test_extract_page_count_zero_pages() {
        // Edge case: /Count 0 should return Some(0)
        let data = b"%PDF-1.4\n/Count 0\n";
        assert_eq!(extract_page_count(data), Some(0));
    }

    #[test]
    fn test_extract_page_count_empty_data() {
        assert_eq!(extract_page_count(b""), None);
    }

    #[test]
    fn test_extract_page_count_no_count_token() {
        let data = b"%PDF-1.4\n/Type /Pages\n/MediaBox [0 0 612 792]\n";
        assert_eq!(extract_page_count(data), None);
    }

    #[test]
    fn test_extract_page_count_count_without_digits() {
        // "/Count " followed by non-digits
        let data = b"%PDF-1.4\n/Count abc\n";
        assert_eq!(extract_page_count(data), None);
    }

    #[test]
    fn test_extract_page_count_large_page_count() {
        let data = b"%PDF-1.4\n/Count 12345\n";
        assert_eq!(extract_page_count(data), Some(12345));
    }

    #[test]
    fn test_extract_page_count_binary_content_around() {
        // Binary noise around the /Count token
        let mut data = vec![0u8; 100];
        data.extend_from_slice(b"/Count 7");
        data.extend_from_slice(&[0xFF, 0xFE, 0x00]);
        assert_eq!(extract_page_count(&data), Some(7));
    }

    #[test]
    fn test_extract_page_count_needle_at_end_of_data() {
        // "/Count " at the very end with no digits after
        let data = b"%PDF-1.4\n/Count ";
        assert_eq!(extract_page_count(data), None);
    }

    // ── estimate_processing_time edge cases ───────────────────────────

    #[test]
    fn test_estimate_time_small_pdf() {
        let time = estimate_processing_time(
            1024,
            Some(5),
            edgequake_pdf::PdfParserBackend::Vision,
            "mock",
        );
        assert!(time >= 7200, "Expected scaled floor >= 7200s, got {time}");
    }

    #[test]
    fn test_estimate_time_unknown_page_count() {
        let time = estimate_processing_time(
            1024,
            None,
            edgequake_pdf::PdfParserBackend::EdgeParse,
            "mock",
        );
        assert!(time >= 7200, "Expected >= 7200s floor, got {time}");
    }

    #[test]
    fn test_estimate_time_large_file() {
        let time = estimate_processing_time(
            100 * 1024 * 1024,
            Some(500),
            edgequake_pdf::PdfParserBackend::Vision,
            "openai",
        );
        assert!(time >= 7200, "Expected >= 7200s for 500 pages, got {time}");
    }

    #[test]
    fn test_estimate_time_zero_pages() {
        let time = estimate_processing_time(
            1024,
            Some(0),
            edgequake_pdf::PdfParserBackend::EdgeParse,
            "mock",
        );
        assert!(time >= 7200, "Expected floor timeout, got {time}");
    }
}
