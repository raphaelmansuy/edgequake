//! SPEC-024 pass 12 — ingestion path uniformity contracts.

#[test]
fn contract_duplicate_reingest_ssot_helper() {
    let helpers = include_str!("../src/handlers/documents/storage_helpers.rs");
    assert!(
        helpers.contains("resolve_workspace_duplicate_for_reingestion"),
        "duplicate handling must be SSOT in storage_helpers"
    );
}

#[test]
fn contract_all_upload_handlers_use_duplicate_ssot() {
    let text = include_str!("../src/handlers/documents/upload/text_upload.rs");
    let file = include_str!("../src/handlers/documents/upload/file_upload.rs");
    let batch = include_str!("../src/handlers/documents/upload/batch_upload.rs");

    for (name, src) in [
        ("text_upload", text),
        ("file_upload", file),
        ("batch_upload", batch),
    ] {
        assert!(
            src.contains("resolve_workspace_duplicate_for_reingestion"),
            "{name} must use duplicate re-ingest SSOT"
        );
    }
}

#[test]
fn contract_upload_handlers_enqueue_insert_tasks() {
    let text = include_str!("../src/handlers/documents/upload/text_upload.rs");
    let file = include_str!("../src/handlers/documents/upload/file_upload.rs");
    let batch = include_str!("../src/handlers/documents/upload/batch_upload.rs");
    let injection = include_str!("../src/handlers/injection/crud.rs");

    assert!(text.contains("enqueue_task"));
    assert!(text.contains("TaskType::Insert"));
    assert!(file.contains("enqueue_task"));
    assert!(batch.contains("enqueue_task"));
    assert!(
        injection.contains("enqueue_task"),
        "injection must use worker queue"
    );
}

#[test]
fn contract_health_exposes_ingestion_and_storage_snapshots() {
    let health = include_str!("../src/handlers/health.rs");
    assert!(health.contains("IngestionHealthSnapshot"));
    assert!(health.contains("StorageHealthSnapshot"));
    assert!(health.contains("worker_queue"));
    assert!(health.contains("content_ref"));
}

#[test]
fn contract_persister_writes_chunk_kv_ssot() {
    let persister = include_str!("../../edgequake-pipeline/src/persistence/ingestion_persister.rs");
    assert!(
        persister.contains("build_chunk_kv_records"),
        "IngestionPersister must write chunk KV (SPEC-024 2.5)"
    );
    assert!(
        persister.contains("kv_storage"),
        "persister config must accept KV storage"
    );
}

#[test]
fn contract_orchestrator_workspace_cache_invalidation() {
    let src = include_str!("../../edgequake-core/src/orchestrator/ingestion.rs");
    assert!(
        src.contains("invalidate_query_result_cache_for_workspace"),
        "library insert must use workspace-scoped cache bust when configured"
    );
}

#[test]
fn contract_orchestrator_uses_workspace_vector_registry() {
    let ingestion = include_str!("../../edgequake-core/src/orchestrator/ingestion.rs");
    assert!(
        ingestion.contains("resolve_ingestion_vector_storage"),
        "library insert must resolve per-workspace vector storage (W7)"
    );

    let resolver = include_str!("../../edgequake-core/src/workspace_vector_resolve.rs");
    assert!(
        resolver.contains("resolve_workspace_vector_storage"),
        "workspace vector resolve SSOT must exist in edgequake-core"
    );

    let storage_helpers = include_str!("../src/handlers/documents/storage_helpers.rs");
    assert!(
        storage_helpers.contains("resolve_workspace_vector_storage"),
        "API storage_helpers must delegate to core SSOT"
    );
}

#[test]
fn contract_chunk_storage_content_ref_ssot() {
    let chunk = include_str!("../../edgequake-pipeline/src/chunk_storage.rs");
    assert!(chunk.contains("build_chunk_vector_metadata"));
    assert!(chunk.contains("content_ref"));
    assert!(
        chunk.contains("meta.get(\"content\").is_none()"),
        "vector metadata contract test must forbid inline content"
    );
}
