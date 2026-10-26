//! Helper for building PDF list queries dynamically

use crate::{
    pdf_storage::{ListPdfFilter, PdfDocument},
    StorageError,
};
use sqlx::{Executor, Postgres, Row};

pub async fn list_pdfs_dynamic<'c>(
    conn: impl Executor<'c, Database = Postgres>,
    filter: &ListPdfFilter,
    limit: i64,
    offset: i64,
) -> Result<Vec<PdfDocument>, StorageError> {
    // Build dynamic query based on filters
    let mut query_parts = Vec::new();
    let mut param_idx = 1;

    // SPEC-090 F-090-16 / LAW-P7: list projection is metadata-only.
    // `pdf_data` and `markdown_content` are fetched on the by-id path only.
    let base_query = r#"
        SELECT
            pdf_id,
            workspace_id,
            document_id,
            filename,
            content_type,
            file_size_bytes,
            sha256_checksum,
            page_count,
            processing_status,
            extraction_method,
            vision_model,
            extraction_errors,
            created_at,
            processed_at,
            updated_at
        FROM pdf_documents
        WHERE 1=1
    "#;

    query_parts.push(base_query.to_string());

    if let Some(_workspace_id) = filter.workspace_id {
        query_parts.push(format!(" AND workspace_id = ${}", param_idx));
        param_idx += 1;
    }

    let status_str;
    if let Some(ref status) = filter.processing_status {
        status_str = status.to_string();
        query_parts.push(format!(" AND processing_status = ${}", param_idx));
        param_idx += 1;
    } else {
        status_str = String::new(); // Initialize to avoid uninitialized error
    }

    // A passed cursor is a required bound. `created_at <= $n` lets a generic
    // plan start at `(workspace_id, created_at)` instead of the global
    // `created_at` index. The row comparison then stops inside a tie.
    // Page numbers keep OFFSET. No scope and no cursor stays unscoped.
    let keyset = filter.keyset_cursor();
    if keyset.is_some() {
        query_parts.push(format!(
            " AND created_at <= ${at} AND (created_at, pdf_id) < (${at}, ${id})",
            at = param_idx,
            id = param_idx + 1
        ));
        param_idx += 2;
    }

    query_parts.push(format!(
        " ORDER BY created_at DESC, pdf_id DESC LIMIT ${}",
        param_idx
    ));
    if keyset.is_none() {
        query_parts.push(format!(" OFFSET ${}", param_idx + 1));
    }

    let query_str = query_parts.join("");
    let mut query = sqlx::query(&query_str);

    if let Some(workspace_id) = filter.workspace_id {
        query = query.bind(workspace_id);
    }
    if filter.processing_status.is_some() {
        query = query.bind(&status_str);
    }
    if let Some((at, id)) = keyset {
        query = query.bind(at).bind(id);
    }
    query = query.bind(limit);
    if keyset.is_none() {
        query = query.bind(offset);
    }

    let rows = query.fetch_all(conn).await?;

    let items = rows
        .into_iter()
        .map(|r| {
            Ok::<PdfDocument, StorageError>(PdfDocument {
                pdf_id: r.try_get("pdf_id")?,
                workspace_id: r.try_get("workspace_id")?,
                document_id: r.try_get("document_id")?,
                filename: r.try_get("filename")?,
                content_type: r.try_get("content_type")?,
                file_size_bytes: r.try_get("file_size_bytes")?,
                sha256_checksum: r.try_get("sha256_checksum")?,
                page_count: r.try_get("page_count")?,
                pdf_data: Vec::new(),
                processing_status: {
                    let status_str: String = r.try_get("processing_status")?;
                    status_str.parse()?
                },
                extraction_method: {
                    let method_opt: Option<String> = r.try_get("extraction_method")?;
                    method_opt.and_then(|m| m.parse().ok())
                },
                vision_model: r.try_get("vision_model")?,
                markdown_content: None,
                extraction_errors: r.try_get("extraction_errors")?,
                created_at: r.try_get("created_at")?,
                processed_at: r.try_get("processed_at")?,
                updated_at: r.try_get("updated_at")?,
            })
        })
        .collect::<Result<Vec<_>, StorageError>>()?;

    Ok(items)
}
