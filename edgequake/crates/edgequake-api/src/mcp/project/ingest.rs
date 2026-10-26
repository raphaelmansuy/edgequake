//! Text ingest for MCP (SPEC-161). Admission is async. The tool returns pending + task_id.

use serde_json::{json, Value};

use crate::error::{ApiError, ApiResult};
use crate::handlers::documents::upload::{
    admit_document_for_processing, DocumentAdmissionInput, DocumentAdmissionOutcome,
    GleaningAdmissionOptions, UploadExtraction,
};
use crate::middleware::TenantContext;
use crate::services::ContentHasher;
use crate::state::AppState;

use super::budget::BudgetClass;
use super::envelope::EnvelopeBuilder;
use super::errors::{eq_error, ErrorCode};

/// Admit inline text. Does not wait for pipeline processing.
pub async fn eq_ingest(
    state: &AppState,
    tenant_ctx: &TenantContext,
    args: &Value,
) -> ApiResult<Value> {
    let access = args
        .get("access")
        .map(|value| {
            serde_json::from_value::<crate::services::document_access_types::DocumentAccess>(
                value.clone(),
            )
            .map_err(|_| {
                ApiError::bad_request_coded("INVALID_ACCESS_PAYLOAD", "Invalid access envelope")
            })
        })
        .transpose()?;
    let content = args.get("content").and_then(|v| v.as_str());
    let content_b64 = args.get("content_base64").and_then(|v| v.as_str());
    let upload_ref = args.get("upload_ref").and_then(|v| v.as_str());

    let kinds = [
        content.filter(|s| !s.is_empty()).is_some(),
        content_b64.filter(|s| !s.is_empty()).is_some(),
        upload_ref.filter(|s| !s.is_empty()).is_some(),
    ]
    .iter()
    .filter(|v| **v)
    .count();

    if kinds == 0 {
        return Ok(eq_error(
            ErrorCode::InvalidId,
            "content, content_base64, or upload_ref required",
            None,
        ));
    }
    if kinds > 1 {
        return Ok(eq_error(
            ErrorCode::InvalidId,
            "provide only one of content, content_base64, or upload_ref",
            None,
        ));
    }

    if let Some(b64) = content_b64.filter(|s| !s.is_empty()) {
        let bytes = match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64) {
            Ok(b) => b,
            Err(_) => {
                return Ok(eq_error(
                    ErrorCode::InvalidId,
                    "content_base64 must be standard Base64",
                    None,
                ));
            }
        };
        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("upload.bin");
        return super::upload_session::admit_bytes_with_access(
            state,
            tenant_ctx,
            title,
            "application/octet-stream",
            bytes,
            access,
        )
        .await;
    }

    if let Some(upload_id) = upload_ref.filter(|s| !s.is_empty()) {
        if access.is_some() {
            return Err(ApiError::bad_request_coded(
                "INVALID_ACCESS_PAYLOAD",
                "Upload session access is immutable",
            ));
        }
        return super::upload_session::commit_upload_id(state, tenant_ctx, upload_id).await;
    }

    let text = content.unwrap_or("");
    if text.trim().is_empty() {
        return Ok(eq_error(
            ErrorCode::InvalidId,
            "content must not be empty",
            None,
        ));
    }

    if crate::validation::validate_content(text, state.config.max_document_size).is_err() {
        return Ok(eq_error(
            ErrorCode::BudgetExceeded,
            "content exceeds max document size",
            None,
        ));
    }

    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("Untitled")
        .to_string();
    let content_hash = ContentHasher::hash_str(text);
    let content_length = text.len();
    let extraction = UploadExtraction {
        mode: Some("llm".into()),
        gate_preset: None,
    };

    let outcome = match admit_document_for_processing(
        state,
        tenant_ctx,
        DocumentAdmissionInput {
            access,
            text_content: text.to_string(),
            title,
            source_type: "markdown",
            mime_type: Some("text/markdown".to_string()),
            raw_byte_size: content_length,
            content_hash,
            custom_metadata: extraction.merge_into(None),
            track_id: None,
            expected_batch_count: None,
            gleaning: GleaningAdmissionOptions::default(),
            document_type: Some("markdown"),
            chunk_strategy: None,
            chunk_options: None,
            extract_max_entities: None,
            extract_max_records: None,
            multimodal: false,
            ingest_mode: None,
            multimodal_manifest: None,
        },
        "upload",
    )
    .await
    {
        Ok(o) => o,
        Err(e) => {
            return Ok(eq_error(ErrorCode::InvalidId, e.to_string(), None));
        }
    };

    Ok(envelope_from_admission(outcome))
}

pub(crate) fn envelope_from_admission(outcome: DocumentAdmissionOutcome) -> Value {
    match outcome {
        DocumentAdmissionOutcome::DuplicateProcessing(dup) => {
            EnvelopeBuilder::new("ingest", BudgetClass::Standard)
                .insert("document_id", json!(dup.document_id))
                .insert("task_id", json!(Value::Null))
                .insert("status", json!("duplicate_processing"))
                .insert("ready", json!(false))
                .build()
        }
        DocumentAdmissionOutcome::Accepted(accepted) => {
            EnvelopeBuilder::new("ingest", BudgetClass::Standard)
                .insert("document_id", json!(accepted.document_id))
                .insert("task_id", json!(accepted.track_id))
                .insert("status", json!("pending"))
                .insert("ready", json!(false))
                .insert(
                    "poll",
                    json!({
                        "tool": "eq_task_get",
                        "until": ["indexed", "failed", "cancelled"],
                    }),
                )
                .build()
        }
    }
}

pub async fn eq_task_get(
    state: &AppState,
    tenant_ctx: &TenantContext,
    args: &Value,
) -> ApiResult<Value> {
    let task_id = args
        .get("task_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("task_id required".into()))?;

    match crate::services::task_scope::get_task_for_context(state, task_id, tenant_ctx).await {
        Ok(task) => {
            let status = task.status.to_string();
            let ready = status == "indexed";
            let mut b = EnvelopeBuilder::new("task_get", BudgetClass::Standard)
                .insert("task_id", json!(task_id))
                .insert("status", json!(status))
                .insert("ready", json!(ready))
                .insert("task_type", json!(task.task_type.to_string()));
            if let Some(err) = task.error_message {
                b = b.insert("error_message", json!(err));
            }
            Ok(b.build())
        }
        Err(ApiError::NotFound(_)) => Ok(eq_error(
            ErrorCode::NotFound,
            format!("Task not found: {task_id}"),
            None,
        )),
        Err(e) => Err(e),
    }
}
