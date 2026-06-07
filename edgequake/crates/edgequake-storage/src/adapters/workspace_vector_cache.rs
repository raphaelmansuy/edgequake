//! Shared workspace vector instance cache (STORE-DRY-004).
//!
//! Double-checked locking for per-workspace `VectorStorage` instances.
//! Memory and PostgreSQL registries share this helper; backends supply
//! validation (e.g. dimension mismatch on cache hit for postgres).

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::Result;
use crate::traits::VectorStorage;

/// In-memory cache of workspace-scoped vector storage instances.
pub struct WorkspaceVectorInstanceCache {
    instances: RwLock<HashMap<Uuid, Arc<dyn VectorStorage>>>,
}

impl WorkspaceVectorInstanceCache {
    pub fn new() -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get(&self, workspace_id: &Uuid) -> Option<Arc<dyn VectorStorage>> {
        self.instances.read().await.get(workspace_id).cloned()
    }

    pub async fn has(&self, workspace_id: &Uuid) -> bool {
        self.instances.read().await.contains_key(workspace_id)
    }

    pub async fn get_dimension(&self, workspace_id: &Uuid) -> Option<usize> {
        self.instances
            .read()
            .await
            .get(workspace_id)
            .map(|s| s.dimension())
    }

    pub async fn list_workspaces(&self) -> Vec<Uuid> {
        self.instances.read().await.keys().copied().collect()
    }

    pub async fn evict(&self, workspace_id: &Uuid) {
        self.instances.write().await.remove(workspace_id);
    }

    pub async fn clear(&self) {
        self.instances.write().await.clear();
    }

    /// Read-check → write-lock → double-check → create → insert.
    pub async fn get_or_create<F, Fut, V>(
        &self,
        workspace_id: Uuid,
        validate: V,
        create: F,
    ) -> Result<Arc<dyn VectorStorage>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Arc<dyn VectorStorage>>>,
        V: Fn(&Arc<dyn VectorStorage>) -> Result<()>,
    {
        {
            let instances = self.instances.read().await;
            if let Some(storage) = instances.get(&workspace_id) {
                validate(storage)?;
                return Ok(Arc::clone(storage));
            }
        }

        let mut instances = self.instances.write().await;
        if let Some(storage) = instances.get(&workspace_id) {
            validate(storage)?;
            return Ok(Arc::clone(storage));
        }

        let storage = create().await?;
        instances.insert(workspace_id, Arc::clone(&storage));
        Ok(storage)
    }
}

impl Default for WorkspaceVectorInstanceCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::MemoryVectorStorage;

    #[tokio::test]
    async fn cache_get_or_create_returns_same_instance() {
        let cache = WorkspaceVectorInstanceCache::new();
        let ws = Uuid::new_v4();
        let noop = |_: &Arc<dyn VectorStorage>| Ok(());

        let a = cache
            .get_or_create(ws, noop, || async {
                Ok(Arc::new(MemoryVectorStorage::new("ws_test", 128)) as Arc<dyn VectorStorage>)
            })
            .await
            .unwrap();
        let b = cache
            .get_or_create(ws, noop, || async {
                panic!("create should not run on cache hit")
            })
            .await
            .unwrap();

        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(cache.get_dimension(&ws).await, Some(128));
    }

    #[tokio::test]
    async fn cache_evict_forces_recreate() {
        let cache = WorkspaceVectorInstanceCache::new();
        let ws = Uuid::new_v4();
        let noop = |_: &Arc<dyn VectorStorage>| Ok(());

        let _ = cache
            .get_or_create(ws, noop, || async {
                Ok(Arc::new(MemoryVectorStorage::new("ws_a", 64)) as Arc<dyn VectorStorage>)
            })
            .await
            .unwrap();

        cache.evict(&ws).await;
        assert!(!cache.has(&ws).await);

        let _ = cache
            .get_or_create(ws, noop, || async {
                Ok(Arc::new(MemoryVectorStorage::new("ws_b", 64)) as Arc<dyn VectorStorage>)
            })
            .await
            .unwrap();
        assert!(cache.has(&ws).await);
    }
}
