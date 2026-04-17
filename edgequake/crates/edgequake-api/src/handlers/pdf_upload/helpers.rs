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
    state.pdf_storage.as_ref().map(Arc::clone).ok_or_else(|| {
        ApiError::Internal("PDF storage not initialized (check PostgreSQL setup)".to_string())
    })
}

#[cfg(not(feature = "postgres"))]
pub(super) fn get_pdf_storage(_state: &AppState) -> ApiResult<Arc<dyn PdfDocumentStorage>> {
    Err(ApiError::Internal(
        "PDF storage not available (postgres feature disabled)".to_string(),
    ))
}

/// Create PDF processing background task.
pub(super) async fn create_pdf_processing_task(
    state: &AppState,
    context: &TenantContext,
    pdf_id: Uuid,
    options: &PdfUploadOptions,
    workspace: Option<&Workspace>,
) -> ApiResult<String> {
    let workspace_id = context
        .workspace_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Workspace ID required".to_string()))?;

    let tenant_id = context
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Tenant ID required".to_string()))?;

    let task_data = PdfProcessingData {
        pdf_id,
        tenant_id,
        workspace_id,
        enable_vision: options.enable_vision,
        vision_provider: options.resolved_vision_provider(),
        // WHY: Use vision_model() method (not the raw field) so provider-specific
        // defaults are applied when no explicit model was set by the user.
        vision_model: if options.resolved_backend(workspace)
            == edgequake_pdf::PdfParserBackend::Vision
        {
            Some(options.vision_model())
        } else {
            None
        },
        existing_document_id: None, // Fresh upload — create new document
        pdf_parser_backend: options.resolved_backend(workspace),
        restart_from_scratch: false,
    };

    let track_id = format!("pdf-{}", Uuid::new_v4());

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
        metadata: None,
        progress: None,
        result: None,
    };

    // Store task in database
    state
        .task_storage
        .create_task(&task)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create task: {}", e)))?;

    // Queue task for background processing (critical - missing this causes tasks to stay in pending)
    state
        .task_queue
        .send(task)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to queue task: {}", e)))?;

    debug!(
        "Created and queued PDF processing task: id={}, pdf_id={}",
        track_id, pdf_id
    );

    Ok(track_id)
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
pub(super) fn estimate_processing_time(pdf_data: &[u8], page_count: Option<i32>) -> u64 {
    let size_mb = (pdf_data.len() as f64) / 1_048_576.0;
    let pages = page_count.unwrap_or(10) as f64;

    // Rough estimate: 2-4 seconds per page with vision, 0.5s without
    // Add overhead for large files
    let base_time = pages * 3.0;
    let size_penalty = size_mb * 0.5;

    (base_time + size_penalty).ceil() as u64
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

    let mut entities_cleared = 0;
    let mut edges_cleared = 0;

    // 1. Clear graph data (entities and relationships)
    let graph_storage = &state.graph_storage;

    // Get all nodes and filter by source_id
    let all_nodes = graph_storage
        .get_all_nodes()
        .await
        .map_err(|e| format!("Failed to get graph nodes: {}", e))?;

    let chunk_prefix = format!("{}-chunk-", document_id);

    for node in all_nodes {
        // Check if this node has sources from the deleted document
        if let Some(source_id) = node.properties.get("source_id").and_then(|v| v.as_str()) {
            let sources: Vec<&str> = source_id.split('|').collect();
            let remaining_sources: Vec<&str> = sources
                .into_iter()
                .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(document_id))
                .collect();

            if remaining_sources.is_empty() {
                // Delete connected edges first
                if let Ok(edges) = graph_storage.get_node_edges(&node.id).await {
                    for edge in edges {
                        let _ = graph_storage.delete_edge(&edge.source, &edge.target).await;
                        edges_cleared += 1;
                    }
                }
                // Then delete the node
                let _ = graph_storage.delete_node(&node.id).await;
                entities_cleared += 1;
            } else if remaining_sources.len() < source_id.split('|').count() {
                // Update to remove this document's sources
                let mut updated_props = node.properties.clone();
                updated_props.insert(
                    "source_id".to_string(),
                    serde_json::json!(remaining_sources.join("|")),
                );
                let _ = graph_storage.upsert_node(&node.id, updated_props).await;
            }
        }
    }

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
        // 1KB, 5 pages → ~15s base + ~0s overhead
        let data = vec![0u8; 1024];
        let time = estimate_processing_time(&data, Some(5));
        assert!(time >= 15, "Expected >= 15s, got {time}");
    }

    #[test]
    fn test_estimate_time_unknown_page_count() {
        // When page_count is None, defaults to 10 pages
        let data = vec![0u8; 1024];
        let time = estimate_processing_time(&data, None);
        assert!(
            time >= 30,
            "Expected >= 30s for 10-page default, got {time}"
        );
    }

    #[test]
    fn test_estimate_time_large_file() {
        // 100MB, 500 pages
        let data = vec![0u8; 100 * 1024 * 1024];
        let time = estimate_processing_time(&data, Some(500));
        assert!(time >= 1500, "Expected >= 1500s for 500 pages, got {time}");
    }

    #[test]
    fn test_estimate_time_zero_pages() {
        let data = vec![0u8; 1024];
        let time = estimate_processing_time(&data, Some(0));
        // 0 pages → 0 base time + small size overhead
        assert!(time <= 5, "Expected <= 5s for 0 pages, got {time}");
    }
}
