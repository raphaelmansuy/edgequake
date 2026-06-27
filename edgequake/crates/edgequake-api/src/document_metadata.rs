//! Document metadata field normalization for API responses and KV writes.
//!
//! WHY: `error_message` must only represent terminal failures. Non-fatal pipeline
//! notices (vision fallback, low-content warnings) belong in `warning_message`.

/// Statuses where `error_message` is semantically valid.
pub fn is_terminal_failure_status(status: &str) -> bool {
    matches!(
        status.to_lowercase().as_str(),
        "failed" | "partial_failure" | "cancelled"
    )
}

/// Statuses indicating the document is still moving through the pipeline.
pub fn is_active_processing_status(status: &str) -> bool {
    matches!(
        status.to_lowercase().as_str(),
        "pending"
            | "processing"
            | "chunking"
            | "extracting"
            | "embedding"
            | "indexing"
            | "uploading"
            | "converting"
            | "preprocessing"
            | "gleaning"
            | "merging"
            | "summarizing"
            | "storing"
    )
}

/// Prefer the fresher of two metadata snapshots when KV returns duplicates
/// (e.g. staging + legacy final shell during in-flight ingestion).
pub fn should_prefer_incoming_document_metadata(
    existing_updated_at: Option<&str>,
    existing_status: Option<&str>,
    existing_stage: Option<&str>,
    incoming_updated_at: Option<&str>,
    incoming_status: Option<&str>,
    incoming_stage: Option<&str>,
) -> bool {
    fn parse_ts(value: Option<&str>) -> Option<i64> {
        value
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.timestamp())
    }

    fn is_stale_upload_shell(status: Option<&str>, stage: Option<&str>) -> bool {
        status.map(str::to_lowercase).as_deref() == Some("pending")
            && stage.map(str::to_lowercase).as_deref() == Some("uploading")
    }

    let existing_ts = parse_ts(existing_updated_at).unwrap_or(0);
    let incoming_ts = parse_ts(incoming_updated_at).unwrap_or(0);
    if incoming_ts != existing_ts {
        return incoming_ts > existing_ts;
    }

    let existing_stale = is_stale_upload_shell(existing_status, existing_stage);
    let incoming_stale = is_stale_upload_shell(incoming_status, incoming_stage);
    if existing_stale != incoming_stale {
        return !incoming_stale;
    }

    true
}

/// Heuristic for legacy rows that stored informational notices in `error_message`.
pub fn is_informational_notice(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("falling back")
        || lower.contains("fallback")
        || lower.contains("unavailable")
        || lower.contains("low text content")
        || lower.contains("may be image-only")
        || lower.contains("consider using vision")
        || lower.contains("auto-recovered")
}

/// Split legacy `error_message` into terminal error vs non-fatal warning for API output.
pub fn normalize_notice_fields(
    status: Option<&str>,
    error_message: Option<String>,
    warning_message: Option<String>,
) -> (Option<String>, Option<String>) {
    let status_str = status.unwrap_or("pending");
    let mut warning = warning_message;
    let mut error = error_message;

    if let Some(msg) = error.clone() {
        if is_terminal_failure_status(status_str) {
            return (error, warning);
        }

        if is_active_processing_status(status_str)
            || status_str == "completed"
            || status_str == "indexed"
        {
            if is_informational_notice(&msg) {
                if warning.is_none() {
                    warning = Some(msg);
                }
                error = None;
            } else if !is_terminal_failure_status(status_str) {
                // Ambiguous legacy notice during processing — treat as warning, not failure.
                if warning.is_none() {
                    warning = Some(msg);
                }
                error = None;
            }
        }
    }

    (error, warning)
}

/// Read and normalize notice fields from KV metadata JSON for API responses.
pub fn extract_notices_from_metadata(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> (Option<String>, Option<String>) {
    let status = obj.get("status").and_then(|v| v.as_str());
    let error = obj
        .get("error_message")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let warning = obj
        .get("warning_message")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    normalize_notice_fields(status, error, warning)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_failure_keeps_error_message() {
        let (err, warn) = normalize_notice_fields(
            Some("failed"),
            Some("Pipeline processing failed".into()),
            None,
        );
        assert_eq!(err.as_deref(), Some("Pipeline processing failed"));
        assert!(warn.is_none());
    }

    #[test]
    fn processing_moves_legacy_fallback_to_warning() {
        let (err, warn) = normalize_notice_fields(
            Some("processing"),
            Some("Vision unavailable. Falling back to EdgeParse.".into()),
            None,
        );
        assert!(err.is_none());
        assert!(warn.is_some());
    }

    #[test]
    fn completed_clears_stale_processing_error() {
        let (err, warn) = normalize_notice_fields(
            Some("completed"),
            Some("Vision unavailable. Falling back to EdgeParse.".into()),
            None,
        );
        assert!(err.is_none());
        assert!(warn.is_some());
    }

    #[test]
    fn explicit_warning_preserved() {
        let (err, warn) =
            normalize_notice_fields(Some("chunking"), None, Some("Using EdgeParse".into()));
        assert!(err.is_none());
        assert_eq!(warn.as_deref(), Some("Using EdgeParse"));
    }

    #[test]
    fn prefers_fresher_metadata_over_stale_upload_shell() {
        assert!(should_prefer_incoming_document_metadata(
            Some("2026-06-27T10:00:00+00:00"),
            Some("pending"),
            Some("uploading"),
            Some("2026-06-27T10:00:01+00:00"),
            Some("chunking"),
            Some("chunking"),
        ));
        assert!(!should_prefer_incoming_document_metadata(
            Some("2026-06-27T10:00:01+00:00"),
            Some("chunking"),
            Some("chunking"),
            Some("2026-06-27T10:00:00+00:00"),
            Some("pending"),
            Some("uploading"),
        ));
    }
}
