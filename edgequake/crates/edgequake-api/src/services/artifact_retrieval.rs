//! Agent artifact retrieval SSOT (SPEC-028).
//!
//! Resolves document, chunk, figure, markdown, and PDF artifacts by stable ID
//! for Agentic Search follow-up after context bundle lineage hints.

use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::context_types::{
    ContextArtifactChunk, ContextArtifactDocument, ContextArtifactFigure, ContextArtifactMarkdown,
    ContextArtifactPdf, ContextArtifactResponse,
};
use crate::handlers::isolation::verify_document_access;
use crate::middleware::TenantContext;
use crate::services::document_body_loader::{load_document_body, pdf_api_paths};
use crate::services::{load_manifest, load_mm_chunks, manifest_item_status_views};
use crate::state::AppState;
use edgequake_storage::PdfProcessingStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArtifactKind {
    Document,
    Chunk,
    Figure,
    Markdown,
    Pdf,
}

fn parse_artifact_kind(raw: &str) -> ApiResult<ArtifactKind> {
    match raw.to_ascii_lowercase().as_str() {
        "document" | "doc" => Ok(ArtifactKind::Document),
        "chunk" => Ok(ArtifactKind::Chunk),
        "figure" | "drawing" | "image" => Ok(ArtifactKind::Figure),
        "markdown" | "md" => Ok(ArtifactKind::Markdown),
        "pdf" => Ok(ArtifactKind::Pdf),
        other => Err(ApiError::BadRequest(format!(
            "Unknown artifact_type '{other}' — use document, chunk, figure, markdown, or pdf"
        ))),
    }
}

pub struct ArtifactRetrievalOptions {
    pub document_id: Option<String>,
    pub include_content: bool,
}

pub async fn retrieve_artifact(
    state: &AppState,
    tenant_ctx: &TenantContext,
    artifact_type: &str,
    artifact_id: &str,
    options: ArtifactRetrievalOptions,
) -> ApiResult<ContextArtifactResponse> {
    let kind = parse_artifact_kind(artifact_type)?;
    match kind {
        ArtifactKind::Document => {
            retrieve_document_artifact(state, tenant_ctx, artifact_id, options.include_content)
                .await
        }
        ArtifactKind::Chunk => retrieve_chunk_artifact(state, tenant_ctx, artifact_id).await,
        ArtifactKind::Figure => {
            let document_id = options.document_id.ok_or_else(|| {
                ApiError::BadRequest(
                    "document_id query parameter required for figure artifacts".into(),
                )
            })?;
            retrieve_figure_artifact(state, tenant_ctx, &document_id, artifact_id).await
        }
        ArtifactKind::Markdown => retrieve_markdown_artifact(state, tenant_ctx, artifact_id).await,
        ArtifactKind::Pdf => {
            retrieve_pdf_artifact(
                state,
                tenant_ctx,
                artifact_id,
                options.document_id.as_deref(),
                options.include_content,
            )
            .await
        }
    }
}

async fn retrieve_document_artifact(
    state: &AppState,
    tenant_ctx: &TenantContext,
    document_id: &str,
    include_content: bool,
) -> ApiResult<ContextArtifactResponse> {
    let metadata =
        verify_document_access(state.storage.kv_storage.as_ref(), document_id, tenant_ctx).await?;

    let chunk_prefix = format!("{document_id}-chunk-");
    let chunk_keys = state
        .storage
        .kv_storage
        .keys_with_prefix(&chunk_prefix)
        .await?;

    let manifest = load_manifest(state.storage.kv_storage.as_ref(), document_id).await;
    let multimodal_item_count = manifest.as_ref().map(|m| m.items.len()).unwrap_or(0);

    let meta_obj = metadata.as_object();
    let title = meta_obj
        .and_then(|o| o.get("title"))
        .or_else(|| meta_obj.and_then(|o| o.get("file_name")))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let file_name = meta_obj
        .and_then(|o| o.get("file_name"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let mime_type = meta_obj
        .and_then(|o| o.get("mime_type"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let mut pdf_id = meta_obj
        .and_then(|o| o.get("pdf_id"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let body = if include_content {
        load_document_body(&state.storage, document_id, &metadata).await
    } else {
        None
    };

    if pdf_id.is_none() {
        pdf_id = body.as_ref().and_then(|b| b.pdf_id.clone());
    }

    let (content, markdown, content_source) = body.as_ref().map_or((None, None, None), |b| {
        (
            Some(b.markdown.clone()),
            Some(b.markdown.clone()),
            Some(b.source.as_str().to_string()),
        )
    });
    let content_summary = content
        .as_ref()
        .map(|c| c.chars().take(200).collect::<String>());

    let (pdf_download_path, pdf_content_path) = pdf_id
        .as_deref()
        .map(pdf_api_paths)
        .unwrap_or((String::new(), String::new()));
    let (pdf_download_path, pdf_content_path) = if pdf_id.is_some() {
        (Some(pdf_download_path), Some(pdf_content_path))
    } else {
        (None, None)
    };

    Ok(ContextArtifactResponse {
        artifact_type: "document".into(),
        artifact_id: document_id.to_string(),
        document: Some(ContextArtifactDocument {
            document_id: document_id.to_string(),
            title,
            file_name,
            mime_type,
            chunk_count: chunk_keys.len(),
            multimodal_item_count,
            pdf_id,
            content_summary,
            content,
            markdown,
            content_source,
            pdf_download_path,
            pdf_content_path,
        }),
        chunk: None,
        figure: None,
        markdown: None,
        pdf: None,
    })
}

async fn retrieve_markdown_artifact(
    state: &AppState,
    tenant_ctx: &TenantContext,
    document_id: &str,
) -> ApiResult<ContextArtifactResponse> {
    let metadata =
        verify_document_access(state.storage.kv_storage.as_ref(), document_id, tenant_ctx).await?;

    let body = load_document_body(&state.storage, document_id, &metadata)
        .await
        .ok_or_else(|| {
            ApiError::NotFound(format!("No markdown content for document '{document_id}'"))
        })?;

    Ok(ContextArtifactResponse {
        artifact_type: "markdown".into(),
        artifact_id: document_id.to_string(),
        document: None,
        chunk: None,
        figure: None,
        markdown: Some(ContextArtifactMarkdown {
            document_id: document_id.to_string(),
            markdown: body.markdown,
            source: body.source.as_str().to_string(),
            pdf_id: body.pdf_id,
        }),
        pdf: None,
    })
}

async fn retrieve_pdf_artifact(
    state: &AppState,
    tenant_ctx: &TenantContext,
    artifact_id: &str,
    document_id_hint: Option<&str>,
    include_markdown: bool,
) -> ApiResult<ContextArtifactResponse> {
    let pdf_storage =
        state
            .storage
            .pdf_storage
            .as_ref()
            .ok_or_else(|| ApiError::ServiceUnavailable {
                message: "PDF storage not available".into(),
                retry_after_secs: 30,
            })?;

    let (pdf_id, linked_document_id) = if let Ok(uuid) = Uuid::parse_str(artifact_id) {
        (uuid, document_id_hint.map(str::to_string))
    } else if let Some(doc_id) = document_id_hint {
        let metadata =
            verify_document_access(state.storage.kv_storage.as_ref(), doc_id, tenant_ctx).await?;
        let pdf_id_str = metadata
            .get("pdf_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ApiError::NotFound(format!("Document '{doc_id}' has no linked pdf_id"))
            })?;
        let uuid = Uuid::parse_str(pdf_id_str)
            .map_err(|_| ApiError::BadRequest(format!("Invalid pdf_id on document '{doc_id}'")))?;
        (uuid, Some(doc_id.to_string()))
    } else {
        return Err(ApiError::BadRequest(
            "pdf artifacts require a valid pdf_id UUID or document_id query parameter".into(),
        ));
    };

    let pdf = pdf_storage
        .get_pdf(&pdf_id)
        .await
        .map_err(|e| ApiError::Internal(format!("PDF lookup failed: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("PDF '{pdf_id}' not found")))?;

    if let Some(workspace_id) = tenant_ctx.workspace_id_uuid() {
        if pdf.workspace_id != workspace_id {
            return Err(ApiError::NotFound(format!("PDF '{pdf_id}' not found")));
        }
    }

    let (download_path, content_path) = pdf_api_paths(&pdf_id.to_string());
    let markdown_content = if include_markdown {
        pdf.markdown_content.clone()
    } else {
        None
    };

    Ok(ContextArtifactResponse {
        artifact_type: "pdf".into(),
        artifact_id: pdf_id.to_string(),
        document: None,
        chunk: None,
        figure: None,
        markdown: None,
        pdf: Some(ContextArtifactPdf {
            pdf_id: pdf_id.to_string(),
            document_id: linked_document_id.or_else(|| pdf.document_id.map(|id| id.to_string())),
            filename: pdf.filename,
            file_size_bytes: pdf.file_size_bytes,
            content_type: pdf.content_type,
            is_processed: pdf.processing_status == PdfProcessingStatus::Completed,
            download_path,
            content_path,
            markdown_content,
        }),
    })
}

async fn retrieve_chunk_artifact(
    state: &AppState,
    tenant_ctx: &TenantContext,
    chunk_id: &str,
) -> ApiResult<ContextArtifactResponse> {
    let chunk_data = state
        .storage
        .kv_storage
        .get_by_id(chunk_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Chunk '{chunk_id}' not found")))?;

    let document_id = if chunk_id.contains("-chunk-") {
        chunk_id
            .split("-chunk-")
            .next()
            .unwrap_or(chunk_id)
            .to_string()
    } else {
        chunk_data
            .get("document_id")
            .and_then(|v| v.as_str())
            .unwrap_or(chunk_id)
            .to_string()
    };

    verify_document_access(state.storage.kv_storage.as_ref(), &document_id, tenant_ctx).await?;

    let content = chunk_data
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let chunk_index = chunk_data
        .get("index")
        .or_else(|| chunk_data.get("chunk_index"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let token_count = chunk_data
        .get("token_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let start_line = chunk_data
        .get("start_line")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);
    let end_line = chunk_data
        .get("end_line")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    Ok(ContextArtifactResponse {
        artifact_type: "chunk".into(),
        artifact_id: chunk_id.to_string(),
        document: None,
        chunk: Some(ContextArtifactChunk {
            chunk_id: chunk_id.to_string(),
            document_id,
            content,
            chunk_index,
            token_count,
            start_line,
            end_line,
        }),
        figure: None,
        markdown: None,
        pdf: None,
    })
}

async fn retrieve_figure_artifact(
    state: &AppState,
    tenant_ctx: &TenantContext,
    document_id: &str,
    item_id: &str,
) -> ApiResult<ContextArtifactResponse> {
    verify_document_access(state.storage.kv_storage.as_ref(), document_id, tenant_ctx).await?;

    let manifest = load_manifest(state.storage.kv_storage.as_ref(), document_id)
        .await
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "No multimodal manifest for document '{document_id}'"
            ))
        })?;

    let item = manifest
        .items
        .iter()
        .find(|i| i.item_id == item_id)
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Figure '{item_id}' not found in document '{document_id}'"
            ))
        })?;

    let views = manifest_item_status_views(&manifest);
    let view = views
        .iter()
        .find(|v| v.item_id == item_id)
        .cloned()
        .unwrap_or_else(|| crate::services::MultimodalItemStatusView {
            item_id: item.item_id.clone(),
            modality: item.modality.clone(),
            status: "pending".into(),
            name: None,
            item_type: None,
            message: None,
        });

    let analyzed_text = load_mm_chunks(state.storage.kv_storage.as_ref(), document_id)
        .await
        .and_then(|chunks| {
            chunks
                .into_iter()
                .find(|c| c.item_id == item_id)
                .map(|c| c.text)
        });

    Ok(ContextArtifactResponse {
        artifact_type: "figure".into(),
        artifact_id: item_id.to_string(),
        document: None,
        chunk: None,
        figure: Some(ContextArtifactFigure {
            item_id: item_id.to_string(),
            document_id: document_id.to_string(),
            modality: view.modality,
            status: view.status,
            name: view.name,
            item_type: view.item_type,
            caption: item.caption.clone(),
            analyzed_text,
        }),
        markdown: None,
        pdf: None,
    })
}
