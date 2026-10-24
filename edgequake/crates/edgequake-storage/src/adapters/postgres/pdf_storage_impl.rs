//! PostgreSQL implementation of PDF document storage.
//!
//! @implements SPEC-007: PDF Upload Support
//! @implements BR0701: Workspace isolation via RLS

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::error::{Result, StorageError};
use crate::originals::{
    after_object_checksum, chunk_matches, chunks_covering, column_matches_ref, digest_at,
    get_inclusive, markdown_pointer, pdf_pointer, prefer_object, read_verified,
    remove_delete_queue, scoped_object_key, workspace_tenant, BlobStore, ObjectPointer,
    ObjectServe, OriginalsClient, OriginalsError, OwnerKind, PDF_CHUNK_BYTES,
};
use crate::pdf_storage::*;

use super::pdf_object::create_object_pdf;

/// PostgreSQL implementation of PdfDocumentStorage.
pub struct PostgresPdfStorage {
    pool: PgPool,
    objects: Option<Arc<OriginalsClient>>,
}

impl PostgresPdfStorage {
    /// Create a new PostgreSQL PDF storage instance. Database mode: no bucket.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            objects: None,
        }
    }

    /// Object mode. `None` keeps the BYTEA path.
    pub fn with_objects(mut self, objects: Option<Arc<OriginalsClient>>) -> Self {
        self.objects = objects;
        self
    }

    /// Key for this PDF only when the pointer's tenant, workspace, and key
    /// all name the workspace that owns the row.
    async fn tenant_pdf_key(
        &self,
        _pdf_id: Uuid,
        workspace_id: Uuid,
        pointer: &ObjectPointer,
    ) -> Result<Option<String>> {
        let Some(client) = &self.objects else {
            return Ok(None);
        };
        let tenant = workspace_tenant(&self.pool, workspace_id)
            .await
            .map_err(|err| StorageError::Database(format!("PDF tenant: {err}")))?;
        Ok(scoped_object_key(
            &client.prefix,
            OwnerKind::Pdf,
            tenant,
            workspace_id,
            pointer.tenant_id,
            pointer.workspace_id,
            &pointer.key,
        ))
    }

    /// Column text in database mode. Object mode fills from the bucket when the
    /// ref is verified and the key names this workspace. A mismatch returns no
    /// text and does not call the bucket.
    async fn markdown_text(
        &self,
        workspace_id: Uuid,
        owner: Uuid,
        location: String,
        column: Option<String>,
    ) -> Result<Option<String>> {
        let Some(client) = &self.objects else {
            return Ok(column);
        };
        let pointer = markdown_pointer(&self.pool, owner)
            .await
            .map_err(|err| StorageError::Database(format!("Failed to read markdown ref: {err}")))?;
        let tenant = workspace_tenant(&self.pool, workspace_id)
            .await
            .map_err(|err| StorageError::Database(format!("markdown tenant: {err}")))?;
        let scoped = pointer.as_ref().and_then(|pointer| {
            scoped_object_key(
                &client.prefix,
                OwnerKind::Markdown,
                tenant,
                workspace_id,
                pointer.tenant_id,
                pointer.workspace_id,
                &pointer.key,
            )
        });
        let verified = scoped.is_some() && pointer.as_ref().is_some_and(|p| p.verified);
        if location == "object" && scoped.is_none() {
            return Ok(None);
        }
        if !prefer_object(true, &location, verified) {
            return Ok(column);
        }
        let Some(key) = scoped else {
            return Ok(None);
        };
        let Some(pointer) = pointer else {
            return Ok(None);
        };
        if location != "object" {
            if let Some(text) = &column {
                if !column_matches_ref(text.as_bytes(), &pointer.sha256, pointer.byte_length) {
                    warn!("markdown column differs from the verified ref; serving database text");
                    return Ok(column);
                }
            }
        }
        let bytes = match read_verified(
            client.blobs.as_ref(),
            &key,
            &pointer.sha256,
            pointer.byte_length,
        )
        .await
        {
            Ok(bytes) => bytes,
            Err(err) => {
                return match after_object_checksum(&location, column.is_some(), false) {
                    ObjectServe::Column => {
                        warn!(error = %err, "markdown object read failed; serving database text");
                        Ok(column)
                    }
                    ObjectServe::Refuse | ObjectServe::Object => {
                        let message = if matches!(err, OriginalsError::Checksum(_)) {
                            "markdown object bytes do not match the verified ref".to_string()
                        } else {
                            format!("markdown object read failed: {err}")
                        };
                        Err(StorageError::Unavailable(message))
                    }
                };
            }
        };
        String::from_utf8(bytes)
            .map(Some)
            .map_err(|err| StorageError::Database(format!("markdown object is not utf-8: {err}")))
    }

    async fn pdf_column(&self, pdf_id: Uuid) -> Result<Vec<u8>> {
        sqlx::query_scalar("SELECT pdf_data FROM pdf_document_blobs WHERE pdf_id = $1")
            .bind(pdf_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to fetch PDF bytes: {e}")))
    }

    async fn pdf_chunk_pack(&self, pdf_id: Uuid) -> Result<Option<Vec<u8>>> {
        let value: Option<Option<Vec<u8>>> =
            sqlx::query_scalar("SELECT chunk_sha256 FROM original_object_refs WHERE pdf_id = $1")
                .bind(pdf_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| {
                    StorageError::Database(format!("Failed to read PDF chunk digests: {e}"))
                })?;
        Ok(value.flatten())
    }

    async fn verified_pdf_window(
        &self,
        blob: &dyn BlobStore,
        key: &str,
        total: u64,
        pack: &[u8],
        start: u64,
        end: u64,
    ) -> Result<Option<Vec<u8>>> {
        let covers = chunks_covering(start, end, total, PDF_CHUNK_BYTES)
            .ok_or_else(|| StorageError::Unavailable("PDF range is outside the object".into()))?;
        let mut assembled = Vec::new();
        let mut base = start;
        for (nth, cover) in covers.iter().enumerate() {
            let digest = digest_at(pack, cover.index)
                .ok_or_else(|| StorageError::Unavailable("PDF chunk digest is missing".into()))?;
            let chunk = get_inclusive(blob, key, cover.start, cover.end)
                .await
                .map_err(|err| {
                    StorageError::Unavailable(format!("PDF range read failed: {err}"))
                })?;
            if !chunk_matches(&chunk, digest) {
                return Err(StorageError::Unavailable(
                    "PDF chunk bytes do not match the stored digest".into(),
                ));
            }
            if nth == 0 {
                base = cover.start;
            }
            assembled.extend(chunk);
        }
        let from = (start - base) as usize;
        let to = (end - base) as usize;
        if to >= assembled.len() || from > to {
            return Err(StorageError::Unavailable(
                "PDF range is outside the checked window".into(),
            ));
        }
        Ok(Some(assembled[from..=to].to_vec()))
    }
}

async fn documents_rls_forced(pool: &PgPool) -> bool {
    !super::rls::pool_bypasses_rls(pool).await.unwrap_or(true)
}

async fn active_tenant_ids(pool: &PgPool) -> Result<Vec<Uuid>> {
    sqlx::query_scalar("SELECT tenant_id FROM public.tenants WHERE is_active")
        .fetch_all(pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("Failed to list tenants for document RLS: {e}"))
        })
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
                d.processing_status,
                d.extraction_method,
                d.vision_model,
                d.markdown_content,
                d.markdown_object_id,
                d.markdown_location,
            d.extraction_errors,
            d.created_at,
            d.processed_at,
            d.updated_at,
            b.byte_location AS byte_location
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
        let Some(r) = row else {
            return Ok(None);
        };
        let pdf_id: Uuid = r.get("pdf_id");
        let workspace_id: Uuid = r.get("workspace_id");
        let byte_location: String = r.get("byte_location");
        let pointer = if self.objects.is_some() {
            pdf_pointer(&self.pool, pdf_id).await.map_err(|err| {
                StorageError::Database(format!("Failed to read original object ref: {err}"))
            })?
        } else {
            None
        };
        let scoped_key = if let Some(pointer) = &pointer {
            self.tenant_pdf_key(pdf_id, workspace_id, pointer).await?
        } else {
            None
        };
        let verified = scoped_key.is_some() && pointer.as_ref().is_some_and(|p| p.verified);
        let pdf_data = if prefer_object(self.objects.is_some(), &byte_location, verified) {
            let Some(key) = scoped_key else {
                return Ok(None);
            };
            let Some(pointer) = pointer else {
                return Ok(None);
            };
            let client = self.objects.as_ref().expect("object client");
            if byte_location != "object" {
                let column = self.pdf_column(pdf_id).await?;
                if !column_matches_ref(&column, &pointer.sha256, pointer.byte_length) {
                    warn!("PDF column differs from the verified ref; serving database bytes");
                    column
                } else {
                    match read_verified(
                        client.blobs.as_ref(),
                        &key,
                        &pointer.sha256,
                        pointer.byte_length,
                    )
                    .await
                    {
                        Ok(bytes) => bytes,
                        Err(err) => {
                            warn!(error = %err, "PDF object read failed; serving database bytes");
                            column
                        }
                    }
                }
            } else {
                match read_verified(
                    client.blobs.as_ref(),
                    &key,
                    &pointer.sha256,
                    pointer.byte_length,
                )
                .await
                {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        let message = if matches!(err, OriginalsError::Checksum(_)) {
                            "PDF object bytes do not match the verified ref".to_string()
                        } else {
                            format!("PDF object read failed: {err}")
                        };
                        return Err(StorageError::Unavailable(message));
                    }
                }
            }
        } else {
            self.pdf_column(pdf_id).await?
        };
        Ok(Some(PdfDocument {
            pdf_id,
            workspace_id: r.get("workspace_id"),
            document_id: r.get("document_id"),
            filename: r.get("filename"),
            content_type: r.get("content_type"),
            file_size_bytes: r.get("file_size_bytes"),
            sha256_checksum: r.get("sha256_checksum"),
            page_count: r.get("page_count"),
            pdf_data,
            processing_status: r
                .get::<String, _>("processing_status")
                .parse()
                .unwrap_or(PdfProcessingStatus::Pending),
            extraction_method: r
                .get::<Option<String>, _>("extraction_method")
                .as_ref()
                .and_then(|m| m.parse().ok()),
            vision_model: r.get("vision_model"),
            markdown_content: self
                .markdown_text(
                    workspace_id,
                    r.get("markdown_object_id"),
                    r.get::<String, _>("markdown_location"),
                    r.get("markdown_content"),
                )
                .await?,
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

        if let Some(client) = &self.objects {
            return create_object_pdf(&self.pool, client, request).await;
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
            WITH written AS (
                INSERT INTO pdf_document_blobs (pdf_id, pdf_data, byte_location)
                VALUES ($1, $2, 'database')
                ON CONFLICT (pdf_id) DO UPDATE SET
                    pdf_data = EXCLUDED.pdf_data,
                    byte_location = 'database',
                    updated_at = NOW()
                RETURNING pdf_id
            ),
            dropped AS (
                DELETE FROM original_object_refs r
                USING written w
                WHERE r.pdf_id = w.pdf_id
                RETURNING r.pdf_id
            )
            SELECT pdf_id FROM written
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

    async fn get_pdf_content_view(&self, pdf_id: &Uuid) -> Result<Option<PdfContentView>> {
        let row = sqlx::query(PDF_CONTENT_VIEW_SQL)
            .bind(pdf_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to fetch PDF content: {e}")))?;
        let Some(row) = row else {
            return Ok(None);
        };
        let workspace_id: Uuid = row.get("workspace_id");
        let location: String = row.get("markdown_location");
        let column: Option<String> = row.get("markdown_content");
        let markdown_object_id: Option<Uuid> = row.get("markdown_object_id");
        let markdown = if let Some(owner) = markdown_object_id.filter(|_| self.objects.is_some()) {
            self.markdown_text(workspace_id, owner, location, column)
                .await?
        } else {
            column
        };
        let status = PdfProcessingStatus::from_str(row.get("processing_status"))?;
        Ok(Some(PdfContentView {
            pdf_id: row.get("pdf_id"),
            workspace_id,
            document_id: row.get("document_id"),
            filename: row.get("filename"),
            content_type: row.get("content_type"),
            file_size_bytes: row.get("file_size_bytes"),
            markdown_content: markdown,
            processing_status: status,
        }))
    }

    async fn get_pdf_blob_info(&self, pdf_id: &Uuid) -> Result<Option<PdfBlobInfo>> {
        let row = sqlx::query(
            r#"
            SELECT d.workspace_id, d.filename, d.content_type, b.byte_location, d.file_size_bytes,
                   CASE WHEN b.byte_location = 'database'
                        THEN octet_length(b.pdf_data)::bigint
                        ELSE d.file_size_bytes
                   END AS bytea_len
            FROM pdf_documents d
            INNER JOIN pdf_document_blobs b ON b.pdf_id = d.pdf_id
            WHERE d.pdf_id = $1
            LIMIT 1
            "#,
        )
        .bind(pdf_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to fetch PDF info: {e}")))?;
        let Some(r) = row else {
            return Ok(None);
        };
        let location: String = r.get("byte_location");
        let workspace_id: Uuid = r.get("workspace_id");
        let pointer = if self.objects.is_some() {
            pdf_pointer(&self.pool, *pdf_id).await.map_err(|err| {
                StorageError::Database(format!("Failed to read original object ref: {err}"))
            })?
        } else {
            None
        };
        let scoped = if let Some(pointer) = &pointer {
            self.tenant_pdf_key(*pdf_id, workspace_id, pointer).await?
        } else {
            None
        };
        if location == "object" && scoped.is_none() {
            return Ok(None);
        }
        let verified = scoped.is_some() && pointer.as_ref().is_some_and(|p| p.verified);
        let total = if prefer_object(self.objects.is_some(), &location, verified) {
            pointer
                .map(|p| p.byte_length.max(0) as u64)
                .unwrap_or_else(|| r.get::<i64, _>("file_size_bytes").max(0) as u64)
        } else {
            r.get::<i64, _>("bytea_len").max(0) as u64
        };
        Ok(Some(PdfBlobInfo {
            workspace_id,
            filename: r.get("filename"),
            content_type: r.get("content_type"),
            total_bytes: total,
        }))
    }

    async fn get_pdf_bytes_range(
        &self,
        pdf_id: &Uuid,
        start: u64,
        end: u64,
    ) -> Result<Option<Vec<u8>>> {
        if let Some(client) = &self.objects {
            let row = sqlx::query(
                "SELECT b.byte_location, d.workspace_id \
                 FROM pdf_document_blobs b \
                 JOIN pdf_documents d ON d.pdf_id = b.pdf_id \
                 WHERE b.pdf_id = $1",
            )
            .bind(pdf_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to read PDF location: {e}")))?;
            let Some(row) = row else {
                return Ok(None);
            };
            let location: String = row.get("byte_location");
            let workspace_id: Uuid = row.get("workspace_id");
            let pointer = pdf_pointer(&self.pool, *pdf_id).await.map_err(|err| {
                StorageError::Database(format!("Failed to read original object ref: {err}"))
            })?;
            let scoped = if let Some(pointer) = &pointer {
                self.tenant_pdf_key(*pdf_id, workspace_id, pointer).await?
            } else {
                None
            };
            let verified = scoped.is_some() && pointer.as_ref().is_some_and(|p| p.verified);
            // Dual-copy rows keep the column. A range cannot prove the object
            // hash, so only `byte_location = object` reads a slice from the bucket.
            if location == "object" {
                if !verified {
                    return Ok(None);
                }
                let Some(key) = scoped else {
                    return Ok(None);
                };
                let Some(pointer) = pointer else {
                    return Ok(None);
                };
                let whole = pointer.byte_length > 0
                    && start == 0
                    && u64::try_from(pointer.byte_length).ok() == Some(end.saturating_add(1));
                if whole {
                    let bytes = read_verified(
                        client.blobs.as_ref(),
                        &key,
                        &pointer.sha256,
                        pointer.byte_length,
                    )
                    .await
                    .map_err(|err| {
                        StorageError::Unavailable(format!("PDF object read failed: {err}"))
                    })?;
                    return Ok(Some(bytes));
                }
                if let Some(pack) = self.pdf_chunk_pack(*pdf_id).await? {
                    if !pack.is_empty() {
                        return self
                            .verified_pdf_window(
                                client.blobs.as_ref(),
                                &key,
                                pointer.byte_length.max(0) as u64,
                                &pack,
                                start,
                                end,
                            )
                            .await;
                    }
                }
                let bytes = get_inclusive(client.blobs.as_ref(), &key, start, end)
                    .await
                    .map_err(|err| {
                        StorageError::Unavailable(format!("PDF range read failed: {err}"))
                    })?;
                return Ok(Some(bytes));
            }
        }
        // `substring(bytea FROM n FOR len)` is 1-based and runs inside Postgres,
        // so only the requested window crosses the wire.
        let from = i32::try_from(start + 1)
            .map_err(|_| StorageError::Database("PDF range start out of bounds".into()))?;
        let len = i32::try_from(end - start + 1)
            .map_err(|_| StorageError::Database("PDF range length out of bounds".into()))?;
        let row = sqlx::query(
            "SELECT substring(pdf_data FROM $2 FOR $3) AS chunk \
             FROM pdf_document_blobs WHERE pdf_id = $1",
        )
        .bind(pdf_id)
        .bind(from)
        .bind(len)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to fetch PDF range: {e}")))?;
        Ok(row.map(|r| r.get::<Vec<u8>, _>("chunk")))
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

        if let Some(markdown) = request
            .markdown_content
            .as_deref()
            .filter(|text| !text.is_empty())
        {
            if let Some(client) = &self.objects {
                return super::pdf_markdown_object::store_markdown(
                    &self.pool, client, &request, markdown,
                )
                .await;
            }
        }

        let wrote_markdown = request
            .markdown_content
            .as_deref()
            .is_some_and(|text| !text.is_empty());
        let mut tx = self.pool.begin().await.map_err(|e| {
            StorageError::Database(format!("Failed to begin PDF processing update: {e}"))
        })?;
        sqlx::query(
            r#"
            UPDATE pdf_documents
            SET processing_status = $1,
                extraction_method = COALESCE($2, extraction_method),
                markdown_content = COALESCE($3, markdown_content),
                markdown_location = CASE
                    WHEN $3 IS NOT NULL AND $3 <> '' THEN 'database'
                    ELSE markdown_location
                END,
                extraction_errors = COALESCE($4, extraction_errors),
                document_id = COALESCE($5, document_id),
                processed_at = COALESCE($6, processed_at),
                vision_model = COALESCE($8, vision_model)
            WHERE pdf_id = $7
            "#,
        )
        .bind(status_str)
        .bind(&method_str)
        .bind(&request.markdown_content)
        .bind(&request.extraction_errors)
        .bind(request.document_id)
        .bind(processed_at)
        .bind(request.pdf_id)
        .bind(&request.vision_model)
        .execute(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to update PDF processing: {e}")))?;
        if wrote_markdown {
            sqlx::query(
                "DELETE FROM original_object_refs r
                 USING pdf_documents d
                 WHERE d.pdf_id = $1 AND r.markdown_object_id = d.markdown_object_id",
            )
            .bind(request.pdf_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to drop markdown ref: {e}")))?;
        }
        tx.commit().await.map_err(|e| {
            StorageError::Database(format!("Failed to commit PDF processing update: {e}"))
        })?;

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
        if page == 0 || page_size == 0 {
            return Err(StorageError::InvalidInput(
                "PDF page and page size must be positive".into(),
            ));
        }
        let limit = i64::try_from(page_size)
            .map_err(|_| StorageError::InvalidInput("PDF page size exceeds i64".into()))?;
        let fetch_limit = limit
            .checked_add(1)
            .ok_or_else(|| StorageError::InvalidInput("PDF page size exceeds i64".into()))?;
        let offset = if filter.keyset_cursor().is_some() {
            0
        } else {
            (page - 1)
                .checked_mul(page_size)
                .and_then(|n| i64::try_from(n).ok())
                .ok_or_else(|| StorageError::InvalidInput("PDF page offset exceeds i64".into()))?
        };

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

        // Get paginated items using helper. One extra row tells `has_more`
        // without a second scan of the skipped keys.
        let mut items =
            super::pdf_list_query::list_pdfs_dynamic(&self.pool, &filter, fetch_limit, offset)
                .await?;
        let has_more = items.len() > page_size;
        items.truncate(page_size);

        Ok(PdfList {
            items,
            total_count,
            page,
            page_size,
            has_more,
        })
    }

    async fn delete_pdf(&self, pdf_id: &Uuid) -> Result<()> {
        let located = if self.objects.is_some() {
            let row = sqlx::query("SELECT d.workspace_id FROM pdf_documents d WHERE d.pdf_id = $1")
                .bind(pdf_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| {
                    StorageError::Database(format!("Failed to read PDF workspace: {e}"))
                })?;
            if let Some(row) = row {
                let workspace_id: Uuid = row.get("workspace_id");
                let pointer = pdf_pointer(&self.pool, *pdf_id).await.map_err(|err| {
                    StorageError::Database(format!("Failed to read original object ref: {err}"))
                })?;
                if let Some(pointer) = pointer {
                    self.tenant_pdf_key(*pdf_id, workspace_id, &pointer).await?
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
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

        if let (Some(client), Some(key)) = (&self.objects, located) {
            match client.blobs.delete_key(&key).await {
                Ok(()) => {
                    let _ = remove_delete_queue(&self.pool, &key).await;
                }
                Err(err) => {
                    warn!(
                        pdf_id = %pdf_id,
                        error = %err,
                        "object delete failed; original_object_deletes keeps the key for the reaper"
                    );
                }
            }
        }

        debug!("Deleted PDF: id={}", pdf_id);

        Ok(())
    }

    async fn clear_markdown(&self, pdf_id: &Uuid) -> Result<()> {
        // WHY: Explicitly NULL out markdown_content + extraction metadata so the
        // pdf_processing resume shortcut cannot reuse a stale conversion. This is
        // the only way to force a true PDF -> markdown re-conversion, because
        // `update_pdf_processing` uses COALESCE and never clears these columns.
        // The ref is removed first. The bucket is called only for a key that
        // names this workspace, and only when the originals client exists.
        let key = super::pdf_markdown_object::take_markdown_key(
            &self.pool,
            self.objects.as_deref(),
            *pdf_id,
        )
        .await?;
        if let (Some(client), Some(key)) = (&self.objects, key) {
            super::pdf_markdown_object::delete_markdown_key(&self.pool, client, &key).await;
        }
        sqlx::query(
            r#"
            UPDATE pdf_documents
            SET markdown_content = NULL,
                markdown_location = 'database',
                extraction_method = NULL,
                extraction_errors = NULL
            WHERE pdf_id = $1
            "#,
        )
        .bind(pdf_id)
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
        if tenant_id.is_some() && documents_rls_forced(&self.pool).await {
            let tenant = *tenant_id.expect("tenant checked above");
            let workspace = *workspace_id;
            let document_id = *document_id;
            let title = title.to_string();
            let content = content.to_string();
            let pg_status = pg_status.clone();
            return super::rls::with_rls_transaction(
                &self.pool,
                tenant,
                Some(workspace),
                None,
                move |conn| {
                    Box::pin(async move {
                        sqlx::query(
                            r#"
                            INSERT INTO public.documents (id, tenant_id, workspace_id, title, content, status, updated_at)
                            SELECT $1, $2, $3, $4, $5, $6, NOW()
                            FROM (
                                SELECT set_config('app.current_tenant_id', COALESCE($2::text, ''), true),
                                       set_config('app.current_workspace_id', $3::text, true)
                            ) ctx
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
                        .bind(tenant)
                        .bind(workspace)
                        .bind(title)
                        .bind(content)
                        .bind(pg_status)
                        .execute(&mut *conn)
                        .await
                        .map_err(|e| {
                            StorageError::Database(format!("Failed to ensure document record: {e}"))
                        })?;
                        Ok(())
                    })
                },
            )
            .await;
        }
        sqlx::query(
            r#"
            INSERT INTO public.documents (id, tenant_id, workspace_id, title, content, status, updated_at)
            SELECT $1, $2, $3, $4, $5, $6, NOW()
            FROM (
                SELECT set_config('app.current_tenant_id', COALESCE($2::text, ''), true),
                       set_config('app.current_workspace_id', $3::text, true)
            ) ctx
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

        if documents_rls_forced(&self.pool).await {
            let tenants = active_tenant_ids(&self.pool).await?;
            for tenant in tenants {
                let status_label = pg_status.clone();
                let pg_status = pg_status.clone();
                let error_message = stats.error_message.map(str::to_string);
                let document_id = stats.document_id;
                let chunk_count = chunk_count;
                let entity_count = entity_count;
                let relationship_count = relationship_count;
                let cost_usd = stats.cost_usd;
                let input_tokens = stats.input_tokens;
                let output_tokens = stats.output_tokens;
                let total_tokens = stats.total_tokens;
                let affected =
                    super::rls::with_rls_transaction(&self.pool, tenant, None, None, move |conn| {
                        Box::pin(async move {
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
                            .bind(document_id)
                            .bind(chunk_count)
                            .bind(entity_count)
                            .bind(relationship_count)
                            .bind(cost_usd)
                            .bind(input_tokens)
                            .bind(output_tokens)
                            .bind(total_tokens)
                            .bind(error_message)
                            .bind(pg_status)
                            .execute(&mut *conn)
                            .await
                            .map_err(|e| {
                                StorageError::Database(format!(
                                    "Failed to update document stats: {e}"
                                ))
                            })?;
                            Ok(result.rows_affected())
                        })
                    })
                    .await?;
                if affected > 0 {
                    debug!(
                        document_id = %document_id,
                        chunk_count,
                        entity_count,
                        relationship_count,
                        status = %status_label,
                        "Updated document stats (SPEC-021 P-A1)"
                    );
                    return Ok(());
                }
            }
            warn!(
                document_id = %stats.document_id,
                "update_document_stats: documents row not found (race with ensure_document_record) — stats not persisted; will converge on next refresh"
            );
            return Ok(());
        }

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
        if documents_rls_forced(&self.pool).await {
            let tenants = active_tenant_ids(&self.pool).await?;
            let document_id = *document_id;
            for tenant in tenants {
                let pg_status = pg_status.clone();
                let affected =
                    super::rls::with_rls_transaction(&self.pool, tenant, None, None, move |conn| {
                        Box::pin(async move {
                            let result = sqlx::query(
                                r#"
                                UPDATE public.documents SET
                                    status     = $2,
                                    updated_at = NOW()
                                WHERE id = $1
                                "#,
                            )
                            .bind(document_id)
                            .bind(pg_status)
                            .execute(&mut *conn)
                            .await
                            .map_err(|e| {
                                StorageError::Database(format!(
                                    "Failed to touch document status: {e}"
                                ))
                            })?;
                            Ok(result.rows_affected())
                        })
                    })
                    .await?;
                if affected > 0 {
                    return Ok(());
                }
            }
            warn!(
                document_id = %document_id,
                "touch_document_status: no documents row yet (non-fatal)"
            );
            return Ok(());
        }
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
        if documents_rls_forced(&self.pool).await {
            let tenants = active_tenant_ids(&self.pool).await?;
            let document_id = *document_id;
            for tenant in tenants {
                let affected =
                    super::rls::with_rls_transaction(&self.pool, tenant, None, None, move |conn| {
                        Box::pin(async move {
                            let result = sqlx::query("DELETE FROM public.documents WHERE id = $1")
                                .bind(document_id)
                                .execute(&mut *conn)
                                .await
                                .map_err(|e| {
                                    StorageError::Database(format!(
                                        "Failed to delete document record: {e}"
                                    ))
                                })?;
                            Ok(result.rows_affected())
                        })
                    })
                    .await?;
                if affected > 0 {
                    return Ok(());
                }
            }
        }
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
