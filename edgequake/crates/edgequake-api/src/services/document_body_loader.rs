//! Document markdown/body loader SSOT (SPEC-028 DRY).
//!
//! Unifies KV `{doc_id}-content` reads with PDF pipeline `markdown_content` hydration.

use serde_json::Value;
use uuid::Uuid;

use crate::state::StorageRuntime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentBodySource {
    Kv,
    PdfStorage,
}

impl DocumentBodySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Kv => "kv",
            Self::PdfStorage => "pdf_storage",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DocumentBody {
    pub markdown: String,
    pub source: DocumentBodySource,
    pub pdf_id: Option<String>,
}

pub async fn load_document_body(
    storage: &StorageRuntime,
    document_id: &str,
    metadata: &Value,
) -> Option<DocumentBody> {
    if let Some(body) = load_kv_document_body(storage, document_id).await {
        return Some(body);
    }
    load_pdf_markdown_body(storage, metadata).await
}

async fn load_kv_document_body(
    storage: &StorageRuntime,
    document_id: &str,
) -> Option<DocumentBody> {
    let content_key = format!("{document_id}-content");
    let values = storage.kv_storage.get_by_ids(&[content_key]).await.ok()?;
    let markdown = values.into_iter().next().and_then(|v| {
        v.get("content")
            .or_else(|| v.get("text"))
            .and_then(|c| c.as_str())
            .map(str::to_string)
            .filter(|s| !s.is_empty())
    })?;
    Some(DocumentBody {
        markdown,
        source: DocumentBodySource::Kv,
        pdf_id: None,
    })
}

async fn load_pdf_markdown_body(
    storage: &StorageRuntime,
    metadata: &Value,
) -> Option<DocumentBody> {
    let obj = metadata.as_object()?;
    let is_pdf = obj
        .get("source_type")
        .and_then(|v| v.as_str())
        .is_some_and(|s| s == "pdf");
    if !is_pdf {
        return None;
    }
    let pdf_id_str = obj.get("pdf_id").and_then(|v| v.as_str())?;
    let pdf_uuid = Uuid::parse_str(pdf_id_str).ok()?;
    let pdf_storage = storage.pdf_storage.as_ref()?;
    let pdf = pdf_storage.get_pdf(&pdf_uuid).await.ok()??;
    let markdown = pdf.markdown_content.filter(|s| !s.trim().is_empty())?;
    Some(DocumentBody {
        markdown,
        source: DocumentBodySource::PdfStorage,
        pdf_id: Some(pdf_id_str.to_string()),
    })
}

pub fn pdf_api_paths(pdf_id: &str) -> (String, String) {
    (
        format!("/api/v1/documents/pdf/{pdf_id}/download"),
        format!("/api/v1/documents/pdf/{pdf_id}/content"),
    )
}
