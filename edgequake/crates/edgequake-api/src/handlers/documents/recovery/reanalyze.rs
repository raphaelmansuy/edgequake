//! POST `/documents/{document_id}/reanalyze` — multimodal analyze without re-parse (Phase 4h).

use axum::{
    extract::{Path, State},
    Json,
};
use tracing::debug;

use crate::error::ApiResult;
use crate::handlers::documents_types::*;
use crate::middleware::TenantContext;
use crate::services::{reanalyze_document_multimodal, MultimodalReanalyzeParams};
use crate::state::AppState;

/// Re-run multimodal analyze on stored markdown (LightRAG analyze worker parity).
#[utoipa::path(
    post,
    path = "/api/v1/documents/{document_id}/reanalyze",
    tag = "Documents",
    params(
        ("document_id" = String, Path, description = "Document ID")
    ),
    request_body = ReanalyzeMultimodalRequest,
    responses(
        (status = 200, description = "Multimodal re-analyze completed or reindex queued", body = ReanalyzeMultimodalResponse),
        (status = 404, description = "Document not found"),
        (status = 422, description = "Analyze failed (strict mode)")
    )
)]
pub async fn reanalyze_multimodal(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(document_id): Path<String>,
    body: Option<Json<ReanalyzeMultimodalRequest>>,
) -> ApiResult<Json<ReanalyzeMultimodalResponse>> {
    let request = body.map(|b| b.0).unwrap_or(ReanalyzeMultimodalRequest {
        process_options: None,
        reindex: true,
    });

    debug!(
        document_id = %document_id,
        reindex = request.reindex,
        process_options = ?request.process_options,
        "reanalyze_multimodal"
    );

    let outcome = reanalyze_document_multimodal(
        &state,
        &tenant_ctx,
        MultimodalReanalyzeParams {
            document_id: document_id.clone(),
            process_options: request.process_options,
            reindex: request.reindex,
        },
    )
    .await?;

    Ok(Json(ReanalyzeMultimodalResponse {
        document_id: outcome.document_id,
        track_id: outcome.track_id,
        requeued: outcome.requeued,
        success: outcome.summary.success,
        skipped: outcome.summary.skipped,
        failed: outcome.summary.failed,
    }))
}
