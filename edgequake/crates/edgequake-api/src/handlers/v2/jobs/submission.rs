//! v2 job submission dispatch (SPEC-027 Level 4 REST).
//!
//! Delegates to task queue or existing v1 handler logic (DRY) without duplicating
//! rebuild/reprocess business rules.

use edgequake_tasks::{Task, TaskType};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::documents::{
    run_reanalyze_multimodal, run_recover_stuck, run_reprocess_failed,
};
use crate::handlers::documents_types::{
    ReanalyzeMultimodalRequest, RecoverStuckRequest, ReprocessFailedRequest,
};
use crate::handlers::v2::jobs::types::CreateJobRequest;
use crate::handlers::workspaces::{
    run_rebuild_embeddings, run_rebuild_knowledge_graph, run_reprocess_all_documents,
};
use crate::handlers::workspaces_types::{
    RebuildEmbeddingsRequest, RebuildKnowledgeGraphRequest, ReprocessAllRequest,
};
use crate::middleware::TenantContext;
use crate::services::job_registry::is_creatable_v2_job_type;
use crate::state::AppState;

/// Result of submitting a v2 job (always yields a track/job id).
#[derive(Debug, Clone)]
pub struct SubmittedJob {
    pub job_id: String,
    pub job_type: String,
}

/// Validate path workspace matches tenant header (Level 4 resource scope).
pub fn ensure_workspace_scope(workspace_id: Uuid, tenant_ctx: &TenantContext) -> ApiResult<()> {
    let ctx_ws = tenant_ctx
        .workspace_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("X-Workspace-ID required".into()))?;
    if ctx_ws != workspace_id {
        return Err(ApiError::BadRequest(
            "Path workspace_id must match X-Workspace-ID header".into(),
        ));
    }
    Ok(())
}

fn payload_as<T: DeserializeOwned>(payload: &serde_json::Value) -> ApiResult<T> {
    let value = if payload.is_null() {
        serde_json::json!({})
    } else {
        payload.clone()
    };
    serde_json::from_value(value).map_err(|e| ApiError::BadRequest(format!("Invalid payload: {e}")))
}

fn parse_task_type(raw: &str) -> ApiResult<TaskType> {
    match raw.to_ascii_lowercase().as_str() {
        "upload" => Ok(TaskType::Upload),
        "insert" => Ok(TaskType::Insert),
        "scan" => Ok(TaskType::Scan),
        "reindex" => Ok(TaskType::Reindex),
        "pdf_processing" => Ok(TaskType::PdfProcessing),
        "knowledge_injection" => Ok(TaskType::KnowledgeInjection),
        other => Err(ApiError::BadRequest(format!("Unknown job_type '{other}'"))),
    }
}

/// Submit a workspace-scoped async job (Level 4 dispatch SSOT).
pub async fn submit_workspace_job(
    state: &AppState,
    workspace_id: Uuid,
    tenant_ctx: &TenantContext,
    request: &CreateJobRequest,
) -> ApiResult<SubmittedJob> {
    ensure_workspace_scope(workspace_id, tenant_ctx)?;

    let tenant_id = tenant_ctx
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("X-Tenant-ID required".into()))?;

    let job_type = request.job_type.to_ascii_lowercase();

    if !is_creatable_v2_job_type(&job_type) {
        return Err(ApiError::BadRequest(format!(
            "Unsupported job_type '{job_type}'. See GET .../jobs/catalog for supported types."
        )));
    }

    match job_type.as_str() {
        "upload" | "insert" | "scan" | "reindex" | "pdf_processing" | "knowledge_injection" => {
            let task_type = parse_task_type(&job_type)?;
            let payload = if request.payload.is_null() {
                serde_json::json!({})
            } else {
                request.payload.clone()
            };
            let task = Task::new(tenant_id, workspace_id, task_type, payload);
            let job_id = task.track_id.clone();
            state.enqueue_task(task).await?;
            Ok(SubmittedJob { job_id, job_type })
        }
        "rebuild_embeddings" => {
            let req: RebuildEmbeddingsRequest = payload_as(&request.payload)?;
            let response =
                run_rebuild_embeddings(state.clone(), workspace_id, tenant_ctx.clone(), req)
                    .await?;
            let job_id = response.job_id.clone().ok_or_else(|| {
                ApiError::Internal("rebuild_embeddings returned no job_id".into())
            })?;
            Ok(SubmittedJob { job_id, job_type })
        }
        "rebuild_knowledge_graph" => {
            let req: RebuildKnowledgeGraphRequest = payload_as(&request.payload)?;
            let response =
                run_rebuild_knowledge_graph(state.clone(), workspace_id, tenant_ctx.clone(), req)
                    .await?;
            let job_id = response.track_id.clone().ok_or_else(|| {
                ApiError::Internal("rebuild_knowledge_graph returned no track_id".into())
            })?;
            Ok(SubmittedJob { job_id, job_type })
        }
        "reprocess_all" => {
            let req: ReprocessAllRequest = payload_as(&request.payload)?;
            let response =
                run_reprocess_all_documents(state.clone(), workspace_id, tenant_ctx.clone(), req)
                    .await?;
            Ok(SubmittedJob {
                job_id: response.track_id,
                job_type,
            })
        }
        "reprocess_failed" => {
            let req: ReprocessFailedRequest = payload_as(&request.payload)?;
            let response = run_reprocess_failed(state.clone(), tenant_ctx.clone(), req).await?;
            Ok(SubmittedJob {
                job_id: response.track_id,
                job_type,
            })
        }
        "recover_stuck" => {
            let req: RecoverStuckRequest = payload_as(&request.payload)?;
            let response = run_recover_stuck(state.clone(), tenant_ctx.clone(), req).await?;
            Ok(SubmittedJob {
                job_id: response.track_id,
                job_type,
            })
        }
        "reanalyze_multimodal" => {
            let document_id = request
                .payload
                .get("document_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ApiError::BadRequest("reanalyze_multimodal requires payload.document_id".into())
                })?
                .to_string();
            let req: ReanalyzeMultimodalRequest = payload_as(&request.payload)?;
            let response = run_reanalyze_multimodal(
                state.clone(),
                tenant_ctx.clone(),
                document_id.clone(),
                req,
            )
            .await?;
            let job_id = response.track_id.clone().unwrap_or_else(|| {
                format!(
                    "reanalyze_{}_{}",
                    document_id,
                    &Uuid::new_v4().to_string()[..8]
                )
            });
            Ok(SubmittedJob { job_id, job_type })
        }
        other => unreachable!("creatable job types validated above: {other}"),
    }
}
