//! Helper for building PDF list queries dynamically

use crate::{
    pdf_storage::{ListPdfFilter, PdfDocument},
    StorageError,
};
use sqlx::{PgPool, Row};

pub async fn list_pdfs_dynamic(
    pool: &PgPool,
    filter: &ListPdfFilter,
    limit: i64,
    offset: i64,
) -> Result<Vec<PdfDocument>, StorageError> {
    // Build dynamic query based on filters
    let mut query_parts = Vec::new();
    let mut param_idx = 1;

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

    query_parts.push(format!(
        " ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
        param_idx,
        param_idx + 1
    ));

    let query_str = query_parts.join("");
    let mut query = sqlx::query(&query_str);

    if let Some(workspace_id) = filter.workspace_id {
        query = query.bind(workspace_id);
    }
    if filter.processing_status.is_some() {
        query = query.bind(&status_str);
    }
    query = query.bind(limit).bind(offset);

    let rows = query.fetch_all(pool).await?;

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
                pdf_data: r.try_get("pdf_data")?,
                processing_status: {
                    let status_str: String = r.try_get("processing_status")?;
                    status_str.parse().unwrap()
                },
                extraction_method: {
                    let method_opt: Option<String> = r.try_get("extraction_method")?;
                    method_opt.and_then(|m| m.parse().ok())
                },
                vision_model: r.try_get("vision_model")?,
                markdown_content: r.try_get("markdown_content")?,
                extraction_errors: r.try_get("extraction_errors")?,
                created_at: r.try_get("created_at")?,
                processed_at: r.try_get("processed_at")?,
                updated_at: r.try_get("updated_at")?,
            })
        })
        .collect::<Result<Vec<_>, StorageError>>()?;

    Ok(items)
}
