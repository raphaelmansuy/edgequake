//! Storage adapter runtime bundle (SPEC-017 P1-04).

use std::sync::Arc;

use super::StorageMode;

/// KV, vector, graph, and optional PDF storage adapters.
#[derive(Clone)]
pub struct StorageRuntime {
    pub kv_storage: Arc<dyn edgequake_storage::traits::KVStorage>,
    pub vector_storage: Arc<dyn edgequake_storage::traits::VectorStorage>,
    pub vector_registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry>,
    pub graph_storage: Arc<dyn edgequake_storage::traits::GraphStorage>,
    #[cfg(feature = "postgres")]
    pub pdf_storage: Option<Arc<dyn edgequake_storage::PdfDocumentStorage>>,
    pub mode: StorageMode,
}

impl StorageRuntime {
    pub fn is_postgresql(&self) -> bool {
        self.mode.is_postgresql()
    }

    pub fn is_memory(&self) -> bool {
        self.mode.is_memory()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::adapters::memory::{
        MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage, MemoryWorkspaceVectorRegistry,
    };

    #[test]
    fn memory_mode_flags() {
        let kv = Arc::new(MemoryKVStorage::new("test"));
        let vector = Arc::new(MemoryVectorStorage::new("test", 1536));
        let graph = Arc::new(MemoryGraphStorage::new("test"));
        let registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry> =
            Arc::new(MemoryWorkspaceVectorRegistry::new(
                Arc::clone(&vector) as Arc<dyn edgequake_storage::traits::VectorStorage>
            ));

        let storage = StorageRuntime {
            kv_storage: Arc::clone(&kv) as Arc<dyn edgequake_storage::traits::KVStorage>,
            vector_storage: Arc::clone(&vector)
                as Arc<dyn edgequake_storage::traits::VectorStorage>,
            vector_registry: registry,
            graph_storage: Arc::clone(&graph) as Arc<dyn edgequake_storage::traits::GraphStorage>,
            #[cfg(feature = "postgres")]
            pdf_storage: None,
            mode: StorageMode::Memory,
        };

        assert!(storage.is_memory());
        assert!(!storage.is_postgresql());
    }
}
