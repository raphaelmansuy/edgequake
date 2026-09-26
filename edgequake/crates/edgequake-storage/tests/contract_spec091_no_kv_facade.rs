//! SPEC-091 IW3 (GAP-091-01 / EC-I6): compile-time census of remaining
//! `KVStorage` trait imports outside typed ports.
//!
//! The generic KV facade is being retired incrementally. This test documents the
//! allowlist of files that may still import or call through `KVStorage` until
//! migration completes. Shrink the allowlist as callers move to typed ports.
//!
//! Run:
//!   cargo test -p edgequake-storage --test contract_spec091_no_kv_facade

use std::path::{Path, PathBuf};

/// Paths relative to `edgequake/crates/` that may still reference `KVStorage`.
///
/// TODO(IW3+): shrink toward `{edgequake-storage/src/adapters/postgres/kv.rs,
/// edgequake-storage/src/adapters/memory/*, edgequake-storage/src/traits/kv.rs,
/// edgequake-storage/src/compensation.rs}` only.
const ALLOWLIST: &[&str] = &[
    // --- typed ports / facade implementation ---
    "edgequake-storage/src/adapters/postgres/kv.rs",
    "edgequake-storage/src/adapters/postgres/mod.rs",
    "edgequake-storage/src/adapters/memory/mod.rs",
    "edgequake-storage/src/adapters/memory/domain/chunk_repository.rs",
    "edgequake-storage/src/traits/mod.rs",
    "edgequake-storage/src/traits/kv.rs",
    "edgequake-storage/src/lib.rs",
    "edgequake-storage/src/compensation.rs",
    // --- orchestrator / pipeline (migrate to ChunkRepository + sidecars) ---
    "edgequake-core/src/orchestrator/mod.rs",
    "edgequake-core/src/orchestrator/ingestion.rs",
    "edgequake-pipeline/src/persistence/ingestion_persister.rs",
    // --- edgequake-api: remaining facade callers (IW3 debt) ---
    "edgequake-api/src/state/postgres.rs",
    "edgequake-api/src/state/memory.rs",
    "edgequake-api/src/state/query_bootstrap.rs",
    "edgequake-api/src/handlers/health.rs",
    "edgequake-api/src/handlers/injection/crud.rs",
    "edgequake-api/src/handlers/documents/storage_helpers.rs",
    "edgequake-api/src/handlers/documents/query/download.rs",
    "edgequake-api/src/handlers/lineage/cache.rs",
    "edgequake-api/src/handlers/query/query_execute.rs",
    "edgequake-api/src/processor/injection_processing.rs",
    "edgequake-api/src/processor/text_insert/extraction.rs",
    "edgequake-api/src/processor/text_insert/persist.rs",
    "edgequake-api/src/processor/pipeline_checkpoint.rs",
    "edgequake-api/src/processor/pdf_processing.rs",
    "edgequake-api/src/services/document_deletion.rs",
    "edgequake-api/src/services/document_body_loader.rs",
    "edgequake-api/src/services/document_metadata_scan.rs",
    "edgequake-api/src/services/list_run_enrich.rs",
    "edgequake-api/src/services/ingestion_persist.rs",
    "edgequake-api/src/services/injection_process.rs",
    "edgequake-api/src/services/multimodal/analyzer.rs",
    "edgequake-api/src/services/multimodal/manifest_store.rs",
    "edgequake-api/src/services/task_document_sync.rs",
    "edgequake-api/tests/common/mod.rs",
    // --- contract / e2e tests that exercise the facade directly ---
    "edgequake-storage/tests/contract_spec091_unknown_family_loud.rs",
    "edgequake-storage/tests/contract_spec091_get_by_ids_typed.rs",
    "edgequake-api/src/services/workspace_document_index.rs",
    "edgequake-api/src/services/compensation_drain_applier.rs",
    "edgequake-api/src/services/cancel_facade.rs",
    "edgequake-api/src/services/query_context.rs",
    "edgequake-api/src/services/tenant_isolation.rs",
    "edgequake-api/src/services/staging_admission.rs",
    "edgequake-api/src/services/ingest_admission.rs",
    "edgequake-api/src/services/injection_list.rs",
    "edgequake-api/src/services/document_metadata_repair.rs",
    "edgequake-api/src/services/document_quota.rs",
    "edgequake-api/src/services/document_graph_cascade.rs",
    "edgequake-api/src/services/cost_aggregation.rs",
    "edgequake-api/src/services/document_mm_asset_persist.rs",
    "edgequake-api/src/services/document_original_persist.rs",
    "edgequake-api/src/services/graph_community.rs",
    "edgequake-api/src/services/pdf_workspace_dedup.rs",
    "edgequake-api/src/services/text_insert_content.rs",
    "edgequake-api/src/services/orphan_staging_recovery.rs",
    "edgequake-api/src/services/orphan_task_recovery.rs",
    "edgequake-api/src/services/orphan_index_retract.rs",
    "edgequake-api/src/services/retract_document_indexes.rs",
    "edgequake-api/src/services/reprocess_stage_reset.rs",
    "edgequake-api/src/services/multimodal/cache.rs",
    "edgequake-api/src/services/multimodal/chunks.rs",
    "edgequake-api/src/services/multimodal/chunks_store.rs",
    "edgequake-api/src/services/multimodal/stage.rs",
    "edgequake-api/src/services/multimodal/image_specialize.rs",
    "edgequake-api/src/handlers/query/mod.rs",
    "edgequake-api/src/handlers/query/document_filter_resolver.rs",
    "edgequake-api/src/handlers/isolation.rs",
    "edgequake-api/src/handlers/health_probes.rs",
    "edgequake-api/src/handlers/lineage/queries.rs",
    "edgequake-api/src/handlers/pdf_upload/upload.rs",
    "edgequake-api/src/handlers/workspaces/stats.rs",
    "edgequake-api/src/handlers/documents/recovery/chunks.rs",
    "edgequake-api/src/handlers/documents/recovery/reprocess.rs",
    "edgequake-api/src/pipeline_progress_callback.rs",
    "edgequake-api/src/processor/mod.rs",
    "edgequake-api/src/processor/status_updates.rs",
    // --- query engine (chunk hydration via KV — migrate to ChunkRepository) ---
    "edgequake-query/src/bootstrap.rs",
    "edgequake-query/src/chunk_hydration.rs",
    "edgequake-query/src/cache/llm_response_cache.rs",
    "edgequake-query/src/engine_impl/mod.rs",
    "edgequake-query/tests/contract_chunk_hydration.rs",
    "edgequake-query/src/engine_impl/query_entry/query_pipeline.rs",
    "edgequake-query/src/engine_impl/query_modes.rs",
    "edgequake-query/src/sparse_retrieval.rs",
    "edgequake-query/src/topic_entity_admit.rs",
    // --- core deletion path ---
    "edgequake-core/src/orchestrator/deletion.rs",
    "edgequake-core/src/conversation_service_impl.rs",
    // --- pipeline merger (graph batch via KVStorage bounds in tests/helpers) ---
    "edgequake-pipeline/src/merger/mod.rs",
    "edgequake-pipeline/src/merger/entity.rs",
    "edgequake-pipeline/src/merger/relationship.rs",
    "edgequake-pipeline/src/text_embedder.rs",
    // --- storage internals referencing KVStorage in docs/annotations ---
    "edgequake-storage/src/adapters/memory/kv.rs",
    "edgequake-storage/src/adapters/postgres/document_shell.rs",
    "edgequake-storage/src/chunk_content.rs",
    "edgequake-storage/src/dataop.rs",
    "edgequake-storage/src/dataop_annotations.rs",
    "edgequake-storage/src/kv_key_schema.rs",
];

const KV_MARKERS: &[&str] = &[
    "use edgequake_storage::traits::KVStorage",
    "dyn KVStorage",
    "PostgresKVStorage",
    "MemoryKVStorage",
    "impl KVStorage for",
];

/// Files that reference KV via field access only (`.kv_storage`) — tracked
/// separately; not counted as facade-import debt.
const FIELD_ACCESS_ONLY_EXCLUDE: &[&str] = &[
    "edgequake-api/src/state/storage_runtime.rs",
    "edgequake-api/src/state/runtime_extractors.rs",
];

fn workspace_crates_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn walk_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            walk_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn rel_from_crates(path: &Path, crates_root: &Path) -> String {
    path.strip_prefix(crates_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn file_matches_kv_marker(src: &str) -> bool {
    if KV_MARKERS.iter().any(|m| src.contains(m)) {
        return true;
    }
    // Trait import variants without full path.
    src.contains("KVStorage") && (src.contains("use ") || src.contains("impl KVStorage"))
}

#[test]
fn contract_spec091_no_kv_facade_outside_allowlist() {
    let crates_root = workspace_crates_root();
    let mut files = Vec::new();
    walk_rs_files(&crates_root, &mut files);

    let allow: std::collections::HashSet<&str> = ALLOWLIST.iter().copied().collect();
    let mut offenders = Vec::new();

    for path in files {
        let rel = rel_from_crates(&path, &crates_root);
        // Census production `src/` only — integration tests may construct MemoryKVStorage.
        if !rel.contains("/src/") || rel.contains("/tests/") {
            continue;
        }
        let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {rel}: {e}"));
        if !file_matches_kv_marker(&src) {
            continue;
        }
        if allow.contains(rel.as_str()) {
            continue;
        }
        if FIELD_ACCESS_ONLY_EXCLUDE.contains(&rel.as_str()) {
            continue;
        }
        // Skip self.
        if rel.contains("contract_spec091_no_kv_facade.rs") {
            continue;
        }
        offenders.push(rel);
    }

    offenders.sort();
    offenders.dedup();
    assert!(
        offenders.is_empty(),
        "KVStorage facade references outside allowlist (migrate to typed ports):\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn contract_spec091_kv_allowlist_non_empty_and_documents_debt() {
    assert!(
        ALLOWLIST.len() >= 10,
        "allowlist must document remaining debt explicitly"
    );
    assert!(
        ALLOWLIST
            .iter()
            .any(|p| p.contains("edgequake-api/src/services/document_deletion.rs")),
        "allowlist must include known high-debt callers until migrated"
    );
}
