//! PostgreSQL implementation of WorkspaceVectorRegistry.
//!
//! # Implements
//!
//! - **FEAT0350**: Per-workspace vector storage with independent dimensions
//!
//! # WHY: Per-Workspace Vector Tables
//!
//! Each workspace may use a different embedding provider:
//! - OpenAI: 1536 dimensions
//! - Ollama nomic-embed-text: 768 dimensions
//! - Cohere: 1024 dimensions
//!
//! This registry creates and manages per-workspace PostgreSQL tables
//! with the correct vector dimension for each workspace.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::config::{PostgresConfig, VectorIndexType};
use super::vector::PgVectorStorage;
use crate::error::{Result, StorageError};
use crate::traits::{VectorStorage, WorkspaceVectorConfig, WorkspaceVectorRegistry};

/// PostgreSQL implementation of WorkspaceVectorRegistry.
///
/// Manages per-workspace vector storage instances with:
/// - Lazy table creation on first access
/// - Cached instances for performance
/// - Correct dimension per workspace
pub struct PgWorkspaceVectorRegistry {
    /// Base PostgreSQL configuration
    config: PostgresConfig,
    /// Cached workspace vector storage instances
    instances: RwLock<HashMap<Uuid, Arc<dyn VectorStorage>>>,
    /// Default vector storage for backward compatibility
    default_storage: Arc<dyn VectorStorage>,
    /// Default dimension for new workspaces
    default_dimension: usize,
}

impl PgWorkspaceVectorRegistry {
    /// Create a new PostgreSQL workspace vector registry.
    ///
    /// # Arguments
    ///
    /// * `config` - Base PostgreSQL configuration
    /// * `default_storage` - Default vector storage for backward compatibility
    /// * `default_dimension` - Default dimension for new workspaces
    pub fn new(
        config: PostgresConfig,
        default_storage: Arc<dyn VectorStorage>,
        default_dimension: usize,
    ) -> Self {
        Self {
            config,
            instances: RwLock::new(HashMap::new()),
            default_storage,
            default_dimension,
        }
    }

    /// Create workspace-specific vector storage.
    ///
    /// @implements OODA-228: Handle dimension changes during provider switch
    ///
    /// When a workspace's embedding provider is changed, the stored vectors may have
    /// a different dimension than the new provider expects. This method ensures the
    /// vector table is recreated with the correct dimension if necessary.
    async fn create_workspace_storage(
        &self,
        config: &WorkspaceVectorConfig,
    ) -> Result<Arc<dyn VectorStorage>> {
        // Create PostgreSQL config with workspace-specific namespace
        let short_id = &config.workspace_id.to_string()[..8];
        let namespace = format!("{}_ws_{}", config.namespace, short_id);

        let pg_config = PostgresConfig::new(
            self.config.host.clone(),
            self.config.port,
            self.config.database.clone(),
            self.config.user.clone(),
            self.config.password.clone(),
        )
        .with_namespace(&namespace)
        .with_max_connections(self.config.max_connections)
        .with_vector_index(VectorIndexType::HNSW);
        // WHY: HNSW config defaults (m=16, ef_construction=64) are already optimal

        // Create storage with workspace-specific dimension
        let storage = PgVectorStorage::with_dimension(pg_config, config.dimension);

        // OODA-228: Ensure table has correct dimension BEFORE initialize
        // WHY: If embedding provider changed (e.g., OpenAI 1536 → Ollama 768),
        // the existing table has the wrong dimension. We must recreate it.
        // This must happen before initialize() which calls CREATE TABLE IF NOT EXISTS.
        //
        // The ensure_dimension() method:
        // 1. Checks stored dimension vs requested dimension
        // 2. If mismatch: DROP TABLE and recreate with new dimension
        // 3. If match or empty: no-op
        let recreated = storage.ensure_dimension(config.dimension).await?;
        if recreated {
            tracing::info!(
                workspace_id = %config.workspace_id,
                dimension = config.dimension,
                "Vector table recreated due to dimension change (OODA-228)"
            );
        }

        // Initialize the storage (creates table if not exists)
        storage.initialize().await?;

        tracing::info!(
            workspace_id = %config.workspace_id,
            dimension = config.dimension,
            table = %config.table_name(),
            "Created workspace-specific vector storage"
        );

        Ok(Arc::new(storage))
    }
}

#[async_trait]
impl WorkspaceVectorRegistry for PgWorkspaceVectorRegistry {
    async fn get_or_create(&self, config: WorkspaceVectorConfig) -> Result<Arc<dyn VectorStorage>> {
        // Check cache first (read lock)
        {
            let instances = self.instances.read().await;
            if let Some(storage) = instances.get(&config.workspace_id) {
                // Validate dimension matches
                let cached_dim = storage.dimension();
                if cached_dim != config.dimension {
                    return Err(StorageError::InvalidQuery(format!(
                        "Dimension mismatch for workspace {}: cached={}, requested={}. \
                         Clear cache with evict() to reinitialize.",
                        config.workspace_id, cached_dim, config.dimension
                    )));
                }
                return Ok(Arc::clone(storage));
            }
        }

        // Create new storage (write lock)
        let mut instances = self.instances.write().await;

        // Double-check after acquiring write lock
        if let Some(storage) = instances.get(&config.workspace_id) {
            return Ok(Arc::clone(storage));
        }

        // Create and cache new storage
        let storage = self.create_workspace_storage(&config).await?;
        instances.insert(config.workspace_id, Arc::clone(&storage));

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
        tracing::debug!(
            workspace_id = %workspace_id,
            "Evicted workspace vector storage from cache"
        );
    }

    async fn clear_cache(&self) {
        let mut instances = self.instances.write().await;
        let count = instances.len();
        instances.clear();
        tracing::info!(
            count = count,
            "Cleared all workspace vector storage instances from cache"
        );
    }

    fn default_storage(&self) -> Arc<dyn VectorStorage> {
        Arc::clone(&self.default_storage)
    }
}

impl std::fmt::Debug for PgWorkspaceVectorRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgWorkspaceVectorRegistry")
            .field("default_dimension", &self.default_dimension)
            .field("config_host", &self.config.host)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_vector_config_table_name() {
        let workspace_id = Uuid::parse_str("4e32a055-9722-40f9-b03e-ade870b07604").unwrap();
        let config = WorkspaceVectorConfig::new(workspace_id, 1536);

        assert_eq!(config.table_name(), "eq_default_ws_4e32a055_vectors");
    }

    #[test]
    fn test_workspace_vector_config_with_namespace() {
        let workspace_id = Uuid::parse_str("4e32a055-9722-40f9-b03e-ade870b07604").unwrap();
        let config = WorkspaceVectorConfig::new(workspace_id, 768).with_namespace("prod");

        assert_eq!(config.table_name(), "eq_prod_ws_4e32a055_vectors");
    }
}
