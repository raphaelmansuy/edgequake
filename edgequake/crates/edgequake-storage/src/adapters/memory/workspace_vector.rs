//! In-memory implementation of WorkspaceVectorRegistry.
//!
//! # Implements
//!
//! - **FEAT0350**: Per-workspace vector storage with independent dimensions
//!
//! # WHY: Testing Support
//!
//! This implementation is used for:
//! - Unit tests
//! - Integration tests without PostgreSQL
//! - Development without database setup

use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use super::vector::MemoryVectorStorage;
use crate::adapters::workspace_vector_cache::WorkspaceVectorInstanceCache;
use crate::error::Result;
use crate::traits::{VectorStorage, WorkspaceVectorConfig, WorkspaceVectorRegistry};

/// In-memory implementation of WorkspaceVectorRegistry.
///
/// Each workspace gets its own MemoryVectorStorage instance with
/// the correct dimension. Useful for testing workspace isolation.
pub struct MemoryWorkspaceVectorRegistry {
    cache: WorkspaceVectorInstanceCache,
    /// Default vector storage for backward compatibility
    default_storage: Arc<dyn VectorStorage>,
}

impl MemoryWorkspaceVectorRegistry {
    /// Create a new in-memory workspace vector registry.
    ///
    /// # Arguments
    ///
    /// * `default_storage` - Default vector storage for backward compatibility
    pub fn new(default_storage: Arc<dyn VectorStorage>) -> Self {
        Self {
            cache: WorkspaceVectorInstanceCache::new(),
            default_storage,
        }
    }

    /// Create with a default dimension.
    pub fn with_default_dimension(dimension: usize) -> Self {
        let default_storage = Arc::new(MemoryVectorStorage::new("default", dimension));
        Self::new(default_storage)
    }
}

#[async_trait]
impl WorkspaceVectorRegistry for MemoryWorkspaceVectorRegistry {
    async fn get_or_create(&self, config: WorkspaceVectorConfig) -> Result<Arc<dyn VectorStorage>> {
        let workspace_id = config.workspace_id;
        let dimension = config.dimension;
        let noop_validate = |_: &Arc<dyn VectorStorage>| Ok(());

        self.cache
            .get_or_create(workspace_id, noop_validate, || async move {
                let namespace = format!("ws_{}", &workspace_id.to_string()[..8]);
                let storage: Arc<dyn VectorStorage> =
                    Arc::new(MemoryVectorStorage::new(&namespace, dimension));

                tracing::debug!(
                    workspace_id = %workspace_id,
                    dimension = dimension,
                    namespace = %namespace,
                    "Created in-memory workspace vector storage"
                );

                Ok(storage)
            })
            .await
    }

    async fn get(&self, workspace_id: &Uuid) -> Option<Arc<dyn VectorStorage>> {
        self.cache.get(workspace_id).await
    }

    async fn has_storage(&self, workspace_id: &Uuid) -> bool {
        self.cache.has(workspace_id).await
    }

    async fn get_dimension(&self, workspace_id: &Uuid) -> Option<usize> {
        self.cache.get_dimension(workspace_id).await
    }

    async fn list_workspaces(&self) -> Vec<Uuid> {
        self.cache.list_workspaces().await
    }

    async fn evict(&self, workspace_id: &Uuid) {
        self.cache.evict(workspace_id).await;
    }

    async fn clear_cache(&self) {
        self.cache.clear().await;
    }

    fn default_storage(&self) -> Arc<dyn VectorStorage> {
        Arc::clone(&self.default_storage)
    }
}

impl std::fmt::Debug for MemoryWorkspaceVectorRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryWorkspaceVectorRegistry")
            .field("default_dimension", &self.default_storage.dimension())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workspace_isolation() {
        let registry = MemoryWorkspaceVectorRegistry::with_default_dimension(1536);

        let ws1 = Uuid::new_v4();
        let ws2 = Uuid::new_v4();

        // Create storage for workspace 1 with 1536 dims
        let config1 = WorkspaceVectorConfig::new(ws1, 1536);
        let storage1 = registry.get_or_create(config1).await.unwrap();
        assert_eq!(storage1.dimension(), 1536);

        // Create storage for workspace 2 with 768 dims
        let config2 = WorkspaceVectorConfig::new(ws2, 768);
        let storage2 = registry.get_or_create(config2).await.unwrap();
        assert_eq!(storage2.dimension(), 768);

        // Verify isolation
        assert!(registry.has_storage(&ws1).await);
        assert!(registry.has_storage(&ws2).await);
        assert_eq!(registry.get_dimension(&ws1).await, Some(1536));
        assert_eq!(registry.get_dimension(&ws2).await, Some(768));
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let registry = MemoryWorkspaceVectorRegistry::with_default_dimension(1536);
        let ws = Uuid::new_v4();

        let config = WorkspaceVectorConfig::new(ws, 1536);
        let _ = registry.get_or_create(config).await.unwrap();
        assert!(registry.has_storage(&ws).await);

        registry.evict(&ws).await;
        assert!(!registry.has_storage(&ws).await);
    }
}
