//! v2 job DTOs (SPEC-027 IMP-025).

use edgequake_tasks::Task;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::handlers::tasks_types::TaskResponse;

#[derive(Debug, Serialize, ToSchema)]
pub struct JobListResponse {
    pub jobs: Vec<JobResponse>,
    pub pagination: crate::handlers::tasks_types::PaginationInfo,
}

impl JobListResponse {
    pub fn from_task_list(task_list: crate::handlers::tasks_types::TaskListResponse) -> Self {
        Self {
            jobs: task_list
                .tasks
                .iter()
                .map(JobResponse::from_task_response)
                .collect(),
            pagination: task_list.pagination,
        }
    }
}

/// Create an async job (maps to background task queue).
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateJobRequest {
    /// Job type (`insert`, `scan`, `reindex`, `upload`, `pdf_processing`, `knowledge_injection`).
    pub job_type: String,
    /// Opaque payload stored on the underlying task.
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// Job resource (v2 REST wrapper over v1 task).
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
    pub v1_task: String,
}

impl JobResponse {
    pub fn from_task(task: &Task) -> Self {
        let job_id = task.track_id.clone();
        Self {
            job_id: job_id.clone(),
            job_type: task.task_type.to_string(),
            status: task.status.to_string(),
            tenant_id: task.tenant_id.to_string(),
            workspace_id: task.workspace_id.to_string(),
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
            links: JobLinks {
                self_link: format!("/api/v2/jobs/{job_id}"),
                v1_task: format!("/api/v1/tasks/{job_id}"),
            },
        }
    }

    pub fn from_task_response(task: &TaskResponse) -> Self {
        let job_id = task.track_id.clone();
        Self {
            job_id: job_id.clone(),
            job_type: task.task_type.clone(),
            status: task.status.clone(),
            tenant_id: task.tenant_id.clone(),
            workspace_id: task.workspace_id.clone(),
            created_at: task.created_at.clone(),
            updated_at: task.updated_at.clone(),
            links: JobLinks {
                self_link: format!("/api/v2/jobs/{job_id}"),
                v1_task: format!("/api/v1/tasks/{job_id}"),
            },
        }
    }
}
