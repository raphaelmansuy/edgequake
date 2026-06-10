//! Bulk re-enrich documents whose topic belongs to the legacy taxonomy.
//!
//! Queues MetadataEnrich tasks for every document in the workspace whose
//! `enrichment_topic` is in the old topic list (or all enriched docs when
//! `force=true`).

use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::isolation::doc_belongs_to_workspace;
use crate::middleware::TenantContext;
use crate::state::AppState;
use edgequake_tasks::{MetadataEnrichData, Task, TaskStatus, TaskType};

/// Topics from the old taxonomy that should be re-classified.
const LEGACY_TOPICS: &[&str] = &[
    "Politics", "Indonesia", "Psychology", "Business", "Communication",
    "Technology", "Science", "SocialMedia", "Religion", "Media",
    "AI", "Culinary", "Finance", "Literature", "Law",
    "Sports", "Education",
    // "History" and "Uncategorized" exist in both old and new — always re-enrich these too
    "History", "Uncategorized",
];

#[derive(Debug, Deserialize)]
pub struct ReEnrichQuery {
    /// When true, re-enrich ALL documents regardless of current topic.
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReEnrichResponse {
    pub queued: usize,
    pub skipped: usize,
    pub track_id: String,
    pub message: String,
}

/// Bulk re-enrich documents with stale/legacy topics.
///
/// By default only queues documents whose `enrichment_topic` is from the
/// old taxonomy. Pass `?force=true` to re-enrich every document in the
/// workspace regardless.
#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/re-enrich",
    tag = "Workspaces",
    params(
        ("workspace_id" = String, Path, description = "Workspace ID"),
        ("force" = Option<bool>, Query, description = "Re-enrich all docs regardless of topic (default: false)")
    ),
    responses(
        (status = 200, description = "Re-enrich tasks queued", body = ReEnrichResponse),
        (status = 404, description = "Workspace not found")
    )
)]
pub async fn re_enrich_documents(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(workspace_id): Path<String>,
    Query(params): Query<ReEnrichQuery>,
) -> ApiResult<Json<ReEnrichResponse>> {
    let workspace_uuid = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".to_string()))?;

    let workspace = state
        .workspace_service
        .get_workspace(workspace_uuid)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to load workspace: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("Workspace not found: {}", workspace_id)))?;

    let tenant_id = tenant_ctx
        .tenant_id_uuid()
        .unwrap_or(workspace.tenant_id);

    let track_id = format!("re_enrich_{}_{}", workspace_id, Uuid::new_v4());

    let all_keys = state
        .kv_storage
        .keys()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to list keys: {}", e)))?;

    let mut queued = 0usize;
    let mut skipped = 0usize;

    for key in all_keys.iter().filter(|k| k.ends_with("-metadata")) {
        let value = match state.kv_storage.get_by_id(key).await {
            Ok(Some(v)) => v,
            _ => continue,
        };
        let obj = match value.as_object() {
            Some(o) => o,
            None => continue,
        };

        // Workspace isolation
        let doc_workspace = obj.get("workspace_id").and_then(|v| v.as_str()).unwrap_or("default");
        if !doc_belongs_to_workspace(doc_workspace, &workspace_uuid.to_string(), &workspace.slug) {
            continue;
        }

        // Only PDF documents can be enriched (need pdf_id)
        let pdf_id_str = match obj.get("pdf_id").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => { skipped += 1; continue; }
        };
        let pdf_id = match Uuid::parse_str(pdf_id_str) {
            Ok(id) => id,
            Err(_) => { skipped += 1; continue; }
        };

        // Filter by legacy topic unless force=true
        if !params.force {
            let current_topic = obj.get("enrichment_topic").and_then(|v| v.as_str()).unwrap_or("");
            let is_legacy = current_topic.is_empty()
                || LEGACY_TOPICS.iter().any(|&t| t.eq_ignore_ascii_case(current_topic));
            if !is_legacy {
                skipped += 1;
                continue;
            }
        }

        let doc_id = match obj.get("id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };

        let doc_tenant_id = obj
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or(tenant_id);

        let doc_workspace_id = obj
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or(workspace_uuid);

        // Reset enrichment fields so the processor writes fresh results
        let mut updated = obj.clone();
        updated.insert("enrichment_status".to_string(), serde_json::json!("pending"));
        updated.remove("enrichment_topic");
        updated.remove("enrichment_summary");
        updated.remove("enrichment_language");
        updated.remove("enrichment_keywords");
        updated.remove("enrichment_tags");
        updated.remove("enrichment_completed_at");
        updated.remove("enrichment_error");

        if let Err(e) = state.kv_storage.upsert(&[(key.clone(), serde_json::Value::Object(updated))]).await {
            tracing::warn!(doc_id = %doc_id, error = %e, "Failed to reset enrichment fields, skipping");
            skipped += 1;
            continue;
        }

        let enrich_data = MetadataEnrichData {
            document_id: doc_id.clone(),
            pdf_id,
            tenant_id: doc_tenant_id,
            workspace_id: doc_workspace_id,
            max_pages: 5,
        };

        let task_track_id = format!("{}-{}", track_id, doc_id);
        let task = Task {
            track_id: task_track_id,
            tenant_id: doc_tenant_id,
            workspace_id: doc_workspace_id,
            task_type: TaskType::MetadataEnrich,
            status: TaskStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            error: None,
            retry_count: 0,
            max_retries: 3,
            consecutive_timeout_failures: 0,
            circuit_breaker_tripped: false,
            task_data: serde_json::to_value(&enrich_data)
                .map_err(|e| ApiError::Internal(format!("Serialization failed: {}", e)))?,
            metadata: None,
            progress: None,
            result: None,
        };

        if let Err(e) = state.task_storage.create_task(&task).await {
            tracing::warn!(doc_id = %doc_id, error = %e, "Failed to create enrich task, skipping");
            skipped += 1;
            continue;
        }

        if let Err(e) = state.enrich_queue.send(task).await {
            tracing::warn!(doc_id = %doc_id, error = %e, "Failed to queue enrich task");
            skipped += 1;
            continue;
        }

        tracing::info!(doc_id = %doc_id, "Re-enrich task queued");
        queued += 1;
    }

    Ok(Json(ReEnrichResponse {
        queued,
        skipped,
        track_id,
        message: format!("Queued {} documents for re-enrichment ({} skipped)", queued, skipped),
    }))
}
