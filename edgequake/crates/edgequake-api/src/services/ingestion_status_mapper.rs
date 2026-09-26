//! Ingestion status presentation SSOT (SPEC-057 P4 / P0.3b).
//!
//! Maps task + doc KV + PDF status (+ cancel intent) → one DTO for API/UI badges.
//! Write-path cancel helpers stay in [`super::ingestion_status`]; phase↔unified
//! slug tables stay in `edgequake_pipeline::stage_bridge`.

use edgequake_tasks::is_cancel_failure_message;

use crate::document_metadata::{
    is_terminal_document_status, is_terminal_failure_status, is_terminal_success_status,
};
use crate::handlers::documents_types::DocumentSummary;

/// Inputs for status projection (all optional string slices).
#[derive(Debug, Clone, Default)]
pub struct IngestionStatusInputs<'a> {
    pub task_status: Option<&'a str>,
    pub doc_status: Option<&'a str>,
    pub current_stage: Option<&'a str>,
    pub failure_class: Option<&'a str>,
    pub pdf_status: Option<&'a str>,
    /// Error / stage message used to detect cancel-classified failures.
    pub error_message: Option<&'a str>,
    pub stage_message: Option<&'a str>,
    pub cancel_intent: bool,
}

/// API/UI presentation view (SSOT for badges).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestionStatusView {
    pub display_status: String,
    pub ui_phase: String,
    pub is_terminal: bool,
    pub is_failure: bool,
    pub is_cancelled: bool,
    pub failure_class: Option<String>,
    pub stage_message: Option<String>,
}

/// Map legacy write-path status names → unified `current_stage` slugs (SPEC-002).
///
/// Absorbed from `status_updates::update_document_status` (DRY).
pub fn legacy_status_to_unified_stage(status: &str) -> &str {
    match status {
        "pending" => "uploading",
        "processing" => "preprocessing",
        "chunking" => "chunking",
        "extracting" => "extracting",
        "embedding" => "embedding",
        "indexing" => "storing",
        "projecting" => "projecting",
        "completed" | "indexed" => "completed",
        "failed" => "failed",
        "partial_failure" => "partial_failure",
        "cancelled" => "cancelled",
        "re_embedding" => "re_embedding",
        other => other,
    }
}

/// Default human-readable stage message for a write-path status.
pub fn default_stage_message_for_status(status: &str) -> &'static str {
    match status {
        "pending" => "Document queued for processing",
        "processing" | "preprocessing" => "Preprocessing document...",
        "chunking" => "Splitting document into chunks...",
        "extracting" => "Extracting entities and relationships...",
        "embedding" | "re_embedding" => "Generating vector embeddings...",
        "indexing" | "storing" => "Storing in knowledge graph...",
        "projecting" => "Applying projections to graph and vectors...",
        "completed" | "indexed" => "Processing complete",
        "failed" => "Processing failed",
        "partial_failure" => "Processing completed with issues",
        "cancelled" => "Processing cancelled",
        "converting" => "Converting document...",
        _ => "Processing...",
    }
}

fn norm(s: Option<&str>) -> Option<&str> {
    s.map(str::trim).filter(|v| !v.is_empty())
}

fn eq_ci(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn is_cancelled_truth(inputs: &IngestionStatusInputs<'_>) -> bool {
    let checks = [
        inputs.task_status,
        inputs.doc_status,
        inputs.failure_class,
        inputs.pdf_status,
        inputs.current_stage,
    ];
    if checks.into_iter().flatten().any(|s| eq_ci(s, "cancelled")) {
        return true;
    }
    if let Some(msg) = inputs.error_message {
        if is_cancel_failure_message(msg) {
            return true;
        }
    }
    if let Some(msg) = inputs.stage_message {
        if is_cancel_failure_message(msg) {
            return true;
        }
    }
    false
}

fn success_display(status: &str) -> &str {
    if eq_ci(status, "indexed") {
        "indexed"
    } else if eq_ci(status, "partial_success") {
        "partial_success"
    } else {
        "completed"
    }
}

fn failure_display(status: &str) -> &str {
    if eq_ci(status, "partial_failure") {
        "partial_failure"
    } else {
        "failed"
    }
}

/// Project task/doc/PDF signals into a single presentation DTO.
pub fn map_ingestion_status(inputs: IngestionStatusInputs<'_>) -> IngestionStatusView {
    let stage_message = norm(inputs.stage_message).map(str::to_string);

    if is_cancelled_truth(&inputs) {
        return IngestionStatusView {
            display_status: "cancelled".into(),
            ui_phase: "terminal".into(),
            is_terminal: true,
            is_failure: false,
            is_cancelled: true,
            failure_class: Some(
                norm(inputs.failure_class)
                    .unwrap_or("cancelled")
                    .to_string(),
            ),
            stage_message,
        };
    }

    let doc = norm(inputs.doc_status);
    let task = norm(inputs.task_status);
    let stage = norm(inputs.current_stage);

    // True when the document row still reports in-flight work.
    // PDF convert tasks finish as `indexed` before the follow-on Insert starts
    // (SPEC-057 P2); while doc/stage remain non-terminal, the document row is
    // the SSOT — never let a completed convert task paint the badge Completed.
    let doc_in_flight = match stage {
        Some(s) => !is_terminal_document_status(legacy_status_to_unified_stage(s)),
        None => doc.is_some_and(|s| !is_terminal_document_status(s)),
    };

    // Terminal success (doc preferred, then task indexed — only when doc is idle).
    if let Some(s) = doc.filter(|s| is_terminal_success_status(s)) {
        return IngestionStatusView {
            display_status: success_display(s).into(),
            ui_phase: "terminal".into(),
            is_terminal: true,
            is_failure: false,
            is_cancelled: false,
            failure_class: None,
            stage_message,
        };
    }
    if !doc_in_flight {
        if let Some(s) = task.filter(|s| eq_ci(s, "indexed") || is_terminal_success_status(s)) {
            return IngestionStatusView {
                display_status: success_display(s).into(),
                ui_phase: "terminal".into(),
                is_terminal: true,
                is_failure: false,
                is_cancelled: false,
                failure_class: None,
                stage_message,
            };
        }
    }

    // Terminal failure (never cancel — already handled).
    if let Some(s) = doc.filter(|s| is_terminal_failure_status(s) && !eq_ci(s, "cancelled")) {
        return IngestionStatusView {
            display_status: failure_display(s).into(),
            ui_phase: "terminal".into(),
            is_terminal: true,
            is_failure: true,
            is_cancelled: false,
            failure_class: norm(inputs.failure_class).map(str::to_string),
            stage_message,
        };
    }
    if let Some(s) = task.filter(|s| eq_ci(s, "failed")) {
        return IngestionStatusView {
            display_status: failure_display(s).into(),
            ui_phase: "terminal".into(),
            is_terminal: true,
            is_failure: true,
            is_cancelled: false,
            failure_class: norm(inputs.failure_class).map(str::to_string),
            stage_message,
        };
    }

    // In-flight: prefer fine-grained stage; PDF Completed does not win over doc stage.
    let display_owned = stage
        .map(legacy_status_to_unified_stage)
        .or_else(|| doc.map(legacy_status_to_unified_stage))
        .map(str::to_string)
        .or_else(|| {
            task.map(|t| match t.to_ascii_lowercase().as_str() {
                "pending" => "pending".to_string(),
                "processing" => "processing".to_string(),
                other => other.to_string(),
            })
        })
        .unwrap_or_else(|| "pending".to_string());
    let terminal = is_terminal_document_status(&display_owned);
    let ui_phase = if inputs.cancel_intent && !terminal {
        "stopping"
    } else if terminal {
        "terminal"
    } else if matches!(display_owned.as_str(), "pending" | "queued" | "uploading") {
        "idle"
    } else {
        "running"
    };

    IngestionStatusView {
        display_status: display_owned,
        ui_phase: ui_phase.into(),
        is_terminal: terminal,
        is_failure: false,
        is_cancelled: false,
        failure_class: None,
        stage_message,
    }
}

/// Build mapper inputs from document summary fields (list/detail enrichment).
pub fn inputs_from_document_summary<'a>(
    summary: &'a DocumentSummary,
    failure_class: Option<&'a str>,
    pdf_status: Option<&'a str>,
    cancel_intent: bool,
) -> IngestionStatusInputs<'a> {
    inputs_from_document_summary_with_task(summary, failure_class, pdf_status, cancel_intent, None)
}

/// Build mapper inputs including durable task row status (cancel lag honesty).
pub fn inputs_from_document_summary_with_task<'a>(
    summary: &'a DocumentSummary,
    failure_class: Option<&'a str>,
    pdf_status: Option<&'a str>,
    cancel_intent: bool,
    task_status: Option<&'a str>,
) -> IngestionStatusInputs<'a> {
    IngestionStatusInputs {
        task_status,
        doc_status: summary.status.as_deref(),
        current_stage: summary.current_stage.as_deref(),
        failure_class,
        pdf_status,
        error_message: summary.error_message.as_deref(),
        stage_message: summary.stage_message.as_deref(),
        cancel_intent,
    }
}

/// Fill `display_status` / `ui_phase` on a summary (idempotent).
pub fn enrich_document_summary_status(
    summary: &mut DocumentSummary,
    failure_class: Option<&str>,
    pdf_status: Option<&str>,
    cancel_intent: bool,
) {
    enrich_document_summary_status_with_task(
        summary,
        failure_class,
        pdf_status,
        cancel_intent,
        None,
    );
}

/// Fill presentation fields with optional task-row status (dual-SSOT lag).
pub fn enrich_document_summary_status_with_task(
    summary: &mut DocumentSummary,
    failure_class: Option<&str>,
    pdf_status: Option<&str>,
    cancel_intent: bool,
    task_status: Option<&str>,
) {
    let view = map_ingestion_status(inputs_from_document_summary_with_task(
        summary,
        failure_class,
        pdf_status,
        cancel_intent,
        task_status,
    ));
    summary.display_status = Some(view.display_status);
    summary.ui_phase = Some(view.ui_phase);
}

/// Enrich a batch after list construction (cancel_intent unknown → false).
pub fn enrich_document_summaries(summaries: &mut [DocumentSummary]) {
    for summary in summaries.iter_mut() {
        enrich_document_summary_status(summary, None, None, false);
    }
}

/// Enrich summaries with cancel-intent + durable task status (list/track SSOT).
pub async fn enrich_document_summaries_with_cancel(
    summaries: &mut [DocumentSummary],
    registry: &edgequake_tasks::CancellationRegistry,
    task_storage: &dyn edgequake_tasks::storage::TaskStorage,
) {
    let track_ids: Vec<String> = summaries
        .iter()
        .filter_map(|s| s.track_id.clone())
        .collect();
    let tasks = task_storage
        .get_tasks_by_track_ids(&track_ids)
        .await
        .unwrap_or_default();

    for summary in summaries.iter_mut() {
        let cancel_intent = match summary.track_id.as_deref() {
            Some(track_id) => registry.has_cancel_intent(track_id).await,
            None => false,
        };
        let task_status_owned = summary
            .track_id
            .as_ref()
            .and_then(|id| tasks.get(id))
            .map(|t| t.status.to_string());
        enrich_document_summary_status_with_task(
            summary,
            None,
            None,
            cancel_intent,
            task_status_owned.as_deref(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        name: &'static str,
        inputs: IngestionStatusInputs<'static>,
        display: &'static str,
        ui_phase: &'static str,
        cancelled: bool,
        terminal: bool,
    }

    fn fixtures() -> Vec<Fixture> {
        vec![
            Fixture {
                name: "doc_cancelled",
                inputs: IngestionStatusInputs {
                    doc_status: Some("cancelled"),
                    current_stage: Some("extracting"),
                    failure_class: Some("cancelled"),
                    ..Default::default()
                },
                display: "cancelled",
                ui_phase: "terminal",
                cancelled: true,
                terminal: true,
            },
            Fixture {
                name: "task_cancelled",
                inputs: IngestionStatusInputs {
                    task_status: Some("cancelled"),
                    doc_status: Some("processing"),
                    current_stage: Some("converting"),
                    ..Default::default()
                },
                display: "cancelled",
                ui_phase: "terminal",
                cancelled: true,
                terminal: true,
            },
            Fixture {
                name: "pdf_cancelled",
                inputs: IngestionStatusInputs {
                    pdf_status: Some("cancelled"),
                    doc_status: Some("processing"),
                    current_stage: Some("converting"),
                    ..Default::default()
                },
                display: "cancelled",
                ui_phase: "terminal",
                cancelled: true,
                terminal: true,
            },
            Fixture {
                name: "cancel_message_not_failed",
                inputs: IngestionStatusInputs {
                    doc_status: Some("failed"),
                    error_message: Some("Task cancelled by user"),
                    failure_class: Some("cancelled"),
                    ..Default::default()
                },
                display: "cancelled",
                ui_phase: "terminal",
                cancelled: true,
                terminal: true,
            },
            Fixture {
                name: "failed",
                inputs: IngestionStatusInputs {
                    doc_status: Some("failed"),
                    failure_class: Some("permanent"),
                    ..Default::default()
                },
                display: "failed",
                ui_phase: "terminal",
                cancelled: false,
                terminal: true,
            },
            Fixture {
                name: "completed",
                inputs: IngestionStatusInputs {
                    doc_status: Some("completed"),
                    current_stage: Some("extracting"),
                    ..Default::default()
                },
                display: "completed",
                ui_phase: "terminal",
                cancelled: false,
                terminal: true,
            },
            Fixture {
                // Completed convert/`indexed` task must not win over in-flight doc.
                name: "indexed_task_doc_inflight",
                inputs: IngestionStatusInputs {
                    task_status: Some("indexed"),
                    doc_status: Some("processing"),
                    current_stage: Some("extracting"),
                    ..Default::default()
                },
                display: "extracting",
                ui_phase: "running",
                cancelled: false,
                terminal: false,
            },
            Fixture {
                // No doc signal — indexed task is honest terminal success.
                name: "indexed_task_no_doc",
                inputs: IngestionStatusInputs {
                    task_status: Some("indexed"),
                    ..Default::default()
                },
                display: "indexed",
                ui_phase: "terminal",
                cancelled: false,
                terminal: true,
            },
            Fixture {
                // PDF convert follow-on: convert task indexed, ingest still queued.
                name: "pdf_convert_follow_on_pending_ingest",
                inputs: IngestionStatusInputs {
                    task_status: Some("indexed"),
                    doc_status: Some("processing"),
                    current_stage: Some("preprocessing"),
                    stage_message: Some("PDF converted — queueing knowledge-graph ingest"),
                    ..Default::default()
                },
                display: "preprocessing",
                ui_phase: "running",
                cancelled: false,
                terminal: false,
            },
            Fixture {
                name: "converting",
                inputs: IngestionStatusInputs {
                    doc_status: Some("processing"),
                    current_stage: Some("converting"),
                    ..Default::default()
                },
                display: "converting",
                ui_phase: "running",
                cancelled: false,
                terminal: false,
            },
            Fixture {
                name: "extracting",
                inputs: IngestionStatusInputs {
                    doc_status: Some("processing"),
                    current_stage: Some("extracting"),
                    ..Default::default()
                },
                display: "extracting",
                ui_phase: "running",
                cancelled: false,
                terminal: false,
            },
            Fixture {
                name: "re_embedding",
                inputs: IngestionStatusInputs {
                    doc_status: Some("processing"),
                    current_stage: Some("re_embedding"),
                    ..Default::default()
                },
                display: "re_embedding",
                ui_phase: "running",
                cancelled: false,
                terminal: false,
            },
            Fixture {
                name: "stopping_intent",
                inputs: IngestionStatusInputs {
                    doc_status: Some("processing"),
                    current_stage: Some("extracting"),
                    cancel_intent: true,
                    ..Default::default()
                },
                display: "extracting",
                ui_phase: "stopping",
                cancelled: false,
                terminal: false,
            },
            Fixture {
                name: "pdf_completed_doc_extracting",
                inputs: IngestionStatusInputs {
                    pdf_status: Some("completed"),
                    doc_status: Some("processing"),
                    current_stage: Some("extracting"),
                    ..Default::default()
                },
                display: "extracting",
                ui_phase: "running",
                cancelled: false,
                terminal: false,
            },
            Fixture {
                name: "pending_idle",
                inputs: IngestionStatusInputs {
                    doc_status: Some("pending"),
                    ..Default::default()
                },
                display: "uploading",
                ui_phase: "idle",
                cancelled: false,
                terminal: false,
            },
        ]
    }

    #[test]
    fn fixture_matrix_covers_badge_outputs() {
        let rows = fixtures();
        assert!(rows.len() >= 12, "DoD requires ≥12 fixture rows");
        for row in rows {
            let view = map_ingestion_status(row.inputs.clone());
            assert_eq!(
                view.display_status, row.display,
                "{}: display_status",
                row.name
            );
            assert_eq!(view.ui_phase, row.ui_phase, "{}: ui_phase", row.name);
            assert_eq!(
                view.is_cancelled, row.cancelled,
                "{}: is_cancelled",
                row.name
            );
            assert_eq!(view.is_terminal, row.terminal, "{}: is_terminal", row.name);
        }
    }

    #[test]
    fn legacy_status_mapping_matches_spec002() {
        assert_eq!(legacy_status_to_unified_stage("indexing"), "storing");
        assert_eq!(legacy_status_to_unified_stage("pending"), "uploading");
        assert_eq!(
            legacy_status_to_unified_stage("re_embedding"),
            "re_embedding"
        );
    }
}
