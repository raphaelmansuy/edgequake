//! Key-value storage trait for document and metadata storage.
//!
//! # Implements
//!
//! - **FEAT0010**: Document Metadata Storage
//! - **FEAT0014**: Cache Storage (query cache, keyword cache)
//!
//! # Enforces
//!
//! - **BR0201**: Namespace-based tenant isolation
//! - **BR0001**: Document uniqueness (via content hash keys)
//!
//! # WHY: Flexible Schema Storage
//!
//! Key-value storage is used for data with varying schemas:
//! - Document metadata (title, hash, created_at)
//! - Chunk content (text, embedding_id)
//! - Cache entries (TTL-based expiration)
//! - Task state (status, progress)
//!
//! JSON values provide flexibility without schema migrations.

use async_trait::async_trait;
use std::collections::HashSet;

use crate::error::Result;

/// Key-value storage interface.
///
/// Provides a simple key-value abstraction for storing documents,
/// chunks, cache entries, and other structured data.
///
/// # Type Parameters
///
/// Methods use generic types for flexibility:
/// - Values must implement `Serialize` for storage and `DeserializeOwned` for retrieval
///
/// # Example Implementation
///
/// ```rust,ignore
/// use edgequake_storage::{KVStorage, StorageError};
/// use async_trait::async_trait;
///
/// struct MyStorage { /* ... */ }
///
/// #[async_trait]
/// impl KVStorage for MyStorage {
///     fn namespace(&self) -> &str { "my_namespace" }
///     // ... implement other methods
/// }
/// ```
#[async_trait]
pub trait KVStorage: Send + Sync {
    /// Get the storage namespace.
    ///
    /// The namespace is used to isolate different types of data
    /// (e.g., "documents", "chunks", "cache").
    fn namespace(&self) -> &str;

    /// Initialize the storage backend.
    ///
    /// This should create necessary tables, indices, or other
    /// infrastructure required for the storage to function.
    async fn initialize(&self) -> Result<()>;

    /// Flush any pending changes to persistent storage.
    ///
    /// For in-memory or buffered implementations, this ensures
    /// all data is written to the underlying storage.
    async fn finalize(&self) -> Result<()>;

    /// Retrieve a single record by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier for the record
    ///
    /// # Returns
    ///
    /// * `Ok(Some(value))` - Record found
    /// * `Ok(None)` - Record not found
    /// * `Err(_)` - Error during retrieval
    async fn get_by_id(&self, id: &str) -> Result<Option<serde_json::Value>>;

    /// Retrieve multiple records by their IDs.
    ///
    /// # Arguments
    ///
    /// * `ids` - List of unique identifiers
    ///
    /// # Returns
    ///
    /// Vector of found records. Missing records are silently omitted.
    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<serde_json::Value>>;

    /// Filter keys to find which do NOT exist in storage.
    ///
    /// This is useful for deduplication - determining which records
    /// need to be inserted vs updated.
    ///
    /// # Arguments
    ///
    /// * `keys` - Set of keys to check
    ///
    /// # Returns
    ///
    /// Set of keys that do not exist in storage.
    async fn filter_keys(&self, keys: HashSet<String>) -> Result<HashSet<String>>;

    /// Insert or update multiple records.
    ///
    /// If a record with the given ID already exists, it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `data` - Vector of (id, value) tuples to upsert
    async fn upsert(&self, data: &[(String, serde_json::Value)]) -> Result<()>;

    /// Delete records by their IDs.
    ///
    /// # Arguments
    ///
    /// * `ids` - List of IDs to delete
    ///
    /// Non-existent IDs are silently ignored.
    async fn delete(&self, ids: &[String]) -> Result<()>;

    /// Check if the storage is empty.
    async fn is_empty(&self) -> Result<bool>;

    /// Get the count of records in storage.
    async fn count(&self) -> Result<usize>;

    /// Get all keys in storage.
    async fn keys(&self) -> Result<Vec<String>>;

    /// Clear all records from storage.
    async fn clear(&self) -> Result<()>;
}

/// Extension trait for KV storage with typed access.
#[allow(dead_code)]
#[async_trait]
pub trait KVStorageExt: KVStorage {
    /// Retrieve a single record and deserialize it.
    async fn get_json<T: serde::de::DeserializeOwned + Send>(&self, id: &str) -> Result<Option<T>> {
        let val = self.get_by_id(id).await?;
        match val {
            Some(v) => Ok(Some(serde_json::from_value(v)?)),
            None => Ok(None),
        }
    }

    /// Retrieve multiple records and deserialize them.
    async fn get_jsons<T: serde::de::DeserializeOwned + Send>(
        &self,
        ids: &[String],
    ) -> Result<Vec<T>> {
        let vals = self.get_by_ids(ids).await?;
        let mut results = Vec::new();
        for v in vals {
            if let Ok(item) = serde_json::from_value(v) {
                results.push(item);
            }
        }
        Ok(results)
    }

    /// Upsert multiple records after serializing them.
    async fn upsert_json<T: serde::Serialize + Send + Sync>(
        &self,
        data: &[(String, T)],
    ) -> Result<()> {
        let mut json_data = Vec::new();
        for (id, val) in data {
            json_data.push((id.clone(), serde_json::to_value(val)?));
        }
        self.upsert(&json_data).await
    }
}

impl<T: KVStorage + ?Sized> KVStorageExt for T {}
