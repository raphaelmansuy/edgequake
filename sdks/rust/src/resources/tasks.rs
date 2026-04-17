//! Tasks resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::operations::*;

pub struct TasksResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> TasksResource<'a> {
    /// `GET /api/v1/tasks`
    pub async fn list(&self) -> Result<TaskListResponse> {
        self.client.get("/api/v1/tasks").await
    }

    /// `GET /api/v1/tasks/{track_id}`
    pub async fn get(&self, track_id: &str) -> Result<TaskInfo> {
        self.client.get(&format!("/api/v1/tasks/{track_id}")).await
    }

    /// `POST /api/v1/tasks/{track_id}/cancel`
    pub async fn cancel(&self, track_id: &str) -> Result<()> {
        self.client
            .post_no_content::<()>(&format!("/api/v1/tasks/{track_id}/cancel"), None)
            .await
    }

    /// `POST /api/v1/tasks/{track_id}/retry` — Retry a failed task.
    pub async fn retry(&self, track_id: &str) -> Result<serde_json::Value> {
        self.client
            .post::<(), serde_json::Value>(&format!("/api/v1/tasks/{track_id}/retry"), None)
            .await
    }
}
