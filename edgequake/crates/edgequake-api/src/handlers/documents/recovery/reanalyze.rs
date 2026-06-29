//! POST `/documents/{document_id}/reanalyze` — multimodal analyze without re-parse (Phase 4h).

use axum::response::Response;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use tracing::debug;

use crate::error::ApiResult;
use crate::handlers::documents_types::*;
use crate::middleware::TenantContext;
use crate::services::{reanalyze_document_multimodal, MultimodalReanalyzeParams};
use crate::state::AppState;

/// Core reanalyze logic (SOLID — shared by v1 HTTP handler and v2 job submission).
pub(crate) async fn run_reanalyze_multimodal(
    state: AppState,
    tenant_ctx: TenantContext,
    document_id: String,
    request: ReanalyzeMultimodalRequest,
) -> ApiResult<ReanalyzeMultimodalResponse> {
    debug!(
        document_id = %document_id,
        reindex = request.reindex,
        process_options = ?request.process_options,
        "run_reanalyze_multimodal"
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

    Ok(ReanalyzeMultimodalResponse {
        document_id: outcome.document_id,
        track_id: outcome.track_id,
        requeued: outcome.requeued,
        success: outcome.summary.success,
        skipped: outcome.summary.skipped,
        failed: outcome.summary.failed,
        v2_migration: tenant_ctx
            .workspace_id
            .as_ref()
            .map(|ws| crate::services::job_registry::v2_migration_hint("reanalyze_multimodal", ws)),
    })
}

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
        (status = 200, description = "Multimodal re-analyze completed (legacy default)", body = ReanalyzeMultimodalResponse),
        (status = 202, description = "Reanalyze accepted when REST-025 opt-in or strict startup", body = ReanalyzeMultimodalResponse),
        (status = 404, description = "Document not found"),
        (status = 422, description = "Analyze failed (strict mode)")
    )
)]
pub async fn reanalyze_multimodal(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(document_id): Path<String>,
    body: Option<Json<ReanalyzeMultimodalRequest>>,
) -> ApiResult<Response> {
    let request = body.map(|b| b.0).unwrap_or(ReanalyzeMultimodalRequest {
        process_options: None,
        reindex: true,
    });
    let workspace_id = tenant_ctx.workspace_id.clone();
    let return_202 = state.security.v1_rpc_return_202;
    let response = run_reanalyze_multimodal(state, tenant_ctx, document_id, request).await?;
    if let Some(ws) = workspace_id.as_deref() {
        let track_id = response.track_id.clone();
        return crate::services::v1_rpc_migration::respond_v1_async_rpc(
            ws,
            track_id.as_deref(),
            return_202,
            response,
        );
    }
    Ok(Json(response).into_response())
}
