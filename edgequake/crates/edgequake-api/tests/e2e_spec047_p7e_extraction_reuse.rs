//! SPEC-047 P7e e2e: durable snapshot survives checkpoint clear; merge-only plan.

use std::sync::Arc;

use edgequake_api::processor::pipeline_checkpoint::{
    clear_extraction_snapshot, clear_pipeline_checkpoint, load_extraction_snapshot,
    load_pipeline_checkpoint, plan_extraction_reuse, save_extraction_snapshot,
    save_pipeline_checkpoint, ExtractionReuseKind, ExtractionReusePlan,
};
use edgequake_pipeline::{ProcessingResult, ProcessingStats};
use edgequake_storage::{KVStorage, MemoryKVStorage};

fn sample_result(doc: &str, ents: usize) -> ProcessingResult {
    ProcessingResult {
        document_id: doc.to_string(),
        chunks: vec![],
        extractions: vec![],
        stats: ProcessingStats {
            entity_count: ents,
            relationship_count: ents.saturating_sub(1),
            ..Default::default()
        },
        lineage: None,
    }
}

#[tokio::test]
async fn e2e_p7e_soft_reprocess_reuses_snapshot_after_success_clear() {
    let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("p7e-e2e"));
    let doc = "doc-soft";
    let text = "full markdown body for soft reprocess";
    let result = sample_result(doc, 12);

    // Simulate mid-flight checkpoint + successful finalize promote.
    save_pipeline_checkpoint(&kv, None, doc, &result, "ws", "openai", "ollama", text)
        .await
        .unwrap();
    save_extraction_snapshot(&kv, None, doc, &result, "ws", "openai", "ollama", text)
        .await
        .unwrap();
    clear_pipeline_checkpoint(&kv, None, doc).await;

    let plan = plan_extraction_reuse(
        load_pipeline_checkpoint(&kv, None, doc, "ws", "openai", "ollama", text)
            .await
            .is_some(),
        load_extraction_snapshot(&kv, None, doc, "ws", "openai", "ollama", text)
            .await
            .is_some(),
        false,
        false,
    );
    assert_eq!(
        plan,
        ExtractionReusePlan::Reuse(ExtractionReuseKind::DurableSnapshot)
    );

    let snap = load_extraction_snapshot(&kv, None, doc, "ws", "openai", "ollama", text)
        .await
        .expect("snapshot");
    assert_eq!(snap.stats.entity_count, 12);
}

#[tokio::test]
async fn e2e_p7e_merge_only_requires_snapshot() {
    let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("p7e-merge"));
    let doc = "doc-merge";
    let text = "merge only body";

    let plan_missing = plan_extraction_reuse(
        load_pipeline_checkpoint(&kv, None, doc, "ws", "openai", "ollama", text)
            .await
            .is_some(),
        load_extraction_snapshot(&kv, None, doc, "ws", "openai", "ollama", text)
            .await
            .is_some(),
        false,
        true,
    );
    assert_eq!(plan_missing, ExtractionReusePlan::MergeOnlyMissing);

    save_extraction_snapshot(
        &kv,
        None,
        doc,
        &sample_result(doc, 3),
        "ws",
        "openai",
        "ollama",
        text,
    )
    .await
    .unwrap();

    let plan_ok = plan_extraction_reuse(
        false,
        load_extraction_snapshot(&kv, None, doc, "ws", "openai", "ollama", text)
            .await
            .is_some(),
        false,
        true,
    );
    assert_eq!(
        plan_ok,
        ExtractionReusePlan::Reuse(ExtractionReuseKind::DurableSnapshot)
    );

    clear_extraction_snapshot(&kv, None, doc).await;
}
