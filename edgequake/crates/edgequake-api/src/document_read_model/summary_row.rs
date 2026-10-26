//! Shared relational document projection for REST and scoped inventory.
use super::{normalize_relational_status, DocumentSummary};

pub(crate) fn relational_summary_from_row(
    row: sqlx::postgres::PgRow,
) -> crate::handlers::documents_types::DocumentSummary {
    use sqlx::Row;

    let title: String = row.get("title");
    let status: String = row.get("status");
    let chunk_count: Option<i32> = row.get("chunk_count");
    let entity_count: Option<i32> = row.get("entity_count");
    let content_length: Option<i64> = row.get("content_length");
    let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
    let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");
    let cost_usd: Option<f64> = row.get("cost_usd");
    let input_tokens: Option<i64> = row.get("input_tokens");
    let output_tokens: Option<i64> = row.get("output_tokens");
    let total_tokens: Option<i64> = row.get("total_tokens");
    let track_id: Option<String> = row.get("track_id");
    let current_stage: Option<String> = row.get("current_stage");
    let content_preview: Option<String> = row.get("content_preview");
    let content_summary = content_preview.filter(|s| !s.is_empty());

    DocumentSummary {
        id: row.get("id"),
        title: Some(title.clone()),
        file_name: Some(title),
        content_summary,
        content_length: content_length
            .filter(|&n| n >= 0)
            .and_then(|n| usize::try_from(n).ok()),
        chunk_count: chunk_count.unwrap_or(0).max(0) as usize,
        entity_count: entity_count.map(|n| n.max(0) as usize),
        status: Some(normalize_relational_status(&status)),
        error_message: row.get("error_message"),
        warning_message: None,
        track_id,
        created_at: Some(created_at.to_rfc3339()),
        updated_at: Some(updated_at.to_rfc3339()),
        cost_usd,
        input_tokens: input_tokens.map(|n| n as usize),
        output_tokens: output_tokens.map(|n| n as usize),
        total_tokens: total_tokens.map(|n| n as usize),
        llm_model: None,
        embedding_model: None,
        source_type: None,
        current_stage,
        stage_progress: None,
        stage_message: None,
        pdf_id: None,
        display_status: None,
        ui_phase: None,
        progress_counts: None,
        run_progress: None,
        queue_position: None,
        eta_seconds: None,
        eta_basis: None,
        query_ready: None,
        cancelled_from_stage: None,
        extraction: Default::default(),

        classification: None,
        compartments: None,
    }
}
