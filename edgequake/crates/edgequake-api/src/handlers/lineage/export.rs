//! Document lineage export handlers.

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;

use super::cache::cached_kv_get;
use crate::error::ApiError;
use crate::handlers::isolation::verify_document_access;
use crate::middleware::TenantContext;
use crate::state::AppState;

/// Query parameters for lineage export.
#[derive(Debug, serde::Deserialize, serde::Serialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct ExportParams {
    /// Export format: "json" (default) or "csv".
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "json".to_string()
}

/// Export complete document lineage as JSON or CSV file.
///
/// OODA-22: Returns lineage data as a downloadable file with proper
/// Content-Disposition headers. CSV format flattens the hierarchical
/// lineage into a table with one row per chunk.
///
/// @implements F5: Single API call retrieves complete document lineage tree
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/lineage/export",
    tag = "Lineage",
    params(
        ("document_id" = String, Path, description = "Document ID to export lineage for"),
        ExportParams,
    ),
    responses(
        (status = 200, description = "Lineage export file (JSON or CSV)"),
        (status = 404, description = "Document or lineage not found")
    )
)]
pub async fn export_document_lineage(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(document_id): Path<String>,
    Query(params): Query<ExportParams>,
) -> Result<impl IntoResponse, ApiError> {
    // SECURITY: Verify the document belongs to the requesting tenant/workspace.
    verify_document_access(state.kv_storage.as_ref(), &document_id, &tenant_ctx).await?;

    // OODA-23: Use cached KV lookup for export
    let lineage_key = format!("{}-lineage", document_id);
    let lineage_data = cached_kv_get(state.kv_storage.as_ref(), &lineage_key)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Lineage for document '{}' not found. \
                 Document may not have been processed yet.",
                document_id
            ))
        })?;

    // Read metadata for context (cached)
    let metadata_key = format!("{}-metadata", document_id);
    let metadata = cached_kv_get(state.kv_storage.as_ref(), &metadata_key)
        .await?
        .unwrap_or(serde_json::json!({"id": document_id}));

    let combined = serde_json::json!({
        "document_id": document_id,
        "metadata": metadata,
        "lineage": lineage_data,
    });

    match params.format.as_str() {
        "csv" => {
            // WHY: CSV flattens hierarchical lineage into a chunk-per-row table.
            // This is useful for spreadsheet analysis and data pipeline ingestion.
            let csv_content = lineage_to_csv(&document_id, &lineage_data);
            let filename = format!("{}-lineage.csv", document_id);
            Ok((
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{}\"", filename),
                    ),
                ],
                csv_content,
            ))
        }
        _ => {
            // Default: JSON export
            let json_content =
                serde_json::to_string_pretty(&combined).unwrap_or_else(|_| "{}".to_string());
            let filename = format!("{}-lineage.json", document_id);
            Ok((
                StatusCode::OK,
                [
                    (
                        header::CONTENT_TYPE,
                        "application/json; charset=utf-8".to_string(),
                    ),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{}\"", filename),
                    ),
                ],
                json_content,
            ))
        }
    }
}

/// Convert lineage data to CSV format.
///
/// WHY: Flattens the document → chunks hierarchy into a tabular format
/// with one row per chunk, suitable for spreadsheets and data analysis.
pub(super) fn lineage_to_csv(document_id: &str, lineage: &serde_json::Value) -> String {
    let mut csv = String::new();
    csv.push_str(
        "document_id,chunk_index,content_preview,tokens,start_line,end_line,entity_count\n",
    );

    if let Some(chunks) = lineage.get("chunks").and_then(|c| c.as_array()) {
        for chunk in chunks {
            let index = chunk
                .get("chunk_index")
                .or_else(|| chunk.get("index"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let content = chunk.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let preview = if content.len() > 100 {
                &content[..100]
            } else {
                content
            };
            // WHY: Escape CSV fields — wrap in quotes and double any internal quotes
            let escaped_preview = preview.replace('"', "\"\"").replace('\n', " ");
            let tokens = chunk.get("tokens").and_then(|v| v.as_u64()).unwrap_or(0);
            let start_line = chunk
                .get("start_line")
                .and_then(|v| v.as_u64())
                .map(|v| v.to_string())
                .unwrap_or_default();
            let end_line = chunk
                .get("end_line")
                .and_then(|v| v.as_u64())
                .map(|v| v.to_string())
                .unwrap_or_default();
            let entity_count = chunk
                .get("entity_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            csv.push_str(&format!(
                "{},{},\"{}\",{},{},{},{}\n",
                document_id, index, escaped_preview, tokens, start_line, end_line, entity_count
            ));
        }
    }

    csv
}
