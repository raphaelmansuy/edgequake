//! Task types module.
//!
//! Defines the core types for background task processing:
//! - Status and type enums
//! - Task struct with lifecycle management and circuit breaker
//! - Progress tracking (step-level and chunk-level)
//! - Failure information with structured error categories
//! - Task-specific data payloads

mod data;
mod failure;
mod progress;
mod status;
mod task;

pub use data::*;
pub use failure::*;
pub use progress::*;
pub use status::*;
pub use task::*;

#[cfg(test)]
mod tests {
    use super::*;

    // Test helper constants for tenant/workspace IDs
    const TEST_TENANT_ID: &str = "00000000-0000-0000-0000-000000000001";
    const TEST_WORKSPACE_ID: &str = "00000000-0000-0000-0000-000000000002";

    fn test_tenant_id() -> uuid::Uuid {
        uuid::Uuid::parse_str(TEST_TENANT_ID).unwrap()
    }

    fn test_workspace_id() -> uuid::Uuid {
        uuid::Uuid::parse_str(TEST_WORKSPACE_ID).unwrap()
    }

    #[test]
    fn test_task_creation() {
        let data = serde_json::json!({
            "file_path": "/tmp/test.pdf",
            "workspace_id": "default"
        });

        let task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Upload,
            data,
        );

        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.task_type, TaskType::Upload);
        assert!(task.track_id.starts_with("upload-"));
        assert_eq!(task.retry_count, 0);
        assert_eq!(task.max_retries, 3);
    }

    #[test]
    fn document_id_prefers_existing_document_id() {
        let task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::PdfProcessing,
            serde_json::json!({
                "existing_document_id": "doc-abc",
                "document_id": "doc-ignored",
                "metadata": { "document_id": "doc-also-ignored" }
            }),
        );
        assert_eq!(task.document_id().as_deref(), Some("doc-abc"));
    }

    #[test]
    fn document_id_from_insert_metadata() {
        let task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            serde_json::json!({
                "metadata": { "document_id": "  doc-xyz  " }
            }),
        );
        assert_eq!(task.document_id().as_deref(), Some("doc-xyz"));
    }

    #[test]
    fn document_id_from_task_metadata_field() {
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::PdfProcessing,
            serde_json::json!({ "pdf_id": uuid::Uuid::new_v4().to_string() }),
        );
        task.metadata = Some(serde_json::json!({ "document_id": "doc-from-meta" }));
        assert_eq!(task.document_id().as_deref(), Some("doc-from-meta"));
    }

    #[test]
    fn document_id_ignores_blank() {
        let task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            serde_json::json!({ "document_id": "   " }),
        );
        assert_eq!(task.document_id(), None);
    }

    #[test]
    fn test_task_lifecycle() {
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );

        assert_eq!(task.status, TaskStatus::Pending);

        task.mark_processing();
        assert_eq!(task.status, TaskStatus::Processing);
        assert!(task.started_at.is_some());

        assert!(task.mark_success(serde_json::json!({"result": "success"})));
        assert_eq!(task.status, TaskStatus::Indexed);
        assert!(task.completed_at.is_some());
        assert!(task.result.is_some());
    }

    #[test]
    fn e2e_cancelled_cannot_mark_success() {
        // X-29 / matrix: Cancelled ↛ Success
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            serde_json::json!({}),
        );
        task.status = TaskStatus::Cancelled;
        assert!(!task.mark_success(serde_json::json!({ "ok": true })));
        assert_eq!(task.status, TaskStatus::Cancelled);
        assert!(task.result.is_none());
    }

    #[test]
    fn test_task_retry_logic() {
        let data = serde_json::json!({});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Upload,
            data,
        );

        assert!(!task.is_terminal());

        task.mark_failed("Error 1".to_string());
        assert_eq!(task.retry_count, 1);
        assert!(task.can_retry());

        task.mark_failed("Error 2".to_string());
        assert_eq!(task.retry_count, 2);
        assert!(task.can_retry());

        task.mark_failed("Error 3".to_string());
        assert_eq!(task.retry_count, 3);
        assert!(!task.can_retry());
        assert!(task.is_terminal());
    }

    #[test]
    fn test_task_progress() {
        let data = serde_json::json!({});
        let mut task = Task::new(test_tenant_id(), test_workspace_id(), TaskType::Scan, data);

        task.update_progress("parsing_files".to_string(), 4, 25);
        assert!(task.progress.is_some());

        let progress = task.progress.as_ref().unwrap();
        assert_eq!(progress.current_step, "parsing_files");
        assert_eq!(progress.total_steps, 4);
        assert_eq!(progress.percent_complete, 25);
    }

    #[test]
    fn test_generate_track_id() {
        let track_id = generate_track_id(TaskType::Upload);
        assert!(track_id.starts_with("upload-"));

        let track_id2 = generate_track_id(TaskType::Insert);
        assert!(track_id2.starts_with("insert-"));

        // IDs should be unique
        assert_ne!(track_id, track_id2);
    }

    #[test]
    fn test_task_error_creation() {
        let error = TaskFailureInfo::new(
            "Test error",
            "chunking",
            "Invalid format",
            "Check the file format",
            true,
        );

        assert_eq!(error.message, "Test error");
        assert_eq!(error.step, "chunking");
        assert_eq!(error.reason, "Invalid format");
        assert_eq!(error.suggestion, "Check the file format");
        assert!(error.retryable);
    }

    #[test]
    fn test_task_error_helpers() {
        let chunking_error = TaskFailureInfo::chunking("Invalid UTF-8");
        assert_eq!(chunking_error.step, "chunking");
        assert!(chunking_error.retryable);

        let embedding_error = TaskFailureInfo::embedding("API timeout");
        assert_eq!(embedding_error.step, "embedding");

        let extraction_error = TaskFailureInfo::extraction("No entities found");
        assert_eq!(extraction_error.step, "extraction");

        let indexing_error = TaskFailureInfo::indexing("Database connection failed");
        assert_eq!(indexing_error.step, "indexing");

        let rate_limit_error = TaskFailureInfo::rate_limit("extraction");
        assert!(rate_limit_error.reason.contains("rate limit"));
    }

    #[test]
    fn test_task_failed_with_details() {
        let data = serde_json::json!({});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );

        let error = TaskFailureInfo::extraction("API rate limit exceeded");
        task.mark_failed_with_details(error);

        assert_eq!(task.status, TaskStatus::Failed);
        assert!(task.error.is_some());
        assert_eq!(task.error.as_ref().unwrap().step, "extraction");
        assert_eq!(
            task.error_message.as_ref().unwrap(),
            "Entity extraction failed"
        );
    }

    #[test]
    fn test_non_retryable_error() {
        let data = serde_json::json!({});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );

        let error = TaskFailureInfo::new(
            "Permanent error",
            "indexing",
            "Invalid data",
            "Contact support",
            false, // Not retryable
        );
        task.mark_failed_with_details(error);

        assert!(!task.can_retry()); // Should not be retryable
    }

    #[test]
    fn spec045_permanent_merge_error_skips_retry() {
        let data = serde_json::json!({});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );

        let error = TaskFailureInfo::from_processing_error(
            "1 knowledge-graph merge error(s) during persist",
        );
        task.mark_failed_with_details(error);

        assert!(!task.can_retry());
        assert_eq!(task.error.as_ref().unwrap().step, "indexing");
        assert_eq!(task.error.as_ref().unwrap().suggestion, "reprocess_full");
    }

    #[test]
    fn spec045_permanent_embedding_400_skips_retry() {
        let data = serde_json::json!({});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );

        let error = TaskFailureInfo::from_processing_error(
            "Embedding error: Too many inputs in request (400)",
        );
        task.mark_failed_with_details(error);

        assert!(!task.can_retry());
    }

    // ============================================
    // Circuit Breaker Tests
    // ============================================

    #[test]
    fn test_consecutive_timeout_increments() {
        // GIVEN: A new task
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );

        // WHEN: Task fails with timeout error
        let error = TaskFailureInfo::timeout("llm_extraction", "LLM request timed out after 300s");
        task.mark_failed_with_details(error);

        // THEN: Consecutive timeout counter increments to 1
        assert_eq!(task.consecutive_timeout_failures, 1);
        assert!(!task.circuit_breaker_tripped);
        assert_eq!(task.status, TaskStatus::Failed);
    }

    #[test]
    fn test_consecutive_timeout_resets_on_success() {
        // GIVEN: A task with 2 consecutive timeouts
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );

        task.mark_failed_with_details(TaskFailureInfo::timeout("step1", "timeout 1"));
        task.mark_failed_with_details(TaskFailureInfo::timeout("step2", "timeout 2"));
        assert_eq!(task.consecutive_timeout_failures, 2);

        // WHEN: Task is retried, claimed, and succeeds (legal path:
        // Failed → Pending → Processing → Indexed; Complete refuses Failed).
        task.requeue_for_retry();
        task.mark_processing();
        task.mark_success(serde_json::json!({"result": "ok"}));

        // THEN: Counter resets to 0
        assert_eq!(task.consecutive_timeout_failures, 0);
        assert_eq!(task.status, TaskStatus::Indexed);
        assert!(!task.circuit_breaker_tripped);
    }

    #[test]
    fn test_consecutive_timeout_resets_on_non_timeout_failure() {
        // GIVEN: A task with 2 consecutive timeouts
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );

        task.mark_failed_with_details(TaskFailureInfo::timeout("step1", "timeout 1"));
        task.mark_failed_with_details(TaskFailureInfo::timeout("step2", "timeout 2"));
        assert_eq!(task.consecutive_timeout_failures, 2);

        // WHEN: Task fails with non-timeout error (network error)
        let network_error = TaskFailureInfo::new(
            "Connection refused",
            "network",
            "Network error",
            "Check network connectivity",
            true, // Retryable
        );
        task.mark_failed_with_details(network_error);

        // THEN: Counter resets to 0 (network errors transient)
        assert_eq!(task.consecutive_timeout_failures, 0);
        assert!(!task.circuit_breaker_tripped);
    }

    #[test]
    fn test_circuit_breaker_trips_at_threshold() {
        // GIVEN: A task with circuit breaker threshold = 3
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );
        task.max_retries = 10; // High retry limit to isolate circuit breaker

        // WHEN: Task fails with 3 consecutive timeouts
        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 1"));
        assert_eq!(task.consecutive_timeout_failures, 1);
        assert!(!task.circuit_breaker_tripped);

        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 2"));
        assert_eq!(task.consecutive_timeout_failures, 2);
        assert!(!task.circuit_breaker_tripped);

        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 3"));

        // THEN: Circuit breaker trips at 3rd consecutive timeout
        assert_eq!(task.consecutive_timeout_failures, 3);
        assert!(task.circuit_breaker_tripped);
        assert_eq!(task.status, TaskStatus::Failed);
    }

    #[test]
    fn test_progress_aware_timeout_does_not_advance_breaker() {
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );
        task.max_retries = 10;

        // Seed one no-progress timeout
        task.mark_failed_with_details(TaskFailureInfo::timeout("vision", "timeout hang"));
        assert_eq!(task.consecutive_timeout_failures, 1);

        // Progressing stall must NOT advance the breaker
        let progressing =
            TaskFailureInfo::timeout("vision", "Vision extraction stalled … [vision_progress=1]")
                .with_made_progress(true);
        task.mark_failed_with_details(progressing);
        assert_eq!(task.consecutive_timeout_failures, 1);
        assert!(!task.circuit_breaker_tripped);

        // from_processing_error must parse the marker (X-30 typed timeout prefix)
        let parsed = TaskFailureInfo::from_processing_error(
            "Operation timed out: Vision stalled [vision_progress=1]",
        );
        assert!(parsed.made_progress);
        assert!(parsed.is_timeout());
    }

    #[test]
    fn test_progress_aware_convert_stall_is_retryable() {
        // SPEC: Vision stall with progress must requeue (checkpoint resume), not
        // permanently fail on attempt 1/3 via retryable=false.
        let data = serde_json::json!({"pdf_id": "test"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::PdfProcessing,
            data,
        );
        task.max_retries = 3;

        let err = TaskFailureInfo::from_processing_error(
            "Timeout: Operation timed out: Vision extraction stalled: no progress for 300s \
             (stall limit 300s) for PDF abc. Provider 'mistral' may be hung. Progress during \
             this attempt is preserved for resume. [failure_class=timeout_phase_convert] \
             [vision_progress=1]",
        );
        assert!(err.is_timeout());
        assert!(err.made_progress);
        assert!(
            err.retryable,
            "progress stall must stay retryable until circuit breaker trips"
        );

        task.mark_failed_with_details(err);
        assert_eq!(task.retry_count, 1);
        assert_eq!(task.consecutive_timeout_failures, 0);
        assert!(!task.circuit_breaker_tripped);
        assert!(
            task.can_retry(),
            "first progress stall must requeue (1 < max_retries=3)"
        );
    }

    #[test]
    fn test_no_progress_timeouts_still_trip_breaker() {
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );
        task.max_retries = 10;

        for i in 1..=3 {
            let err = TaskFailureInfo::timeout("vision", format!("stall {i} [vision_progress=0]"));
            assert!(!err.made_progress);
            task.mark_failed_with_details(err);
        }
        assert!(task.circuit_breaker_tripped);
        assert!(!task.can_retry());
        assert!(
            task.error.as_ref().is_some_and(|e| !e.retryable),
            "breaker must flip retryable=false"
        );
    }

    #[test]
    fn test_can_retry_respects_circuit_breaker() {
        // GIVEN: A task with circuit breaker tripped
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );
        task.max_retries = 10; // High retry limit

        // Trigger circuit breaker
        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 1"));
        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 2"));
        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 3"));

        assert!(task.circuit_breaker_tripped);
        assert_eq!(task.retry_count, 3);

        // WHEN: Checking if task can retry
        let can_retry = task.can_retry();

        // THEN: Task cannot retry (circuit breaker prevents retry)
        assert!(!can_retry);
    }

    #[test]
    fn test_is_timeout_detection() {
        // X-30: typed — only the timeout() factory trips the breaker.
        let typed = TaskFailureInfo::timeout("extract", "LLM request timed out after 300s");
        assert!(typed.is_timeout());

        // Business text mentioning "timeout" must NOT classify as timeout.
        let business = TaskFailureInfo::new(
            "User asked about timeout policy",
            "extract",
            "business",
            "n/a",
            true,
        );
        assert!(!business.is_timeout());

        let non_timeout_cases = vec![
            "Connection refused",
            "Invalid API key",
            "Rate limit exceeded",
            "Server error 500",
            "Document parsing failed",
        ];

        for msg in non_timeout_cases {
            let failure = TaskFailureInfo::new(msg, "test", "error", "retry", true);
            assert!(
                !failure.is_timeout(),
                "Should NOT detect '{}' as timeout",
                msg
            );
        }
    }

    #[test]
    fn test_circuit_breaker_error_message_enhancement() {
        // GIVEN: A task approaching circuit breaker threshold
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );

        // WHEN: Task fails with 3rd consecutive timeout
        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 1"));
        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 2"));
        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 3"));

        // THEN: Error message enhanced with circuit breaker info
        let error = task.error.expect("Task should have structured error");
        assert!(
            error.message.contains("Circuit breaker"),
            "Error message should mention circuit breaker: {}",
            error.message
        );
        assert!(
            error.message.contains("3 consecutive timeout"),
            "Error message should mention consecutive timeouts: {}",
            error.message
        );
        assert!(!error.retryable, "Error should be marked as non-retryable");
    }

    #[test]
    fn test_mixed_failures_do_not_trip_circuit_breaker() {
        // GIVEN: A task with mixed failure types
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );
        task.max_retries = 15; // High retry limit

        // WHEN: Task fails with alternating timeout and network errors
        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 1"));
        assert_eq!(task.consecutive_timeout_failures, 1);

        task.mark_failed_with_details(TaskFailureInfo::new(
            "Network error",
            "network",
            "Connection refused",
            "retry",
            true,
        ));
        assert_eq!(task.consecutive_timeout_failures, 0); // Reset on non-timeout

        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 2"));
        assert_eq!(task.consecutive_timeout_failures, 1);

        task.mark_failed_with_details(TaskFailureInfo::new(
            "Rate limit exceeded",
            "llm",
            "Too many requests",
            "wait and retry",
            true,
        ));
        assert_eq!(task.consecutive_timeout_failures, 0); // Reset on non-timeout

        // THEN: Circuit breaker never trips (no 3 consecutive timeouts)
        assert!(!task.circuit_breaker_tripped);
        assert!(task.can_retry());
    }

    #[test]
    fn test_circuit_breaker_with_max_retries_exhausted() {
        // GIVEN: A task with low max_retries but circuit breaker threshold not reached
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );
        task.max_retries = 2; // Lower than circuit breaker threshold (3)

        // WHEN: Task fails 2 times with non-timeout errors
        task.mark_failed_with_details(TaskFailureInfo::new(
            "Error 1", "step1", "fail", "retry", true,
        ));
        task.mark_failed_with_details(TaskFailureInfo::new(
            "Error 2", "step2", "fail", "retry", true,
        ));

        // THEN: Task cannot retry (max_retries exhausted, not circuit breaker)
        assert_eq!(task.retry_count, 2);
        assert_eq!(task.consecutive_timeout_failures, 0);
        assert!(!task.circuit_breaker_tripped);
        assert!(!task.can_retry()); // max_retries exhausted
    }
}
