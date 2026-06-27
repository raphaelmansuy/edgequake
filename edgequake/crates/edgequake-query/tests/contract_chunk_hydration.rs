//! SPEC-024 2.5 — chunk content hydration from KV when vector metadata omits inline text.

use std::sync::Arc;

use edgequake_query::chunk_hydration::hydrate_retrieved_chunks;
use edgequake_query::context::RetrievedChunk;
use edgequake_storage::adapters::memory::MemoryKVStorage;
use edgequake_storage::traits::KVStorage;

#[tokio::test]
async fn contract_hydrate_fills_empty_chunk_from_kv() {
    let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("hydrate"));
    kv.upsert(&[(
        "doc-chunk-0".to_string(),
        serde_json::json!({ "content": "Authoritative chunk body from KV" }),
    )])
    .await
    .unwrap();

    let mut chunks = vec![RetrievedChunk::new("doc-chunk-0", "", 0.9)];

    hydrate_retrieved_chunks(Some(kv.as_ref()), &mut chunks).await;

    assert_eq!(chunks[0].content, "Authoritative chunk body from KV");
}
