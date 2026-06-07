//! In-memory PDF document storage (SPEC-017 STORE-SOLID-L-002 / STORE-SOLID-D-001).
//!
//! Enables upper-layer tests without PostgreSQL while preserving `PdfDocumentStorage`
//! semantics (workspace isolation, checksum deduplication, document FK helpers).

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

use crate::error::{Result, StorageError};
use crate::pdf_storage::*;

use super::lock::map_lock_err;

#[derive(Debug, Default)]
struct DocumentRecord {
    #[allow(dead_code)]
    workspace_id: Uuid,
    #[allow(dead_code)]
    tenant_id: Option<Uuid>,
    #[allow(dead_code)]
    title: String,
    #[allow(dead_code)]
    content: String,
    #[allow(dead_code)]
    status: String,
}

/// In-memory implementation of [`PdfDocumentStorage`].
pub struct MemoryPdfStorage {
    pdfs: RwLock<HashMap<Uuid, PdfDocument>>,
    /// `(workspace_id, checksum)` → pdf_id for deduplication.
    checksum_index: RwLock<HashMap<(Uuid, String), Uuid>>,
    documents: RwLock<HashMap<Uuid, DocumentRecord>>,
}

impl MemoryPdfStorage {
    pub fn new() -> Self {
        Self {
            pdfs: RwLock::new(HashMap::new()),
            checksum_index: RwLock::new(HashMap::new()),
            documents: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryPdfStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PdfDocumentStorage for MemoryPdfStorage {
    async fn create_pdf(&self, request: CreatePdfRequest) -> Result<Uuid> {
        validate_pdf_data(&request.pdf_data)?;

        if let Some(existing) = self
            .find_pdf_by_checksum(&request.workspace_id, &request.sha256_checksum)
            .await?
        {
            return Err(StorageError::Conflict(format!(
                "PDF already exists with ID: {}",
                existing.pdf_id
            )));
        }

        let pdf_id = Uuid::new_v4();
        let now = Utc::now();
        let doc = PdfDocument {
            pdf_id,
            workspace_id: request.workspace_id,
            document_id: None,
            filename: request.filename,
            content_type: request.content_type,
            file_size_bytes: request.file_size_bytes,
            sha256_checksum: request.sha256_checksum.clone(),
            page_count: request.page_count,
            pdf_data: request.pdf_data,
            processing_status: PdfProcessingStatus::Pending,
            extraction_method: None,
            vision_model: request.vision_model,
            markdown_content: None,
            extraction_errors: None,
            created_at: now,
            processed_at: None,
            updated_at: now,
        };

        {
            let mut pdfs = self.pdfs.write().map_err(map_lock_err)?;
            let mut index = self.checksum_index.write().map_err(map_lock_err)?;
            pdfs.insert(pdf_id, doc);
            index.insert((request.workspace_id, request.sha256_checksum), pdf_id);
        }

        Ok(pdf_id)
    }

    async fn get_pdf(&self, pdf_id: &Uuid) -> Result<Option<PdfDocument>> {
        let pdfs = self.pdfs.read().map_err(map_lock_err)?;
        Ok(pdfs.get(pdf_id).cloned())
    }

    async fn find_pdf_by_checksum(
        &self,
        workspace_id: &Uuid,
        checksum: &str,
    ) -> Result<Option<PdfDocument>> {
        let index = self.checksum_index.read().map_err(map_lock_err)?;
        let pdfs = self.pdfs.read().map_err(map_lock_err)?;
        Ok(index
            .get(&(*workspace_id, checksum.to_string()))
            .and_then(|id| pdfs.get(id).cloned()))
    }

    async fn update_pdf_status(&self, pdf_id: &Uuid, status: PdfProcessingStatus) -> Result<()> {
        let mut pdfs = self.pdfs.write().map_err(map_lock_err)?;
        let doc = pdfs
            .get_mut(pdf_id)
            .ok_or_else(|| StorageError::NotFound(format!("PDF {pdf_id} not found")))?;

        doc.processing_status = status;
        if matches!(
            status,
            PdfProcessingStatus::Completed | PdfProcessingStatus::Failed
        ) {
            doc.processed_at = Some(Utc::now());
        }
        doc.updated_at = Utc::now();
        Ok(())
    }

    async fn update_pdf_processing(&self, request: UpdatePdfProcessingRequest) -> Result<()> {
        let mut pdfs = self.pdfs.write().map_err(map_lock_err)?;
        let doc = pdfs
            .get_mut(&request.pdf_id)
            .ok_or_else(|| StorageError::NotFound(format!("PDF {} not found", request.pdf_id)))?;

        doc.processing_status = request.processing_status;
        if let Some(method) = request.extraction_method {
            doc.extraction_method = Some(method);
        }
        if let Some(md) = request.markdown_content {
            doc.markdown_content = Some(md);
        }
        if let Some(errs) = request.extraction_errors {
            doc.extraction_errors = Some(errs);
        }
        if let Some(document_id) = request.document_id {
            doc.document_id = Some(document_id);
        }
        if let Some(model) = request.vision_model {
            doc.vision_model = Some(model);
        }
        if matches!(
            request.processing_status,
            PdfProcessingStatus::Completed | PdfProcessingStatus::Failed
        ) {
            doc.processed_at = Some(Utc::now());
        }
        doc.updated_at = Utc::now();
        Ok(())
    }

    async fn link_pdf_to_document(&self, pdf_id: &Uuid, document_id: &Uuid) -> Result<()> {
        let documents = self.documents.read().map_err(map_lock_err)?;
        if !documents.contains_key(document_id) {
            return Err(StorageError::NotFound(format!(
                "Document {document_id} not found (call ensure_document_record first)"
            )));
        }
        drop(documents);

        let mut pdfs = self.pdfs.write().map_err(map_lock_err)?;
        let doc = pdfs
            .get_mut(pdf_id)
            .ok_or_else(|| StorageError::NotFound(format!("PDF {pdf_id} not found")))?;
        doc.document_id = Some(*document_id);
        doc.updated_at = Utc::now();
        Ok(())
    }

    async fn list_pdfs(&self, filter: ListPdfFilter) -> Result<PdfList> {
        let pdfs = self.pdfs.read().map_err(map_lock_err)?;
        let page = filter.page.unwrap_or(1).max(1);
        let page_size = filter.page_size.unwrap_or(20).max(1);

        let mut items: Vec<PdfDocument> = pdfs
            .values()
            .filter(|pdf| {
                if let Some(ws) = filter.workspace_id {
                    if pdf.workspace_id != ws {
                        return false;
                    }
                }
                if let Some(status) = filter.processing_status {
                    if pdf.processing_status != status {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        items.sort_by_key(|p| std::cmp::Reverse(p.created_at));
        let total_count = items.len() as i64;
        let offset = (page - 1) * page_size;
        items = items.into_iter().skip(offset).take(page_size).collect();

        Ok(PdfList {
            items,
            total_count,
            page,
            page_size,
        })
    }

    async fn delete_pdf(&self, pdf_id: &Uuid) -> Result<()> {
        let mut pdfs = self.pdfs.write().map_err(map_lock_err)?;
        let mut index = self.checksum_index.write().map_err(map_lock_err)?;
        let doc = pdfs
            .remove(pdf_id)
            .ok_or_else(|| StorageError::NotFound(format!("PDF {pdf_id} not found")))?;
        index.remove(&(doc.workspace_id, doc.sha256_checksum));
        Ok(())
    }

    async fn count_pdfs(
        &self,
        workspace_id: &Uuid,
        status: Option<PdfProcessingStatus>,
    ) -> Result<i64> {
        let pdfs = self.pdfs.read().map_err(map_lock_err)?;
        let count = pdfs
            .values()
            .filter(|pdf| {
                pdf.workspace_id == *workspace_id
                    && status.is_none_or(|s| pdf.processing_status == s)
            })
            .count();
        Ok(count as i64)
    }

    async fn ensure_document_record(
        &self,
        document_id: &Uuid,
        workspace_id: &Uuid,
        tenant_id: Option<&Uuid>,
        title: &str,
        content: &str,
        status: &str,
    ) -> Result<()> {
        let mut documents = self.documents.write().map_err(map_lock_err)?;
        documents.insert(
            *document_id,
            DocumentRecord {
                workspace_id: *workspace_id,
                tenant_id: tenant_id.copied(),
                title: title.to_string(),
                content: content.to_string(),
                status: status.to_string(),
            },
        );
        Ok(())
    }

    async fn delete_document_record(&self, document_id: &Uuid) -> Result<()> {
        let mut documents = self.documents.write().map_err(map_lock_err)?;
        documents.remove(document_id);

        let mut pdfs = self.pdfs.write().map_err(map_lock_err)?;
        let mut index = self.checksum_index.write().map_err(map_lock_err)?;
        let linked: Vec<Uuid> = pdfs
            .iter()
            .filter_map(|(id, pdf)| {
                if pdf.document_id == Some(*document_id) {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        for id in linked {
            if let Some(pdf) = pdfs.remove(&id) {
                index.remove(&(pdf.workspace_id, pdf.sha256_checksum));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_pdf_request(workspace_id: Uuid) -> CreatePdfRequest {
        CreatePdfRequest {
            workspace_id,
            filename: "test.pdf".into(),
            content_type: "application/pdf".into(),
            file_size_bytes: 8,
            sha256_checksum: calculate_pdf_checksum(b"%PDF-1.4"),
            page_count: Some(1),
            pdf_data: b"%PDF-1.4".to_vec(),
            vision_model: None,
        }
    }

    #[tokio::test]
    async fn create_and_deduplicate_by_checksum() {
        let storage = MemoryPdfStorage::new();
        let ws = Uuid::new_v4();
        let req = sample_pdf_request(ws);

        let id1 = storage.create_pdf(req.clone()).await.unwrap();
        let err = storage.create_pdf(req).await.unwrap_err();
        assert!(matches!(err, StorageError::Conflict(_)));

        let found = storage.get_pdf(&id1).await.unwrap().unwrap();
        assert_eq!(found.filename, "test.pdf");
    }

    #[tokio::test]
    async fn link_requires_document_record() {
        let storage = MemoryPdfStorage::new();
        let ws = Uuid::new_v4();
        let pdf_id = storage.create_pdf(sample_pdf_request(ws)).await.unwrap();
        let doc_id = Uuid::new_v4();

        assert!(storage
            .link_pdf_to_document(&pdf_id, &doc_id)
            .await
            .is_err());

        storage
            .ensure_document_record(&doc_id, &ws, None, "t", "c", "ready")
            .await
            .unwrap();
        storage
            .link_pdf_to_document(&pdf_id, &doc_id)
            .await
            .unwrap();
        assert_eq!(
            storage.get_pdf(&pdf_id).await.unwrap().unwrap().document_id,
            Some(doc_id)
        );
    }
}
