//! Workspace-scoped vector storage registry.
//!
//! # Implements
//!
//! - **FEAT0350**: Per-workspace vector storage with independent dimensions
//!
//! # Enforces
//!
//! - **BR0350**: Each workspace has isolated vector storage
//! - **BR0351**: Dimension is determined by workspace embedding configuration
//! - **BR0352**: Workspaces cannot cross-query vectors with different dimensions
//!
//! # WHY: Workspace Vector Isolation
//!
//! Different workspaces may use different embedding providers:
//! - OpenAI text-embedding-3-small: 1536 dimensions
//! - Ollama nomic-embed-text: 768 dimensions
//! - Cohere embed-v3: 1024 dimensions
//!
//! Mixing dimensions in a single vector table causes:
//! - Corrupt similarity scores (comparing apples to oranges)
//! - Index inefficiency (can't optimize for single dimension)
//! - Provider lock-in (can't change per-workspace)
//!
//! This registry creates per-workspace vector tables with:
//! - Correct dimension for the workspace's embedding provider
//! - Isolated HNSW/IVFFlat indices
//! - Independent lifecycle (can rebuild one workspace without affecting others)

use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use super::VectorStorage;
use crate::error::Result;

/// Configuration for workspace vector storage.
#[derive(Debug, Clone)]
pub struct WorkspaceVectorConfig {
    /// Workspace UUID
    pub workspace_id: Uuid,
    /// Embedding dimension for this workspace
    pub dimension: usize,
    /// Optional namespace prefix (default: "default")
    pub namespace: String,
}

impl WorkspaceVectorConfig {
    /// Create a new workspace vector configuration.
    pub fn new(workspace_id: Uuid, dimension: usize) -> Self {
        Self {
            workspace_id,
            dimension,
            namespace: "default".to_string(),
        }
    }

    /// Set the namespace prefix.
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Generate the table name for this workspace.
    ///
    /// Format: eq_{namespace}_ws_{workspace_id_short}_vectors
    /// Example: eq_default_ws_4e32a055_vectors
    pub fn table_name(&self) -> String {
        let short_id = &self.workspace_id.to_string()[..8];
        format!("eq_{}_ws_{}_vectors", self.namespace, short_id)
    }
}

/// Registry for managing per-workspace vector storage instances.
///
/// This registry provides lazy initialization of workspace-specific
/// vector storage with correct dimensions. Each workspace gets its
/// own PostgreSQL table with:
/// - Correct vector dimension
/// - Optimized HNSW index
/// - Isolated data lifecycle
///
/// # Thread Safety
///
/// The registry is thread-safe and can be shared across request handlers.
/// Internal locking ensures safe concurrent access to the instance cache.
///
/// # Example
///
/// ```ignore
/// // Get vector storage for a workspace
/// let config = WorkspaceVectorConfig::new(workspace_id, 1536);
/// let storage = registry.get_or_create(config).await?;
///
/// // Use the workspace-specific storage
/// storage.upsert(&vectors).await?;
/// ```
#[async_trait]
pub trait WorkspaceVectorRegistry: Send + Sync {
    /// Get or create vector storage for a workspace.
    ///
    /// If the workspace already has a storage instance cached, returns it.
    /// Otherwise, creates a new storage with the specified dimension.
    ///
    /// # Arguments
    ///
    /// * `config` - Workspace vector configuration including dimension
    ///
    /// # Returns
    ///
    /// Arc to the workspace's vector storage instance.
    async fn get_or_create(&self, config: WorkspaceVectorConfig) -> Result<Arc<dyn VectorStorage>>;

    /// Get existing vector storage for a workspace without creating.
    ///
    /// Returns None if the workspace doesn't have a cached storage instance.
    async fn get(&self, workspace_id: &Uuid) -> Option<Arc<dyn VectorStorage>>;

    /// Check if a workspace has vector storage initialized.
    async fn has_storage(&self, workspace_id: &Uuid) -> bool;

    /// Get the dimension of a workspace's vector storage.
    ///
    /// Returns None if the workspace doesn't have storage initialized.
    async fn get_dimension(&self, workspace_id: &Uuid) -> Option<usize>;

    /// List all workspace IDs that have vector storage.
    async fn list_workspaces(&self) -> Vec<Uuid>;

    /// Remove a workspace's vector storage from the cache.
    ///
    /// This does NOT delete the underlying table, just removes
    /// the cached instance. Useful for forcing re-initialization.
    async fn evict(&self, workspace_id: &Uuid);

    /// Clear all cached instances.
    async fn clear_cache(&self);

    /// Get the default/fallback vector storage.
    ///
    /// Used for backward compatibility when workspace_id is not specified.
    fn default_storage(&self) -> Arc<dyn VectorStorage>;
}
