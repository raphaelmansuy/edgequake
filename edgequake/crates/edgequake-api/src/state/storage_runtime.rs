//! Storage adapter runtime bundle (SPEC-017 P1-04).

use std::sync::Arc;

use super::StorageMode;
use crate::services::auth_memory_store::AuthMemoryStore;

/// KV, vector, graph, and optional PDF storage adapters.
#[derive(Clone)]
pub struct StorageRuntime {
    pub kv_storage: Arc<dyn edgequake_storage::traits::KVStorage>,
    pub vector_storage: Arc<dyn edgequake_storage::traits::VectorStorage>,
    pub vector_registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry>,
    pub graph_storage: Arc<dyn edgequake_storage::traits::GraphStorage>,
    /// In-memory auth artifacts when PostgreSQL pool is unavailable (never KV).
    pub auth_memory: Arc<AuthMemoryStore>,
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

    /// Fail closed when PostgreSQL mode is active but PDF storage was not wired (P1-08).
    #[cfg(feature = "postgres")]
    pub fn validate_postgres_adapters(&self) -> Result<(), String> {
        if !self.is_postgresql() {
            return Ok(());
        }
        if self.pdf_storage.is_none() {
            return Err(
                "PostgreSQL mode requires PostgresPdfStorage adapter (SPEC-017 P1-08)".into(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "postgres")]
    use edgequake_storage::adapters::memory::MemoryPdfStorage;
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
            auth_memory: Arc::new(AuthMemoryStore::new()),
            #[cfg(feature = "postgres")]
            pdf_storage: None,
            mode: StorageMode::Memory,
        };

        assert!(storage.is_memory());
        assert!(!storage.is_postgresql());
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn postgres_mode_requires_pdf_storage() {
        let kv = Arc::new(MemoryKVStorage::new("test"));
        let vector = Arc::new(MemoryVectorStorage::new("test", 1536));
        let graph = Arc::new(MemoryGraphStorage::new("test"));
        let registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry> =
            Arc::new(MemoryWorkspaceVectorRegistry::new(
                Arc::clone(&vector) as Arc<dyn edgequake_storage::traits::VectorStorage>
            ));

        let missing_pdf = StorageRuntime {
            kv_storage: Arc::clone(&kv) as Arc<dyn edgequake_storage::traits::KVStorage>,
            vector_storage: Arc::clone(&vector)
                as Arc<dyn edgequake_storage::traits::VectorStorage>,
            vector_registry: registry,
            graph_storage: Arc::clone(&graph) as Arc<dyn edgequake_storage::traits::GraphStorage>,
            auth_memory: Arc::new(AuthMemoryStore::new()),
            pdf_storage: None,
            mode: StorageMode::PostgreSQL,
        };
        assert!(missing_pdf.validate_postgres_adapters().is_err());
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn memory_mode_can_wire_pdf_storage() {
        let kv = Arc::new(MemoryKVStorage::new("test"));
        let vector = Arc::new(MemoryVectorStorage::new("test", 1536));
        let graph = Arc::new(MemoryGraphStorage::new("test"));
        let registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry> =
            Arc::new(MemoryWorkspaceVectorRegistry::new(
                Arc::clone(&vector) as Arc<dyn edgequake_storage::traits::VectorStorage>
            ));
        let pdf: Arc<dyn edgequake_storage::PdfDocumentStorage> = Arc::new(MemoryPdfStorage::new());

        let storage = StorageRuntime {
            kv_storage: Arc::clone(&kv) as Arc<dyn edgequake_storage::traits::KVStorage>,
            vector_storage: Arc::clone(&vector)
                as Arc<dyn edgequake_storage::traits::VectorStorage>,
            vector_registry: registry,
            graph_storage: Arc::clone(&graph) as Arc<dyn edgequake_storage::traits::GraphStorage>,
            auth_memory: Arc::new(AuthMemoryStore::new()),
            pdf_storage: Some(pdf),
            mode: StorageMode::Memory,
        };

        assert!(storage.is_memory());
        assert!(storage.pdf_storage.is_some());
        assert!(storage.validate_postgres_adapters().is_ok());
    }
}
