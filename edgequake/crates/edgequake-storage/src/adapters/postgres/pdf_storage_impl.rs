//! PostgreSQL implementation of PDF document storage.
//!
//! @implements SPEC-007: PDF Upload Support
//! @implements BR0701: Workspace isolation via RLS

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
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

impl PostgresPdfStorage {
    /// PostgreSQL `undefined_column` (42703) — migration 041 not applied yet.
    ///
    /// WHY: Do NOT match bare "does not exist" — that also matches missing
    /// relations/functions and falsely routes to the legacy path, which used to
    /// leave `documents.relationship_count` at 0 while only patching JSONB.
    fn is_missing_column_error(err: &sqlx::Error) -> bool {
        match err {
            sqlx::Error::Database(db) => {
                if db.code().as_deref() == Some("42703") {
                    return true;
                }
                let msg = db.message().to_ascii_lowercase();
                msg.contains("column") && msg.contains("does not exist")
            }
            _ => false,
        }
    }

    fn stats_metadata_patch(stats: &DocumentStatsUpdate<'_>) -> serde_json::Value {
        let mut patch = serde_json::Map::new();
        if let Some(cost) = stats.cost_usd {
            patch.insert("cost_usd".into(), serde_json::json!(cost));
        }
        if let Some(v) = stats.input_tokens {
            patch.insert("input_tokens".into(), serde_json::json!(v));
        }
        if let Some(v) = stats.output_tokens {
            patch.insert("output_tokens".into(), serde_json::json!(v));
        }
        if let Some(v) = stats.total_tokens {
            patch.insert("total_tokens".into(), serde_json::json!(v));
        }
        patch.insert(
            "relationship_count".into(),
            serde_json::json!(stats.relationship_count.max(0)),
        );
        if let Some(msg) = stats.error_message {
            patch.insert("error_message".into(), serde_json::json!(msg));
        } else {
            // Non-failure stats writes must clear a stale error_message so the
            // UI cannot keep a prior persist failure after a successful commit.
            patch.insert("error_message".into(), serde_json::Value::Null);
        }
        serde_json::Value::Object(patch)
    }

    async fn update_document_stats_legacy(
        &self,
        stats: &DocumentStatsUpdate<'_>,
        metadata_patch: serde_json::Value,
    ) -> Result<()> {
        let chunk_count = stats.chunk_count.max(0);
        let entity_count = stats.entity_count.max(0);
        let relationship_count = stats.relationship_count.max(0);
        // SPEC-129: legacy fallback must also project CHECK-safe status.
        let pg_status = crate::relational_documents_status_for_write(stats.status);

        // Prefer writing relationship_count into the column when present (M041+).
        // Fall back to metadata-only if that column is still missing on older DBs.
        let result = sqlx::query(
            r#"
            UPDATE public.documents SET
                chunk_count        = $2,
                entity_count       = $3,
                relationship_count = $4,
                status             = $5,
                updated_at         = NOW(),
                metadata           = COALESCE(metadata, '{}'::jsonb) || $6::jsonb
            WHERE id = $1
            "#,
        )
        .bind(stats.document_id)
        .bind(chunk_count)
        .bind(entity_count)
        .bind(relationship_count)
        .bind(&pg_status)
        .bind(metadata_patch.clone())
        .execute(&self.pool)
        .await;

        let result = match result {
            Ok(r) => r,
            Err(e) if Self::is_missing_column_error(&e) => {
                // Pre-M041 schema: no relationship_count column — metadata JSONB only.
                sqlx::query(
                    r#"
                    UPDATE public.documents SET
                        chunk_count = $2,
                        entity_count  = $3,
                        status        = $4,
                        updated_at    = NOW(),
                        metadata      = COALESCE(metadata, '{}'::jsonb) || $5::jsonb
                    WHERE id = $1
                    "#,
                )
                .bind(stats.document_id)
                .bind(chunk_count)
                .bind(entity_count)
                .bind(&pg_status)
                .bind(metadata_patch)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    StorageError::Database(format!("Failed to update document stats: {}", e))
                })?
            }
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "Failed to update document stats: {}",
                    e
                )));
            }
        };

        if result.rows_affected() == 0 {
            warn!(
                document_id = %stats.document_id,
                "update_document_stats (legacy): documents row not found — stats in metadata only until row exists"
            );
        } else {
            debug!(
                document_id = %stats.document_id,
                chunk_count = chunk_count,
                entity_count = entity_count,
                relationship_count = relationship_count,
                status = %pg_status,
                "Updated document stats via legacy/metadata fallback"
            );
        }

        Ok(())
    }

    /// SPEC-090 F-090-16: load metadata + bytes from `pdf_document_blobs` (fail if blob missing).
    async fn fetch_pdf_row(
        &self,
        where_sql: &str,
        bind_pdf_id: Option<&Uuid>,
        bind_workspace: Option<&Uuid>,
        bind_checksum: Option<&str>,
    ) -> Result<Option<PdfDocument>> {
        let sql = format!(
            r#"
            SELECT
                d.pdf_id,
                d.workspace_id,
                d.document_id,
                d.filename,
                d.content_type,
                d.file_size_bytes,
                d.sha256_checksum,
                d.page_count,
                b.pdf_data AS pdf_data,
                d.processing_status,
                d.extraction_method,
                d.vision_model,
                d.markdown_content,
                d.extraction_errors,
                d.created_at,
                d.processed_at,
                d.updated_at
            FROM pdf_documents d
            INNER JOIN pdf_document_blobs b ON b.pdf_id = d.pdf_id
            {where_sql}
            LIMIT 1
            "#
        );
        let mut q = sqlx::query(&sql);
        if let Some(id) = bind_pdf_id {
            q = q.bind(id);
        }
        if let Some(ws) = bind_workspace {
            q = q.bind(ws);
        }
        if let Some(cs) = bind_checksum {
            q = q.bind(cs);
        }
        let row = q
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to fetch PDF: {e}")))?;
        Ok(row.map(|r| PdfDocument {
            pdf_id: r.get("pdf_id"),
            workspace_id: r.get("workspace_id"),
            document_id: r.get("document_id"),
            filename: r.get("filename"),
            content_type: r.get("content_type"),
            file_size_bytes: r.get("file_size_bytes"),
            sha256_checksum: r.get("sha256_checksum"),
            page_count: r.get("page_count"),
            pdf_data: r.get("pdf_data"),
            processing_status: r
                .get::<String, _>("processing_status")
                .parse()
                .unwrap_or(PdfProcessingStatus::Pending),
            extraction_method: r
                .get::<Option<String>, _>("extraction_method")
                .as_ref()
                .and_then(|m| m.parse().ok()),
            vision_model: r.get("vision_model"),
            markdown_content: r.get("markdown_content"),
            extraction_errors: r.get("extraction_errors"),
            created_at: r.get::<DateTime<Utc>, _>("created_at"),
            processed_at: r.get("processed_at"),
            updated_at: r.get::<DateTime<Utc>, _>("updated_at"),
        }))
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

        // SPEC-090 F-090-16: metadata row + blob side-table (SSOT for bytes) in one TX.
        let pdf_id = Uuid::new_v4();
        let mut tx = self.pool.begin().await.map_err(|e| {
            StorageError::Database(format!("Failed to begin PDF create transaction: {e}"))
        })?;

        sqlx::query(
            r#"
            INSERT INTO pdf_documents (
                pdf_id,
                workspace_id,
                filename,
                content_type,
                file_size_bytes,
                sha256_checksum,
                page_count,
                processing_status,
                vision_model
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(pdf_id)
        .bind(request.workspace_id)
        .bind(&request.filename)
        .bind(&request.content_type)
        .bind(request.file_size_bytes)
        .bind(&request.sha256_checksum)
        .bind(request.page_count)
        .bind(PdfProcessingStatus::Pending.as_str())
        .bind(&request.vision_model)
        .execute(&mut *tx)
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

        sqlx::query(
            r#"
            INSERT INTO pdf_document_blobs (pdf_id, pdf_data)
            VALUES ($1, $2)
            ON CONFLICT (pdf_id) DO UPDATE SET
                pdf_data = EXCLUDED.pdf_data,
                updated_at = NOW()
            "#,
        )
        .bind(pdf_id)
        .bind(&request.pdf_data)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            StorageError::Database(format!(
                "Failed to write pdf_document_blobs for {pdf_id}: {e}"
            ))
        })?;

        tx.commit().await.map_err(|e| {
            StorageError::Database(format!("Failed to commit PDF create transaction: {e}"))
        })?;

        debug!(
            "Created PDF document: id={}, workspace={}, size={}",
            pdf_id, request.workspace_id, request.file_size_bytes
        );

        Ok(pdf_id)
    }

    async fn get_pdf(&self, pdf_id: &Uuid) -> Result<Option<PdfDocument>> {
        self.fetch_pdf_row("WHERE d.pdf_id = $1", Some(pdf_id), None, None)
            .await
    }

    async fn find_pdf_by_checksum(
        &self,
        workspace_id: &Uuid,
        checksum: &str,
    ) -> Result<Option<PdfDocument>> {
        self.fetch_pdf_row(
            "WHERE d.workspace_id = $1 AND d.sha256_checksum = $2",
            None,
            Some(workspace_id),
            Some(checksum),
        )
        .await
    }

    async fn update_pdf_status(&self, pdf_id: &Uuid, status: PdfProcessingStatus) -> Result<()> {
        let status_str = status.as_str();

        let processed_at = if status.is_terminal() {
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

    async fn update_pdf_page_count(&self, pdf_id: &Uuid, page_count: i32) -> Result<()> {
        // Use non-macro query so SQLX_OFFLINE builds don't require a cache refresh
        // for this heal-only path.
        sqlx::query(
            r#"
            UPDATE pdf_documents
            SET page_count = $1
            WHERE pdf_id = $2
            "#,
        )
        .bind(page_count)
        .bind(pdf_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to update PDF page_count: {}", e)))?;

        debug!(
            "Updated PDF page_count: id={}, page_count={}",
            pdf_id, page_count
        );
        Ok(())
    }

    async fn update_pdf_processing(&self, request: UpdatePdfProcessingRequest) -> Result<()> {
        let status_str = request.processing_status.as_str();
        let method_str = request.extraction_method.map(|m| m.as_str().to_string());

        let processed_at = if request.processing_status.is_terminal() {
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

    async fn clear_markdown(&self, pdf_id: &Uuid) -> Result<()> {
        // WHY: Explicitly NULL out markdown_content + extraction metadata so the
        // pdf_processing resume shortcut cannot reuse a stale conversion. This is
        // the only way to force a true PDF -> markdown re-conversion, because
        // `update_pdf_processing` uses COALESCE and never clears these columns.
        sqlx::query!(
            r#"
            UPDATE pdf_documents
            SET markdown_content = NULL,
                extraction_method = NULL,
                extraction_errors = NULL
            WHERE pdf_id = $1
            "#,
            pdf_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to clear PDF markdown: {}", e)))?;

        debug!("Cleared cached markdown for PDF: id={}", pdf_id);

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
        // INVARIANT: never shrink `content` — a shorter payload (e.g. legacy 500-char
        // summary mistaken for body) must not clobber a longer stored body.
        // @implements FIX-ISSUE-74: Ensure document record exists before FK link
        // SPEC-129: storage owns CHECK projection — never bind raw KV stages.
        let pg_status = crate::relational_documents_status_for_write(status);
        sqlx::query(
            r#"
            INSERT INTO public.documents (id, tenant_id, workspace_id, title, content, status, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (id) DO UPDATE SET
                content = CASE
                    WHEN length(EXCLUDED.content) >= length(documents.content)
                        THEN EXCLUDED.content
                    ELSE documents.content
                END,
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
        .bind(&pg_status)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to ensure document record: {}", e)))?;

        debug!(
            "Ensured document record: id={}, workspace_id={}",
            document_id, workspace_id
        );

        Ok(())
    }

    async fn update_document_stats(&self, stats: &DocumentStatsUpdate<'_>) -> Result<()> {
        // SPEC-021 P-A1/P-A2: refresh the denormalized stat/cost columns.
        //
        // Idempotent + race-safe: if the row was never inserted by
        // `ensure_document_record` (race), UPDATE affects 0 rows. We log a
        // warn and return Ok so the caller's best-effort contract holds; the
        // next `ensure_document_record` + a follow-up stats refresh will
        // converge. We deliberately do NOT upsert-create the row here because
        // we do not have workspace_id/tenant_id/title/content at this call
        // site — those are `ensure_document_record`'s responsibility.
        //
        // Counts are clamped to >= 0 to defend against buggy upstream counters
        // (E5 in file 17 §10). Cost/token columns are nullable and left as-is
        // when the caller passes None (preserving any prior value).
        let chunk_count = stats.chunk_count.max(0);
        let entity_count = stats.entity_count.max(0);
        let relationship_count = stats.relationship_count.max(0);

        // SPEC-129: defense-in-depth — never bind a raw KV stage to the column.
        let pg_status = crate::relational_documents_status_for_write(stats.status);

        let result = sqlx::query(
            r#"
            UPDATE public.documents SET
                chunk_count        = $2,
                entity_count       = $3,
                relationship_count = $4,
                cost_usd           = COALESCE($5, cost_usd),
                input_tokens       = COALESCE($6, input_tokens),
                output_tokens      = COALESCE($7, output_tokens),
                total_tokens       = COALESCE($8, total_tokens),
                error_message      = $9,
                status             = $10,
                updated_at         = NOW()
            WHERE id = $1
            "#,
        )
        .bind(stats.document_id)
        .bind(chunk_count)
        .bind(entity_count)
        .bind(relationship_count)
        .bind(stats.cost_usd)
        .bind(stats.input_tokens)
        .bind(stats.output_tokens)
        .bind(stats.total_tokens)
        .bind(stats.error_message)
        .bind(&pg_status)
        .execute(&self.pool)
        .await;

        let result = match result {
            Ok(r) => r,
            Err(e) if Self::is_missing_column_error(&e) => {
                warn!(
                    document_id = %stats.document_id,
                    "documents M041 stat columns missing — falling back to metadata JSONB patch"
                );
                return self
                    .update_document_stats_legacy(stats, Self::stats_metadata_patch(stats))
                    .await;
            }
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "Failed to update document stats: {}",
                    e
                )));
            }
        };

        if result.rows_affected() == 0 {
            warn!(
                document_id = %stats.document_id,
                "update_document_stats: documents row not found (race with ensure_document_record) — stats not persisted; will converge on next refresh"
            );
        } else {
            debug!(
                document_id = %stats.document_id,
                chunk_count = chunk_count,
                entity_count = entity_count,
                relationship_count = relationship_count,
                status = %pg_status,
                "Updated document stats (SPEC-021 P-A1)"
            );
        }

        Ok(())
    }

    async fn touch_document_status(&self, document_id: &Uuid, status: &str) -> Result<()> {
        // SPEC-047 P1: status + updated_at only — never clobber entity_count.
        // SPEC-129: project via SSOT so KV stages like `re_embedding` never
        // violate `documents_valid_status`.
        let pg_status = crate::relational_documents_status_for_write(status);
        let result = sqlx::query(
            r#"
            UPDATE public.documents SET
                status     = $2,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(document_id)
        .bind(&pg_status)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to touch document status: {e}")))?;

        if result.rows_affected() == 0 {
            warn!(
                document_id = %document_id,
                "touch_document_status: no documents row yet (non-fatal)"
            );
        }
        Ok(())
    }

    async fn delete_document_record(&self, document_id: &Uuid) -> Result<()> {
        // WHY: CASCADE on chunks.document_id and pdf_documents.document_id
        // means this single DELETE propagates to related rows automatically.
        // @implements FIX-ISSUE-73: Cascade delete pdf_documents/chunks on document removal
        let result = sqlx::query(
            r#"
            DELETE FROM public.documents WHERE id = $1
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
