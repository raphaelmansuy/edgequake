//! Object-mode PDF create. The BYTEA insert stays in `create_pdf`.

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Result, StorageError};
use crate::originals::{
    confirm_put, delete_loser, insert_inflight, put_bytes, remove_inflight, upload_key,
    workspace_tenant, OriginalsClient, OwnerKind,
};
use crate::pdf_storage::{calculate_pdf_checksum, CreatePdfRequest, PdfProcessingStatus};

pub(super) fn is_unique_violation(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db) => db.code().as_deref() == Some("23505"),
        _ => false,
    }
}

fn map_store(err: impl std::fmt::Display) -> StorageError {
    StorageError::Unavailable(err.to_string())
}

/// Inflight commit, PUT, then the pointer. A checksum conflict deletes the loser object.
pub(super) async fn create_object_pdf(
    pool: &PgPool,
    client: &OriginalsClient,
    request: CreatePdfRequest,
) -> Result<Uuid> {
    let sha = calculate_pdf_checksum(&request.pdf_data);
    let pdf_id = Uuid::new_v4();
    let tenant_id = workspace_tenant(pool, request.workspace_id)
        .await
        .map_err(map_store)?;
    let key = upload_key(
        &client.prefix,
        OwnerKind::Pdf,
        tenant_id,
        request.workspace_id,
        pdf_id,
    );
    insert_inflight(pool, &key).await.map_err(map_store)?;
    let meta = match put_bytes(client.blobs.as_ref(), &key, &request.pdf_data, client.sse).await {
        Ok(meta) => meta,
        Err(err) => {
            let _ = remove_inflight(pool, &key).await;
            return Err(map_store(err));
        }
    };
    if let Err(err) = confirm_put(client.blobs.as_ref(), &key, &request.pdf_data).await {
        let _ = remove_inflight(pool, &key).await;
        let _ = delete_loser(client.blobs.as_ref(), &key).await;
        return Err(map_store(err));
    }

    let mut tx = pool.begin().await.map_err(|e| {
        StorageError::Database(format!("Failed to begin PDF create transaction: {e}"))
    })?;
    if let Some(session) = crate::clearance_session::ClearanceSession::current() {
        super::rls::apply_clearance_session(&mut tx, &session).await?;
    }
    let inserted = sqlx::query(
        r#"
        INSERT INTO pdf_documents (
            pdf_id, workspace_id, filename, content_type, file_size_bytes,
            sha256_checksum, page_count, processing_status, vision_model
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(pdf_id)
    .bind(request.workspace_id)
    .bind(&request.filename)
    .bind(&request.content_type)
    .bind(request.pdf_data.len() as i64)
    .bind(&sha)
    .bind(request.page_count)
    .bind(PdfProcessingStatus::Pending.as_str())
    .bind(&request.vision_model)
    .execute(&mut *tx)
    .await;

    if let Err(err) = inserted {
        drop(tx);
        let _ = delete_loser(client.blobs.as_ref(), &key).await;
        let _ = remove_inflight(pool, &key).await;
        if is_unique_violation(&err) {
            return Err(StorageError::Conflict(format!(
                "PDF with checksum {sha} already exists in this workspace (concurrent upload detected)"
            )));
        }
        return Err(StorageError::Database(format!(
            "Failed to create PDF document: {err}"
        )));
    }

    if let Err(err) = sqlx::query(
        r#"
        INSERT INTO pdf_document_blobs (pdf_id, pdf_data, byte_location)
        VALUES ($1, NULL, 'object')
        "#,
    )
    .bind(pdf_id)
    .execute(&mut *tx)
    .await
    {
        drop(tx);
        let _ = delete_loser(client.blobs.as_ref(), &key).await;
        let _ = remove_inflight(pool, &key).await;
        return Err(StorageError::Database(format!(
            "Failed to write pdf_document_blobs for {pdf_id}: {err}"
        )));
    }

    if let Err(err) = sqlx::query(
        r#"
        INSERT INTO original_object_refs (
            pdf_id, tenant_id, workspace_id, object_key, sha256, byte_length,
            content_type, etag, verified_at, chunk_sha256
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now(), $9)
        "#,
    )
    .bind(pdf_id)
    .bind(tenant_id)
    .bind(request.workspace_id)
    .bind(&key)
    .bind(&sha)
    .bind(request.pdf_data.len() as i64)
    .bind(&request.content_type)
    .bind(&meta.etag)
    .bind(crate::originals::pack_digests(
        &crate::originals::chunk_digests(&request.pdf_data, crate::originals::PDF_CHUNK_BYTES),
    ))
    .execute(&mut *tx)
    .await
    {
        drop(tx);
        let _ = delete_loser(client.blobs.as_ref(), &key).await;
        let _ = remove_inflight(pool, &key).await;
        if is_unique_violation(&err) {
            return Err(StorageError::Conflict(format!(
                "PDF with checksum {sha} already exists in this workspace (concurrent upload detected)"
            )));
        }
        return Err(StorageError::Database(format!(
            "Failed to write original_object_refs for {pdf_id}: {err}"
        )));
    }

    if let Err(err) = sqlx::query("DELETE FROM original_object_inflight WHERE object_key = $1")
        .bind(&key)
        .execute(&mut *tx)
        .await
    {
        drop(tx);
        return Err(StorageError::Database(format!(
            "Failed to clear inflight for {pdf_id}: {err}"
        )));
    }

    if let Err(err) = tx.commit().await {
        let _ = delete_loser(client.blobs.as_ref(), &key).await;
        let _ = remove_inflight(pool, &key).await;
        return Err(StorageError::Database(format!(
            "Failed to commit PDF create transaction: {err}"
        )));
    }
    Ok(pdf_id)
}
