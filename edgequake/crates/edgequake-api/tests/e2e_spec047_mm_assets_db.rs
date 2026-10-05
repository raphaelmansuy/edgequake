//! SPEC-047 — durable mm-assets: persist on ingest, get by id, delete with document.

mod common;

#[cfg(feature = "postgres")]
use edgequake_api::services::{
    delete_document_mm_assets, list_mm_asset_summaries_for_document, load_mm_asset_bytes,
    load_mm_asset_bytes_by_id, materialize_mm_assets_to_dir, persist_document_mm_assets_from_dir,
};
#[cfg(feature = "postgres")]
use edgequake_storage::{asset_id_from_path, ASSET_KIND_PAGE_CHART_CROP};
use edgequake_storage::{DocumentMmAssetStorage, MemoryMmAssetStorage, ASSET_KIND_PAGE_FULL};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
#[cfg(feature = "postgres")]
async fn mm_assets_persist_roundtrip_with_page_lineage() {
    let dir = tempfile::tempdir().expect("temp");
    let assets = dir.path().join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(assets.join("page-0002.png"), b"\x89PNG full").unwrap();
    std::fs::write(assets.join("page-0002-chart.png"), b"\x89PNG crop").unwrap();

    let storage = Arc::new(MemoryMmAssetStorage::new());
    let doc = Uuid::new_v4();
    let ws = Uuid::new_v4();

    let n = persist_document_mm_assets_from_dir(storage.as_ref(), doc, ws, dir.path())
        .await
        .expect("persist");
    assert_eq!(n, 2);

    let summaries =
        list_mm_asset_summaries_for_document(Some(storage.as_ref()), &doc.to_string(), ws)
            .await
            .expect("summaries");
    assert_eq!(summaries.len(), 2);
    assert!(summaries.iter().all(|s| s.page_num == Some(2)));
    assert!(summaries
        .iter()
        .any(|s| s.asset_kind == ASSET_KIND_PAGE_FULL && s.asset_id == "page-0002"));
    assert!(summaries.iter().any(|s| {
        s.asset_kind == ASSET_KIND_PAGE_CHART_CROP && s.asset_id == "page-0002-chart"
    }));

    let (bytes, ct) = load_mm_asset_bytes(
        Some(storage.as_ref()),
        &doc.to_string(),
        Some(ws),
        "assets/page-0002-chart.png",
    )
    .await
    .expect("load");
    assert_eq!(ct, "image/png");
    assert!(bytes.starts_with(b"\x89PNG"));
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn mm_assets_get_by_stable_id() {
    let dir = tempfile::tempdir().expect("temp");
    let assets = dir.path().join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(assets.join("page-0001.png"), b"\x89PNG by-id").unwrap();
    std::fs::write(assets.join("page-0001-chart.png"), b"\x89PNG chart-id").unwrap();

    let storage = Arc::new(MemoryMmAssetStorage::new());
    let doc = Uuid::new_v4();
    let ws = Uuid::new_v4();
    persist_document_mm_assets_from_dir(storage.as_ref(), doc, ws, dir.path())
        .await
        .expect("persist");

    assert_eq!(
        asset_id_from_path("assets/page-0001-chart.png"),
        "page-0001-chart"
    );

    let (bytes, ct) = load_mm_asset_bytes_by_id(
        Some(storage.as_ref()),
        &doc.to_string(),
        Some(ws),
        "page-0001-chart",
    )
    .await
    .expect("load by id");
    assert_eq!(ct, "image/png");
    assert_eq!(bytes, b"\x89PNG chart-id");

    let (full, _) = load_mm_asset_bytes_by_id(
        Some(storage.as_ref()),
        &doc.to_string(),
        Some(ws),
        "page-0001",
    )
    .await
    .expect("load full page by id");
    assert_eq!(full, b"\x89PNG by-id");
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn mm_assets_materialize_restores_disk_cache_for_analyze() {
    let storage = Arc::new(MemoryMmAssetStorage::new());
    let doc = Uuid::new_v4();
    let ws = Uuid::new_v4();

    storage
        .store_asset(edgequake_storage::StoreMmAssetRequest {
            document_id: doc,
            workspace_id: ws,
            asset_id: "page-0001".into(),
            asset_path: "assets/page-0001.png".into(),
            content_type: "image/png".into(),
            asset_data: b"\x89PNG from-db".to_vec(),
            asset_kind: ASSET_KIND_PAGE_FULL.into(),
            page_num: Some(1),
        })
        .await
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let written = materialize_mm_assets_to_dir(storage.as_ref(), doc, ws, dir.path())
        .await
        .expect("materialize");
    assert_eq!(written, 1);
    let on_disk = std::fs::read(dir.path().join("assets/page-0001.png")).unwrap();
    assert_eq!(on_disk, b"\x89PNG from-db");

    let again = materialize_mm_assets_to_dir(storage.as_ref(), doc, ws, dir.path())
        .await
        .unwrap();
    assert_eq!(again, 0);
}

#[tokio::test]
#[cfg(feature = "postgres")]
async fn delete_document_removes_assets_db_and_fs() {
    let dir = tempfile::tempdir().expect("temp");
    let assets = dir.path().join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(assets.join("page-0003.png"), b"\x89PNG").unwrap();

    let storage = Arc::new(MemoryMmAssetStorage::new());
    let doc = Uuid::new_v4();
    let ws = Uuid::new_v4();
    persist_document_mm_assets_from_dir(storage.as_ref(), doc, ws, dir.path())
        .await
        .unwrap();

    // Simulate document-scoped FS cache under document_mm_assets_root.
    let cache_root = edgequake_api::services::document_mm_assets_root(&doc.to_string());
    std::fs::create_dir_all(cache_root.join("assets")).unwrap();
    std::fs::write(cache_root.join("assets/page-0003.png"), b"\x89PNG").unwrap();

    let deleted = delete_document_mm_assets(Some(storage.as_ref()), &doc.to_string(), Some(ws))
        .await
        .expect("delete");
    assert_eq!(deleted, 1);
    assert!(storage
        .list_asset_summaries(&ws, &doc)
        .await
        .unwrap()
        .is_empty());
    assert!(
        !cache_root.exists(),
        "filesystem mm-assets cache must be removed"
    );

    let err = load_mm_asset_bytes_by_id(
        Some(storage.as_ref()),
        &doc.to_string(),
        Some(ws),
        "page-0003",
    )
    .await;
    assert!(err.is_err(), "get-by-id after delete must fail");
}

#[tokio::test]
async fn cascade_delete_removes_assets_keeping_lineage_integrity() {
    let storage = Arc::new(MemoryMmAssetStorage::new());
    let doc = Uuid::new_v4();
    let ws = Uuid::new_v4();
    storage
        .store_asset(edgequake_storage::StoreMmAssetRequest {
            document_id: doc,
            workspace_id: ws,
            asset_id: "page-0003".into(),
            asset_path: "assets/page-0003.png".into(),
            content_type: "image/png".into(),
            asset_data: b"\x89PNG".to_vec(),
            asset_kind: ASSET_KIND_PAGE_FULL.into(),
            page_num: Some(3),
        })
        .await
        .unwrap();

    let deleted = storage.delete_assets_for_document(&ws, &doc).await.unwrap();
    assert_eq!(deleted, 1);
    assert!(storage
        .list_asset_summaries(&ws, &doc)
        .await
        .unwrap()
        .is_empty());
}
