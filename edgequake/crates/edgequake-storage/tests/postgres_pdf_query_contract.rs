//! Actual PDF adapter query branches on a session-local metadata fixture.
#![cfg(feature = "postgres")]
#[path = "support/postgres_access_pool.rs"]
#[allow(dead_code)]
mod postgres_access_pool;

use edgequake_storage::pdf_storage::{ListPdfFilter, PdfDocumentStorage, PdfProcessingStatus};
use edgequake_storage::{PostgresPdfStorage, StorageError};
use std::time::Duration;
use uuid::Uuid;

async fn fixture() -> Option<(sqlx::PgPool, Uuid, Uuid)> {
    let pool =
        postgres_access_pool::test_pool_with_acquire_timeout(Duration::from_secs(10)).await?;
    // LIKE copies column types and defaults; omit CHECK/FK constraints so a
    // corrupt status can exercise decoding without altering the real table.
    sqlx::query("CREATE TEMP TABLE pdf_documents (LIKE public.pdf_documents INCLUDING DEFAULTS)")
        .execute(&pool)
        .await
        .unwrap();
    let (first, second) = (Uuid::new_v4(), Uuid::new_v4());
    for (workspace, status, seconds) in [
        (first, "pending", 3),
        (first, "completed", 2),
        (second, "completed", 1),
    ] {
        sqlx::query("INSERT INTO pdf_documents(pdf_id,workspace_id,filename,content_type,file_size_bytes,sha256_checksum,processing_status,created_at) VALUES($1,$2,'quoted\"file.pdf','application/pdf',1024,$3,$4,now()-make_interval(secs=>$5))")
            .bind(Uuid::new_v4()).bind(workspace).bind(Uuid::new_v4().to_string()).bind(status).bind(seconds as f64)
            .execute(&pool).await.unwrap();
    }
    Some((pool, first, second))
}

#[tokio::test]
async fn pdf_list_filters_bind_all_combinations_and_paginate_metadata_only() {
    let Some((pool, workspace, _)) = fixture().await else {
        return;
    };
    let storage = PostgresPdfStorage::new(pool.clone());
    for (scope, status, expected) in [
        (None, None, 3),
        (Some(workspace), None, 2),
        (None, Some(PdfProcessingStatus::Completed), 2),
        (Some(workspace), Some(PdfProcessingStatus::Completed), 1),
    ] {
        let result = storage
            .list_pdfs(ListPdfFilter {
                workspace_id: scope,
                processing_status: status,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(result.total_count, expected);
        assert_eq!(result.items.len(), expected as usize);
        assert!(result
            .items
            .iter()
            .all(|p| p.pdf_data.is_empty() && p.markdown_content.is_none()));
    }
    let first = storage
        .list_pdfs(ListPdfFilter {
            page: Some(1),
            page_size: Some(1),
            ..Default::default()
        })
        .await
        .unwrap();
    let second = storage
        .list_pdfs(ListPdfFilter {
            page: Some(2),
            page_size: Some(1),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(first.total_count, 3);
    assert_ne!(first.items[0].pdf_id, second.items[0].pdf_id);
    assert!(first.items[0].created_at > second.items[0].created_at);
    let missing = storage
        .list_pdfs(ListPdfFilter {
            workspace_id: Some(Uuid::new_v4()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(missing.total_count, 0);
    assert!(missing.items.is_empty());
    for (page, size) in [(0, 20), (1, 0), (1, usize::MAX), (usize::MAX, 20)] {
        assert!(matches!(
            storage
                .list_pdfs(ListPdfFilter {
                    page: Some(page),
                    page_size: Some(size),
                    ..Default::default()
                })
                .await,
            Err(StorageError::InvalidInput(_))
        ));
    }
    pool.close().await;
}

#[tokio::test]
async fn malformed_pdf_status_returns_invalid_data_without_panicking() {
    let Some((pool, workspace, _)) = fixture().await else {
        return;
    };
    sqlx::query("UPDATE pdf_documents SET processing_status='corrupt' WHERE workspace_id=$1")
        .bind(workspace)
        .execute(&pool)
        .await
        .unwrap();
    let result = PostgresPdfStorage::new(pool.clone())
        .list_pdfs(ListPdfFilter {
            workspace_id: Some(workspace),
            ..Default::default()
        })
        .await;
    assert!(matches!(result, Err(StorageError::InvalidData(_))));
    pool.close().await;
}
