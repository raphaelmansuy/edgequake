//! PostgreSQL implementation of PDF document storage.
//!
//! @implements SPEC-007: PDF Upload Support
//! @implements BR0701: Workspace isolation via RLS

use async_trait::async_trait;
use sqlx::PgPool;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::error::{Result, StorageError};
use crate::pdf_storage::*;

/// PostgreSQL implementation of PdfDocumentStorage.
pub struct PostgresPdfStorage {
    pool: PgPool,
}

impl PostgresPdfStorage {
    /// Create a new PostgreSQL PDF storage instance.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PdfDocumentStorage for PostgresPdfStorage {
    async fn create_pdf(&self, request: CreatePdfRequest) -> Result<Uuid> {
        // Validate PDF data
        validate_pdf_data(&request.pdf_data)?;

        // Check for duplicate
        if let Some(existing) = self
            .find_pdf_by_checksum(&request.workspace_id, &request.sha256_checksum)
            .await?
        {
            warn!(
                "Duplicate PDF upload detected: checksum={}, existing_id={}",
                request.sha256_checksum, existing.pdf_id
            );
            return Err(StorageError::Conflict(format!(
                "PDF already exists with ID: {}",
                existing.pdf_id
            )));
        }

        // Insert PDF
        let pdf_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO pdf_documents (
                pdf_id,
                workspace_id,
                filename,
                content_type,
                file_size_bytes,
                sha256_checksum,
                page_count,
                pdf_data,
                processing_status,
                vision_model
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            pdf_id,
            request.workspace_id,
            request.filename,
            request.content_type,
            request.file_size_bytes,
            request.sha256_checksum,
            request.page_count,
            request.pdf_data,
            PdfProcessingStatus::Pending.as_str(),
            request.vision_model,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            // FIX-DUPLICATE-BUG: Convert unique constraint violation to Conflict error.
            // WHY: The idx_pdf_documents_workspace_checksum_unique constraint catches
            // TOCTOU race conditions where two concurrent uploads of the same PDF
            // both pass the application-level find_pdf_by_checksum check.
            if let sqlx::Error::Database(ref db_err) = e {
                // PostgreSQL error code 23505 = unique_violation
                if db_err.code().as_deref() == Some("23505") {
                    return StorageError::Conflict(format!(
                        "PDF with checksum {} already exists in this workspace (concurrent upload detected)",
                        request.sha256_checksum
                    ));
                }
            }
            StorageError::Database(format!("Failed to create PDF document: {}", e))
        })?;

        debug!(
            "Created PDF document: id={}, workspace={}, size={}",
            pdf_id, request.workspace_id, request.file_size_bytes
        );

        Ok(pdf_id)
    }

    async fn get_pdf(&self, pdf_id: &Uuid) -> Result<Option<PdfDocument>> {
        let row = sqlx::query!(
            r#"
            SELECT
                pdf_id,
                workspace_id,
                document_id,
                filename,
                content_type,
                file_size_bytes,
                sha256_checksum,
                page_count,
                pdf_data,
                processing_status,
                extraction_method,
                vision_model,
                markdown_content,
                extraction_errors,
                created_at,
                processed_at,
                updated_at
            FROM pdf_documents
            WHERE pdf_id = $1
            "#,
            pdf_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to get PDF: {}", e)))?;

        Ok(row.map(|r| PdfDocument {
            pdf_id: r.pdf_id,
            workspace_id: r.workspace_id,
            document_id: r.document_id,
            filename: r.filename,
            content_type: r.content_type,
            file_size_bytes: r.file_size_bytes,
            sha256_checksum: r.sha256_checksum,
            page_count: r.page_count,
            pdf_data: r.pdf_data,
            processing_status: r.processing_status.parse().unwrap(),
            extraction_method: r.extraction_method.as_ref().and_then(|m| m.parse().ok()),
            vision_model: r.vision_model,
            markdown_content: r.markdown_content,
            extraction_errors: r.extraction_errors,
            created_at: r.created_at,
            processed_at: r.processed_at,
            updated_at: r.updated_at,
        }))
    }

    async fn find_pdf_by_checksum(
        &self,
        workspace_id: &Uuid,
        checksum: &str,
    ) -> Result<Option<PdfDocument>> {
        let row = sqlx::query!(
            r#"
            SELECT
                pdf_id,
                workspace_id,
                document_id,
                filename,
                content_type,
                file_size_bytes,
                sha256_checksum,
                page_count,
                pdf_data,
                processing_status,
                extraction_method,
                vision_model,
                markdown_content,
                extraction_errors,
                created_at,
                processed_at,
                updated_at
            FROM pdf_documents
            WHERE workspace_id = $1 AND sha256_checksum = $2
            LIMIT 1
            "#,
            workspace_id,
            checksum
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to find PDF by checksum: {}", e)))?;

        Ok(row.map(|r| PdfDocument {
            pdf_id: r.pdf_id,
            workspace_id: r.workspace_id,
            document_id: r.document_id,
            filename: r.filename,
            content_type: r.content_type,
            file_size_bytes: r.file_size_bytes,
            sha256_checksum: r.sha256_checksum,
            page_count: r.page_count,
            pdf_data: r.pdf_data,
            processing_status: r.processing_status.parse().unwrap(),
            extraction_method: r.extraction_method.as_ref().and_then(|m| m.parse().ok()),
            vision_model: r.vision_model,
            markdown_content: r.markdown_content,
            extraction_errors: r.extraction_errors,
            created_at: r.created_at,
            processed_at: r.processed_at,
            updated_at: r.updated_at,
        }))
    }

    async fn update_pdf_status(&self, pdf_id: &Uuid, status: PdfProcessingStatus) -> Result<()> {
        let status_str = status.as_str();

        let processed_at =
            if status == PdfProcessingStatus::Completed || status == PdfProcessingStatus::Failed {
                Some(chrono::Utc::now())
            } else {
                None
            };

        sqlx::query!(
            r#"
            UPDATE pdf_documents
            SET processing_status = $1,
                processed_at = COALESCE($2, processed_at)
            WHERE pdf_id = $3
            "#,
            status_str,
            processed_at,
            pdf_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to update PDF status: {}", e)))?;

        debug!("Updated PDF status: id={}, status={}", pdf_id, status_str);

        Ok(())
    }

    async fn update_pdf_processing(&self, request: UpdatePdfProcessingRequest) -> Result<()> {
        let status_str = request.processing_status.as_str();
        let method_str = request.extraction_method.map(|m| m.as_str().to_string());

        let processed_at = if request.processing_status == PdfProcessingStatus::Completed
            || request.processing_status == PdfProcessingStatus::Failed
        {
            Some(chrono::Utc::now())
        } else {
            None
        };

        // FIX-REBUILD: Include vision_model in the UPDATE statement.
        // WHY: When reprocessing with a different vision LLM (e.g. gpt-4o-mini → gemma3:12b),
        // the vision_model column must be updated to reflect the model actually used.
        // Previously this field was never written, leaving stale model info in the DB.
        sqlx::query!(
            r#"
            UPDATE pdf_documents
            SET processing_status = $1,
                extraction_method = COALESCE($2, extraction_method),
                markdown_content = COALESCE($3, markdown_content),
                extraction_errors = COALESCE($4, extraction_errors),
                document_id = COALESCE($5, document_id),
                processed_at = COALESCE($6, processed_at),
                vision_model = COALESCE($8, vision_model)
            WHERE pdf_id = $7
            "#,
            status_str,
            method_str,
            request.markdown_content,
            request.extraction_errors,
            request.document_id,
            processed_at,
            request.pdf_id,
            request.vision_model
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to update PDF processing: {}", e)))?;

        debug!(
            "Updated PDF processing: id={}, status={}, method={:?}, vision_model={:?}",
            request.pdf_id, status_str, method_str, request.vision_model
        );

        Ok(())
    }

    async fn link_pdf_to_document(&self, pdf_id: &Uuid, document_id: &Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE pdf_documents
            SET document_id = $1
            WHERE pdf_id = $2
            "#,
            document_id,
            pdf_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to link PDF to document: {}", e)))?;

        debug!(
            "Linked PDF to document: pdf_id={}, document_id={}",
            pdf_id, document_id
        );

        Ok(())
    }

    async fn list_pdfs(&self, filter: ListPdfFilter) -> Result<PdfList> {
        let page = filter.page.unwrap_or(1);
        let page_size = filter.page_size.unwrap_or(20);
        let offset = ((page - 1) * page_size) as i64;
        let limit = page_size as i64;

        let status_filter = filter.processing_status.map(|s| s.as_str().to_string());

        // Get total count
        let total_count: i64 = if let Some(workspace_id) = filter.workspace_id {
            if let Some(status) = &status_filter {
                sqlx::query_scalar!(
                    r#"
                    SELECT COUNT(*) as "count!"
                    FROM pdf_documents
                    WHERE workspace_id = $1 AND processing_status = $2
                    "#,
                    workspace_id,
                    status
                )
                .fetch_one(&self.pool)
                .await?
            } else {
                sqlx::query_scalar!(
                    r#"
                    SELECT COUNT(*) as "count!"
                    FROM pdf_documents
                    WHERE workspace_id = $1
                    "#,
                    workspace_id
                )
                .fetch_one(&self.pool)
                .await?
            }
        } else if let Some(status) = &status_filter {
            sqlx::query_scalar!(
                r#"
                SELECT COUNT(*) as "count!"
                FROM pdf_documents
                WHERE processing_status = $1
                "#,
                status
            )
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_scalar!(
                r#"
                SELECT COUNT(*) as "count!"
                FROM pdf_documents
                "#
            )
            .fetch_one(&self.pool)
            .await?
        };

        // Get paginated items using helper
        let items =
            super::pdf_list_query::list_pdfs_dynamic(&self.pool, &filter, limit, offset).await?;

        Ok(PdfList {
            items,
            total_count,
            page,
            page_size,
        })
    }

    async fn delete_pdf(&self, pdf_id: &Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM pdf_documents
            WHERE pdf_id = $1
            "#,
            pdf_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to delete PDF: {}", e)))?;

        debug!("Deleted PDF: id={}", pdf_id);

        Ok(())
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
        // WHY: INSERT ... ON CONFLICT ensures idempotency (safe to call multiple times).
        // Updates status and content on conflict so reprocessing refreshes the record.
        // @implements FIX-ISSUE-74: Ensure document record exists before FK link
        sqlx::query(
            r#"
            INSERT INTO documents (id, tenant_id, workspace_id, title, content, status, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (id) DO UPDATE SET
                content = EXCLUDED.content,
                status  = EXCLUDED.status,
                title   = EXCLUDED.title,
                updated_at = NOW()
            "#,
        )
        .bind(document_id)
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(title)
        .bind(content)
        .bind(status)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to ensure document record: {}", e)))?;

        debug!(
            "Ensured document record: id={}, workspace_id={}",
            document_id, workspace_id
        );

        Ok(())
    }

    async fn delete_document_record(&self, document_id: &Uuid) -> Result<()> {
        // WHY: CASCADE on chunks.document_id and pdf_documents.document_id
        // means this single DELETE propagates to related rows automatically.
        // @implements FIX-ISSUE-73: Cascade delete pdf_documents/chunks on document removal
        let result = sqlx::query(
            r#"
            DELETE FROM documents WHERE id = $1
            "#,
        )
        .bind(document_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to delete document record: {}", e)))?;

        debug!(
            "Deleted document record: id={}, rows_affected={}",
            document_id,
            result.rows_affected()
        );

        Ok(())
    }

    async fn count_pdfs(
        &self,
        workspace_id: &Uuid,
        status: Option<PdfProcessingStatus>,
    ) -> Result<i64> {
        let count = if let Some(status) = status {
            let status_str = status.as_str();
            sqlx::query_scalar!(
                r#"
                SELECT COUNT(*) as "count!"
                FROM pdf_documents
                WHERE workspace_id = $1 AND processing_status = $2
                "#,
                workspace_id,
                status_str
            )
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_scalar!(
                r#"
                SELECT COUNT(*) as "count!"
                FROM pdf_documents
                WHERE workspace_id = $1
                "#,
                workspace_id
            )
            .fetch_one(&self.pool)
            .await?
        };

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    // Note: Integration tests would require a test database
    // These are placeholder unit tests

    #[test]
    fn test_postgres_pdf_storage_creation() {
        // This is a placeholder - actual tests would use sqlx test pool
        assert!(true);
    }
}
