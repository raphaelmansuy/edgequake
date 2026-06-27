//! Persist structured mm chunks for pipeline modality-relation injection.

use edgequake_storage::traits::KVStorage;

use super::chunks::MultimodalChunk;

pub fn mm_chunks_key(document_id: &str) -> String {
    format!("{document_id}-multimodal-chunks")
}

pub async fn persist_mm_chunks(
    kv: &dyn KVStorage,
    document_id: &str,
    chunks: &[MultimodalChunk],
) -> Result<(), String> {
    let key = mm_chunks_key(document_id);
    let value = serde_json::to_value(chunks).map_err(|e| e.to_string())?;
    kv.upsert(&[(key, value)]).await.map_err(|e| e.to_string())
}

pub async fn load_mm_chunks(kv: &dyn KVStorage, document_id: &str) -> Option<Vec<MultimodalChunk>> {
    let key = mm_chunks_key(document_id);
    let value = kv.get_by_id(&key).await.ok()??;
    serde_json::from_value(value).ok()
}
