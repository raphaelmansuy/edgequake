//! In-memory ChunkRepository adapter (SPEC-091 conformance stub).

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;
use uuid::Uuid;

use crate::adapters::memory::lock::map_lock_err;
use crate::error::StorageError;
use crate::traits::domain::{
    Chunk, ChunkCursor, ChunkId, ChunkRepository, ChunkText, DocumentId, InsertReport, Page,
    UnitOfWork,
};

#[derive(Default)]
pub struct MemoryChunkRepository {
    inner: RwLock<HashMap<Uuid, Chunk>>,
    serving_states: RwLock<HashMap<Uuid, String>>,
}

impl MemoryChunkRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ChunkRepository for MemoryChunkRepository {
    async fn insert_batch(
        &self,
        _tx: &mut UnitOfWork,
        chunks: &[Chunk],
    ) -> Result<InsertReport, StorageError> {
        let mut guard = self.inner.write().map_err(map_lock_err)?;
        let mut inserted = 0u64;
        let mut skipped = 0u64;
        for chunk in chunks {
            let dup = guard.values().any(|existing| {
                existing.document_id == chunk.document_id
                    && existing.chunk_index == chunk.chunk_index
            });
            if dup {
                skipped += 1;
                continue;
            }
            guard.insert(chunk.id.0, chunk.clone());
            inserted += 1;
        }
        Ok(InsertReport { inserted, skipped })
    }

    async fn load_texts(&self, ids: &[ChunkId]) -> Result<Vec<ChunkText>, StorageError> {
        let guard = self.inner.read().map_err(map_lock_err)?;
        Ok(ids
            .iter()
            .filter_map(|id| {
                guard.get(&id.0).map(|c| ChunkText {
                    id: c.id,
                    content: c.content.clone(),
                })
            })
            .collect())
    }

    async fn load_for_document(&self, document_id: DocumentId) -> Result<Vec<Chunk>, StorageError> {
        let guard = self.inner.read().map_err(map_lock_err)?;
        let mut items: Vec<Chunk> = guard
            .values()
            .filter(|c| c.document_id == document_id)
            .cloned()
            .collect();
        items.sort_by_key(|c| c.chunk_index);
        Ok(items)
    }

    async fn load_one(
        &self,
        document_id: DocumentId,
        chunk_index: i32,
    ) -> Result<Option<Chunk>, StorageError> {
        let guard = self.inner.read().map_err(map_lock_err)?;
        Ok(guard
            .values()
            .find(|c| c.document_id == document_id && c.chunk_index == chunk_index)
            .cloned())
    }

    async fn count_for_document(&self, document_id: DocumentId) -> Result<u64, StorageError> {
        let guard = self.inner.read().map_err(map_lock_err)?;
        Ok(guard
            .values()
            .filter(|c| c.document_id == document_id)
            .count() as u64)
    }

    async fn scan_from(
        &self,
        cursor: Option<ChunkCursor>,
        limit: u32,
    ) -> Result<Page<Chunk>, StorageError> {
        let guard = self.inner.read().map_err(map_lock_err)?;
        let mut items: Vec<Chunk> = guard.values().cloned().collect();
        items.sort_by(|a, b| {
            a.document_id
                .into_uuid()
                .cmp(&b.document_id.into_uuid())
                .then(a.chunk_index.cmp(&b.chunk_index))
        });
        if let Some(cur) = cursor {
            items.retain(|c| {
                c.document_id.into_uuid() > cur.document_id.into_uuid()
                    || (c.document_id.into_uuid() == cur.document_id.into_uuid()
                        && c.chunk_index > cur.chunk_index)
            });
        }
        let limit = limit as usize;
        let has_more = items.len() > limit;
        items.truncate(limit);
        let next_cursor = if has_more {
            items.last().map(|c| ChunkCursor {
                document_id: c.document_id,
                chunk_index: c.chunk_index,
            })
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }

    async fn delete_for_document(
        &self,
        _tx: &mut UnitOfWork,
        document_id: DocumentId,
    ) -> Result<u64, StorageError> {
        let mut guard = self.inner.write().map_err(map_lock_err)?;
        let before = guard.len();
        let removed_ids: Vec<Uuid> = guard
            .values()
            .filter(|chunk| chunk.document_id == document_id)
            .map(|chunk| chunk.id.0)
            .collect();
        guard.retain(|_, c| c.document_id != document_id);
        let mut states = self.serving_states.write().map_err(map_lock_err)?;
        for id in removed_ids {
            states.remove(&id);
        }
        Ok((before - guard.len()) as u64)
    }

    async fn set_serving_state(
        &self,
        document_id: DocumentId,
        state: &str,
    ) -> Result<u64, StorageError> {
        let chunks = self.inner.read().map_err(map_lock_err)?;
        let chunk_ids: Vec<Uuid> = chunks
            .values()
            .filter(|chunk| chunk.document_id == document_id)
            .map(|chunk| chunk.id.0)
            .collect();
        drop(chunks);

        let mut states = self.serving_states.write().map_err(map_lock_err)?;
        let mut changed = 0u64;
        for id in &chunk_ids {
            let previous = states.get(id).map(String::as_str);
            if previous != Some(state) {
                states.insert(*id, state.to_string());
                changed += 1;
            }
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::domain::{TenantId, WorkspaceId};

    #[tokio::test]
    async fn contract_spec091_memory_insert_batch_empty() {
        let repo = MemoryChunkRepository::new();
        let report = repo
            .insert_batch(&mut UnitOfWork::default(), &[])
            .await
            .expect("empty batch");
        assert_eq!(report.inserted, 0);
        assert_eq!(report.skipped, 0);
    }

    #[tokio::test]
    async fn contract_spec091_memory_insert_batch_idempotent() {
        let repo = MemoryChunkRepository::new();
        let chunk = Chunk {
            id: ChunkId::new(Uuid::new_v4()),
            document_id: DocumentId::new(Uuid::new_v4()),
            tenant_id: Some(TenantId::new(Uuid::new_v4())),
            workspace_id: Some(WorkspaceId::new(Uuid::new_v4())),
            chunk_index: 0,
            content: "hello".into(),
            start_offset: Some(0),
            end_offset: Some(5),
            token_count: Some(1),
            metadata: serde_json::json!({}),
            page_start: None,
            page_end: None,
        };
        let first = repo
            .insert_batch(&mut UnitOfWork::default(), std::slice::from_ref(&chunk))
            .await
            .unwrap();
        assert_eq!(first.inserted, 1);
        let second = repo
            .insert_batch(&mut UnitOfWork::default(), std::slice::from_ref(&chunk))
            .await
            .unwrap();
        assert_eq!(second.skipped, 1);
    }

    #[tokio::test]
    async fn set_serving_state_counts_only_changed_rows() {
        let repo = MemoryChunkRepository::new();
        let document_id = DocumentId::new(Uuid::new_v4());
        let chunk = Chunk {
            id: ChunkId::new(Uuid::new_v4()),
            document_id,
            tenant_id: Some(TenantId::new(Uuid::new_v4())),
            workspace_id: Some(WorkspaceId::new(Uuid::new_v4())),
            chunk_index: 0,
            content: "hello".into(),
            start_offset: Some(0),
            end_offset: Some(5),
            token_count: Some(1),
            metadata: serde_json::json!({}),
            page_start: None,
            page_end: None,
        };
        repo.insert_batch(&mut UnitOfWork::default(), std::slice::from_ref(&chunk))
            .await
            .unwrap();

        let first = repo.set_serving_state(document_id, "ready").await.unwrap();
        assert_eq!(first, 1);
        let second = repo.set_serving_state(document_id, "ready").await.unwrap();
        assert_eq!(second, 0);
        let third = repo
            .set_serving_state(document_id, "embedded")
            .await
            .unwrap();
        assert_eq!(third, 1);
    }
}
