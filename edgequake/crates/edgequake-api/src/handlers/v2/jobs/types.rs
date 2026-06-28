//! v2 job DTOs — Level 4 workspace-scoped REST resources (SPEC-027 IMP-025).

use edgequake_tasks::Task;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::handlers::tasks_types::TaskResponse;

#[derive(Debug, Serialize, ToSchema)]
pub struct JobListResponse {
    pub workspace_id: String,
    pub jobs: Vec<JobResponse>,
    pub pagination: crate::handlers::tasks_types::PaginationInfo,
    pub links: JobCollectionLinks,
}

impl JobListResponse {
    pub fn from_task_list(
        workspace_id: &str,
        task_list: crate::handlers::tasks_types::TaskListResponse,
    ) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            jobs: task_list
                .tasks
                .iter()
                .map(|t| JobResponse::from_task_response_for_workspace(workspace_id, t))
                .collect(),
            pagination: task_list.pagination,
            links: JobCollectionLinks::for_workspace(workspace_id),
        }
    }
}

/// Create an async job under a workspace.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateJobRequest {
    /// Job type — see `GET .../jobs/catalog` for supported values.
    pub job_type: String,
    /// Type-specific payload (document_id for reanalyze, force for rebuild, etc.).
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// Job resource (Level 4 — workspace-scoped async operation).
#[derive(Debug, Serialize, ToSchema)]
pub struct JobResponse {
    pub job_id: String,
    pub job_type: String,
    pub status: String,
    pub tenant_id: String,
    pub workspace_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub links: JobLinks,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct JobLinks {
    pub self_link: String,
    /// DELETE target to cancel a pending job.
    pub cancel: String,
    pub catalog: String,
    /// Legacy v1 task monitor (migration hint).
    pub v1_task: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct JobCollectionLinks {
    pub self_link: String,
    pub catalog: String,
}

impl JobCollectionLinks {
    pub fn for_workspace(workspace_id: &str) -> Self {
        Self {
            self_link: format!("/api/v2/workspaces/{workspace_id}/jobs"),
            catalog: format!("/api/v2/workspaces/{workspace_id}/jobs/catalog"),
        }
    }
}

impl JobLinks {
    pub fn for_job(workspace_id: &str, job_id: &str) -> Self {
        let resource = format!("/api/v2/workspaces/{workspace_id}/jobs/{job_id}");
        Self {
            self_link: resource.clone(),
            cancel: resource,
            catalog: format!("/api/v2/workspaces/{workspace_id}/jobs/catalog"),
            v1_task: format!("/api/v1/tasks/{job_id}"),
        }
    }
}

impl JobResponse {
    pub fn from_task_for_workspace(workspace_id: &str, task: &Task) -> Self {
        let job_id = task.track_id.clone();
        Self {
            job_id: job_id.clone(),
            job_type: task.task_type.to_string(),
            status: task.status.to_string(),
            tenant_id: task.tenant_id.to_string(),
            workspace_id: task.workspace_id.to_string(),
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
            links: JobLinks::for_job(workspace_id, &job_id),
        }
    }

    pub fn from_task_response_for_workspace(workspace_id: &str, task: &TaskResponse) -> Self {
        let job_id = task.track_id.clone();
        Self {
            job_id: job_id.clone(),
            job_type: task.task_type.clone(),
            status: task.status.clone(),
            tenant_id: task.tenant_id.clone(),
            workspace_id: task.workspace_id.clone(),
            created_at: task.created_at.clone(),
            updated_at: task.updated_at.clone(),
            links: JobLinks::for_job(workspace_id, &job_id),
        }
    }

    pub fn synthetic_accepted(
        workspace_id: &str,
        tenant_ctx: &crate::middleware::TenantContext,
        job_id: &str,
        job_type: &str,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            job_id: job_id.to_string(),
            job_type: job_type.to_string(),
            status: "pending".to_string(),
            tenant_id: tenant_ctx
                .tenant_id
                .clone()
                .unwrap_or_else(|| "unknown".into()),
            workspace_id: workspace_id.to_string(),
            created_at: now.clone(),
            updated_at: now,
            links: JobLinks::for_job(workspace_id, job_id),
        }
    }
}
