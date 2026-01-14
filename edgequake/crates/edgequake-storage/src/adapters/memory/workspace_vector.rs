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
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::vector::MemoryVectorStorage;
use crate::error::Result;
use crate::traits::{VectorStorage, WorkspaceVectorConfig, WorkspaceVectorRegistry};

/// In-memory implementation of WorkspaceVectorRegistry.
///
/// Each workspace gets its own MemoryVectorStorage instance with
/// the correct dimension. Useful for testing workspace isolation.
pub struct MemoryWorkspaceVectorRegistry {
    /// Cached workspace vector storage instances
    instances: RwLock<HashMap<Uuid, Arc<dyn VectorStorage>>>,
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
            instances: RwLock::new(HashMap::new()),
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
        // Check cache first (read lock)
        {
            let instances = self.instances.read().await;
            if let Some(storage) = instances.get(&config.workspace_id) {
                return Ok(Arc::clone(storage));
            }
        }

        // Create new storage (write lock)
        let mut instances = self.instances.write().await;

        // Double-check after acquiring write lock
        if let Some(storage) = instances.get(&config.workspace_id) {
            return Ok(Arc::clone(storage));
        }

        // Create workspace-specific namespace
        let namespace = format!("ws_{}", &config.workspace_id.to_string()[..8]);
        let storage: Arc<dyn VectorStorage> =
            Arc::new(MemoryVectorStorage::new(&namespace, config.dimension));

        instances.insert(config.workspace_id, Arc::clone(&storage));

        tracing::debug!(
            workspace_id = %config.workspace_id,
            dimension = config.dimension,
            namespace = %namespace,
            "Created in-memory workspace vector storage"
        );

        Ok(storage)
    }

    async fn get(&self, workspace_id: &Uuid) -> Option<Arc<dyn VectorStorage>> {
        let instances = self.instances.read().await;
        instances.get(workspace_id).cloned()
    }

    async fn has_storage(&self, workspace_id: &Uuid) -> bool {
        let instances = self.instances.read().await;
        instances.contains_key(workspace_id)
    }

    async fn get_dimension(&self, workspace_id: &Uuid) -> Option<usize> {
        let instances = self.instances.read().await;
        instances.get(workspace_id).map(|s| s.dimension())
    }

    async fn list_workspaces(&self) -> Vec<Uuid> {
        let instances = self.instances.read().await;
        instances.keys().copied().collect()
    }

    async fn evict(&self, workspace_id: &Uuid) {
        let mut instances = self.instances.write().await;
        instances.remove(workspace_id);
    }

    async fn clear_cache(&self) {
        let mut instances = self.instances.write().await;
        instances.clear();
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
