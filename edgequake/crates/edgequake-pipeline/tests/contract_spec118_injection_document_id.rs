//! SPEC-118 — injection composite document ids under relational chunk authority.
//!
//! Regression for GitHub #376: `injection::{ws}::{id}` must map to the injection
//! UUID for `public.chunks.document_id` instead of hard-failing Uuid::parse_str.

use edgequake_pipeline::chunker::TextChunk;
use edgequake_pipeline::pipeline::ProcessingResult;
use edgequake_pipeline::{
    build_relational_chunks, persist_relational_chunks, resolve_relational_document_id,
    IngestionPersistContext,
};
use edgequake_storage::traits::domain::ChunkRepository;
use edgequake_storage::MemoryChunkRepository;
use edgequake_storage::StorageError;
use std::sync::Arc;
use uuid::Uuid;

fn sample_result(doc_id: &str) -> ProcessingResult {
    ProcessingResult {
        document_id: doc_id.into(),
        chunks: vec![TextChunk {
            id: format!("{doc_id}-chunk-0"),
            content: "glossary Term Alpha relates to Term Beta.".into(),
            index: 0,
            start_offset: 0,
            end_offset: 40,
            start_line: 1,
            end_line: 1,
            token_count: 8,
            embedding: Some(vec![0.1, 0.2, 0.3]),
            section: None,
            page_start: None,
            page_end: None,
            modality: None,
        }],
        extractions: vec![],
        stats: Default::default(),
        lineage: None,
    }
}

#[test]
fn contract_spec118_issue376_composite_resolves() {
    let raw =
        "injection::00000000-0000-0000-0000-000000000000::3fc4a415-33e7-4a38-88d9-86ae6b8bb36e";
    assert_eq!(raw.len(), 85);
    let id = resolve_relational_document_id(raw).expect("resolve");
    assert_eq!(
        id.into_uuid(),
        Uuid::parse_str("3fc4a415-33e7-4a38-88d9-86ae6b8bb36e").unwrap()
    );
}

#[tokio::test]
async fn e2e_spec118_relational_persist_injection_composite() {
    let ws = Uuid::new_v4();
    let inj = Uuid::new_v4();
    let composite = format!("injection::{ws}::{inj}");
    let ctx = IngestionPersistContext::new(composite.clone(), None, Some(ws.to_string()));
    let repo = Arc::new(MemoryChunkRepository::new());

    persist_relational_chunks(repo.as_ref(), &ctx, &sample_result(&composite))
        .await
        .expect("persist must not fail on injection:: composite (#376)");

    let spine = repo
        .load_for_document(edgequake_storage::traits::domain::DocumentId::new(inj))
        .await
        .expect("load");
    assert_eq!(spine.len(), 1);
    assert_eq!(spine[0].document_id.into_uuid(), inj);
    assert_eq!(
        spine[0]
            .metadata
            .get("legacy_document_id")
            .and_then(|v| v.as_str()),
        Some(composite.as_str())
    );
}

#[test]
fn contract_spec118_garbage_still_fail_closed() {
    let ctx = IngestionPersistContext::new("not-a-uuid", None, None);
    let err = build_relational_chunks(&ctx, &sample_result("not-a-uuid")).unwrap_err();
    assert!(matches!(err, StorageError::InvalidData(_)));
}
