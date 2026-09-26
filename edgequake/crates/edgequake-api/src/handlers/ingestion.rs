//! SPEC-048: GET /ingestion/{track_id}/progress (+ batch).
//! 068: text/MD progress identity = insert-* task_id; include staging metadata.

use axum::{
    extract::{Path, State},
    Json,
};

use crate::error::{ApiError, ApiResult};
use crate::handlers::ingestion_types::{BatchIngestionProgressRequest, IngestionProgressResponse};
use crate::middleware::TenantContext;
use crate::services::document_metadata_scan::load_scoped_document_metadata_for_progress;
use crate::services::progress_facade::progress_from_document_metadata;
use crate::services::tenant_guard::has_full_tenant_context;
use crate::state::AppState;

/// Match progress track against metadata `track_id` or `task_id` (068 defense-in-depth).
fn metadata_matches_progress_track(
    obj: &serde_json::Map<String, serde_json::Value>,
    track_id: &str,
) -> bool {
    let doc_track = obj.get("track_id").and_then(|v| v.as_str()).unwrap_or("");
    if doc_track == track_id {
        return true;
    }
    let task_id = obj.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
    task_id == track_id
}

fn find_progress_for_track(
    metadata_values: &[serde_json::Value],
    track_id: &str,
) -> Option<IngestionProgressResponse> {
    for value in metadata_values {
        let Some(obj) = value.as_object() else {
            continue;
        };
        if metadata_matches_progress_track(obj, track_id) {
            return Some(progress_from_document_metadata(track_id, obj));
        }
    }
    None
}

/// Get real-time ingestion progress for a track ID (SPEC-048 DEF-01 / 068).
#[utoipa::path(
    get,
    path = "/api/v1/ingestion/{track_id}/progress",
    tag = "Pipeline",
    params(
        ("track_id" = String, Path, description = "Ingestion track ID")
    ),
    responses(
        (status = 200, description = "Track progress", body = IngestionProgressResponse),
        (status = 404, description = "Track not found")
    )
)]
pub async fn get_ingestion_progress(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(track_id): Path<String>,
) -> ApiResult<Json<IngestionProgressResponse>> {
    if !has_full_tenant_context(&tenant_ctx) {
        return Err(ApiError::NotFound(format!("Track not found: {}", track_id)));
    }

    let metadata_values = load_scoped_document_metadata_for_progress(
        state.storage.kv_storage.as_ref(),
        state.optional_pg_pool(),
        &tenant_ctx,
    )
    .await?;

    find_progress_for_track(&metadata_values, &track_id)
        .map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("Track not found: {}", track_id)))
}

/// Batch progress for multiple tracks (FE getMultipleTrackProgress).
#[utoipa::path(
    post,
    path = "/api/v1/ingestion/progress",
    tag = "Pipeline",
    request_body = BatchIngestionProgressRequest,
    responses(
        (status = 200, description = "Track progress list", body = Vec<IngestionProgressResponse>)
    )
)]
pub async fn post_ingestion_progress_batch(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(body): Json<BatchIngestionProgressRequest>,
) -> ApiResult<Json<Vec<IngestionProgressResponse>>> {
    if !has_full_tenant_context(&tenant_ctx) {
        return Ok(Json(vec![]));
    }

    let metadata_values = load_scoped_document_metadata_for_progress(
        state.storage.kv_storage.as_ref(),
        state.optional_pg_pool(),
        &tenant_ctx,
    )
    .await?;

    let mut out = Vec::new();
    for track_id in body.track_ids {
        if let Some(p) = find_progress_for_track(&metadata_values, &track_id) {
            out.push(p);
        }
    }
    Ok(Json(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn find_progress_matches_track_id() {
        let values = vec![json!({
            "id": "doc-1",
            "track_id": "insert-abc",
            "status": "pending",
            "title": "a.md",
            "current_stage": "extracting",
        })];
        let p = find_progress_for_track(&values, "insert-abc").expect("found");
        assert_eq!(p.track_id, "insert-abc");
    }

    #[test]
    fn find_progress_matches_task_id_when_track_differs() {
        // Legacy rows: track_id=upload_*, state.optional_pg_pool(), task_id=insert-*
        let values = vec![json!({
            "id": "doc-2",
            "track_id": "upload_20260722000000_deadbeef",
            "task_id": "insert-legacy",
            "status": "processing",
            "title": "b.md",
            "current_stage": "extracting",
        })];
        let p = find_progress_for_track(&values, "insert-legacy").expect("found via task_id");
        assert_eq!(p.track_id, "insert-legacy");
    }

    #[test]
    fn find_progress_misses_unrelated_track() {
        let values = vec![json!({
            "id": "doc-3",
            "track_id": "insert-other",
            "status": "pending",
            "title": "c.md",
        })];
        assert!(find_progress_for_track(&values, "insert-missing").is_none());
    }
}
