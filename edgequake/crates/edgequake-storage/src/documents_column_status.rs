//! Map KV / pipeline status strings onto `documents_valid_status` (migration 141).
//!
//! Richer stage vocabulary stays in `metadata->>'status'`; the column must only
//! hold CHECK-allowlisted values or relational INSERT/UPDATE fails closed.
//!
//! SPEC-129: all dual-write touch/stats/sidecar paths must use
//! [`relational_documents_status_for_write`] — never pass raw KV stages through.

/// Map KV / pipeline status strings onto `documents_valid_status` (migration 141).
///
/// SPEC-098 LAW-098-11: lifecycle statuses `deleting` / `delete_failed` pass
/// through unchanged — never collapse to cancelled/failed.
pub fn normalize_documents_column_status(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return "processing".to_string();
    }
    let lower = s.to_ascii_lowercase();
    match lower.as_str() {
        "pending" | "processing" | "chunking" | "extracting" | "embedding" | "indexing"
        | "completed" | "indexed" | "failed" | "partial_failure" | "cancelled" | "deleting"
        | "delete_failed" => lower,
        "queued" => "pending".to_string(),
        "partial_success" => "partial_failure".to_string(),
        // Pipeline stage slugs and anything else → generic processing.
        // `projecting` is post-commit SPEC-149 replay — column CHECK has no projecting.
        "uploading" | "converting" | "preprocessing" | "gleaning" | "merging" | "summarizing"
        | "storing" | "re_embedding" | "projecting" => "processing".to_string(),
        _ => "processing".to_string(),
    }
}

/// CHECK-safe column status for relational dual-writes (touch + stats + sidecar).
///
/// Applies [`normalize_documents_column_status`], then maps `completed` → `indexed`
/// (KV uses `completed`; relational CHECK + Documents list prefer `indexed`).
///
/// SPEC-129 / #381: closes the gap where `touch_document_status("re_embedding")`
/// bypassed normalization and violated `documents_valid_status`.
pub fn relational_documents_status_for_write(raw: &str) -> String {
    let normalized = normalize_documents_column_status(raw);
    if normalized == "completed" {
        "indexed".to_string()
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_documents_column_status_maps_kv_vocabulary() {
        assert_eq!(normalize_documents_column_status("queued"), "pending");
        // SPEC-098 LAW-098-11: lifecycle pass-through.
        assert_eq!(normalize_documents_column_status("deleting"), "deleting");
        assert_eq!(
            normalize_documents_column_status("delete_failed"),
            "delete_failed"
        );
        assert_eq!(
            normalize_documents_column_status("partial_success"),
            "partial_failure"
        );
        assert_eq!(
            normalize_documents_column_status("converting"),
            "processing"
        );
        assert_eq!(normalize_documents_column_status("cancelled"), "cancelled");
        assert_eq!(
            normalize_documents_column_status("extracting"),
            "extracting"
        );
        assert_eq!(normalize_documents_column_status(""), "processing");
        // SPEC-129 / #381
        assert_eq!(
            normalize_documents_column_status("re_embedding"),
            "processing"
        );
        assert_eq!(
            normalize_documents_column_status("Re_Embedding"),
            "processing"
        );
        assert_eq!(
            normalize_documents_column_status("projecting"),
            "processing"
        );
    }

    #[test]
    fn relational_documents_status_for_write_projects_check_safe() {
        assert_eq!(
            relational_documents_status_for_write("re_embedding"),
            "processing"
        );
        assert_eq!(
            relational_documents_status_for_write("projecting"),
            "processing"
        );
        assert_eq!(
            relational_documents_status_for_write("completed"),
            "indexed"
        );
        assert_eq!(relational_documents_status_for_write("indexed"), "indexed");
        assert_eq!(relational_documents_status_for_write("queued"), "pending");
        assert_eq!(
            relational_documents_status_for_write("merging"),
            "processing"
        );
        assert_eq!(
            relational_documents_status_for_write("deleting"),
            "deleting"
        );
        assert_eq!(
            relational_documents_status_for_write("extracting"),
            "extracting"
        );
        assert_eq!(relational_documents_status_for_write(""), "processing");
    }
}
