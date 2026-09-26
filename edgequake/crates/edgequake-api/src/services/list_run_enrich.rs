//! SPEC-091 IS2/IS3 — enrich document list page with queue ETA + serving readiness.
//!
//! WHY: ActiveRuns reads the documents list. Queue position/ETA (LAW-Q4 / LAW-IS4)
//! and serving-fence queryability (LD-09) must be projections on that surface,
//! not a second poll product.

use edgequake_tasks::{estimate_queues_batch, TaskStorage};

use crate::handlers::documents_types::DocumentSummary;

/// True when the row should show queue chrome (pending admission / fairness wait).
pub fn needs_queue_estimate(doc: &DocumentSummary) -> bool {
    let stage = doc
        .current_stage
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    let status = doc.status.as_deref().unwrap_or("").to_ascii_lowercase();
    matches!(
        stage.as_str(),
        "queued" | "pending" | "cleaning" | "uploading"
    ) || matches!(status.as_str(), "pending" | "queued")
}

/// Attach `queue_position` / `eta_seconds` / `eta_basis` for pending tracks on this page.
///
/// SPEC-091 IP0 / IP-AC-02: one batched rank fetch + one drain-rate sample (≤2 RTs),
/// not per-doc `get_task` + `estimate_queue`.
pub async fn enrich_page_queue_estimates(
    storage: &dyn TaskStorage,
    documents: &mut [DocumentSummary],
) {
    let track_ids: Vec<String> = documents
        .iter()
        .filter(|d| needs_queue_estimate(d))
        .filter_map(|d| d.track_id.clone())
        .collect();
    if track_ids.is_empty() {
        return;
    }
    let Ok(estimates) = estimate_queues_batch(storage, &track_ids).await else {
        return;
    };
    for doc in documents.iter_mut() {
        let Some(track_id) = doc.track_id.as_deref() else {
            continue;
        };
        let Some(estimate) = estimates.get(track_id) else {
            continue;
        };
        // Position is 1-based for UI ("#3 in queue"); estimate.position is
        // "tasks ahead" (0 = next). LAW-Q4 FCFS: display position = ahead + 1.
        doc.queue_position = Some(estimate.position.saturating_add(1));
        doc.eta_seconds = Some(estimate.eta_seconds);
        doc.eta_basis = Some(estimate.basis.as_str().to_string());
    }
}

/// When serving fence is on, set `query_ready` for terminal completed/indexed rows.
///
/// Fence off → leave `query_ready` unset (UI hides the badge).
/// Fence on + no chunks → ready (nothing to filter).
/// Fence on + chunks → ready iff every chunk is `ready` in `chunk_serving_state`.
#[cfg(feature = "postgres")]
pub async fn enrich_page_query_ready(
    pool: &sqlx::PgPool,
    fence_enabled: bool,
    documents: &mut [DocumentSummary],
) {
    if !fence_enabled {
        for doc in documents.iter_mut() {
            doc.query_ready = None;
        }
        return;
    }

    let mut ids: Vec<uuid::Uuid> = Vec::new();
    for doc in documents.iter() {
        let status = doc.status.as_deref().unwrap_or("").to_ascii_lowercase();
        if !matches!(status.as_str(), "completed" | "indexed") {
            continue;
        }
        if let Ok(u) = uuid::Uuid::parse_str(&doc.id) {
            ids.push(u);
        }
    }
    if ids.is_empty() {
        return;
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        document_id: uuid::Uuid,
        all_ready: bool,
        chunk_n: i64,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT
            c.document_id,
            COALESCE(
                bool_and(COALESCE(css.state, 'declared') = 'ready'),
                true
            ) AS all_ready,
            COUNT(*)::bigint AS chunk_n
        FROM public.chunks c
        LEFT JOIN public.chunk_serving_state css ON css.chunk_id = c.id
        WHERE c.document_id = ANY($1)
        GROUP BY c.document_id
        "#,
    )
    .bind(&ids)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut map = std::collections::HashMap::new();
    for row in rows {
        map.insert(row.document_id.to_string(), (row.all_ready, row.chunk_n));
    }

    for doc in documents.iter_mut() {
        let status = doc.status.as_deref().unwrap_or("").to_ascii_lowercase();
        if !matches!(status.as_str(), "completed" | "indexed") {
            continue;
        }
        match map.get(&doc.id) {
            Some((all_ready, n)) if *n > 0 => doc.query_ready = Some(*all_ready),
            // No chunk rows — treat as ready (nothing filtered) when fence on.
            _ => doc.query_ready = Some(true),
        }
    }
}

/// Promote stuck `projecting` rows on the visible page when deliveries are applied.
///
/// WHY: Durable commit leaves `projecting` until SPEC-149 replay settles. The
/// periodic reconciler eventually heals, but list polls should close ActiveRuns
/// as soon as deliveries are applied without waiting for the next reconcile tick.
#[cfg(feature = "postgres")]
pub async fn enrich_page_projecting_promote(
    kv: &std::sync::Arc<dyn edgequake_storage::traits::KVStorage>,
    pool: &sqlx::PgPool,
    documents: &mut [DocumentSummary],
) {
    for doc in documents.iter_mut() {
        let status = doc.status.as_deref().unwrap_or("").to_ascii_lowercase();
        let stage = doc
            .current_stage
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase();
        if status != "projecting" && stage != "projecting" {
            continue;
        }
        match crate::services::sync_doc_projecting_when_applied(
            std::sync::Arc::clone(kv),
            Some(pool),
            &doc.id,
        )
        .await
        {
            Ok(true) => {
                doc.status = Some("completed".into());
                doc.current_stage = Some("completed".into());
                doc.error_message = None;
                doc.stage_progress = Some(1.0);
                if doc.stage_message.as_deref().unwrap_or("").is_empty() {
                    doc.stage_message = Some("Processing complete".into());
                }
            }
            Ok(false) => {}
            Err(e) => {
                tracing::debug!(
                    document_id = %doc.id,
                    error = %e,
                    "SPEC-149: list projecting promote skipped"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(stage: &str, status: &str) -> DocumentSummary {
        DocumentSummary {
            id: "d1".into(),
            title: None,
            file_name: None,
            content_summary: None,
            content_length: None,
            chunk_count: 0,
            entity_count: None,
            status: Some(status.into()),
            error_message: None,
            warning_message: None,
            track_id: Some("t1".into()),
            created_at: None,
            updated_at: None,
            cost_usd: None,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            llm_model: None,
            embedding_model: None,
            source_type: None,
            current_stage: Some(stage.into()),
            stage_progress: None,
            stage_message: None,
            pdf_id: None,
            display_status: None,
            ui_phase: None,
            progress_counts: None,
            queue_position: None,
            eta_seconds: None,
            eta_basis: None,
            query_ready: None,
            cancelled_from_stage: None,
        }
    }

    #[test]
    fn queue_estimate_needed_for_admission_stages() {
        assert!(needs_queue_estimate(&summary("queued", "pending")));
        assert!(needs_queue_estimate(&summary("cleaning", "processing")));
        assert!(!needs_queue_estimate(&summary("extracting", "processing")));
        assert!(!needs_queue_estimate(&summary("completed", "completed")));
    }
}
