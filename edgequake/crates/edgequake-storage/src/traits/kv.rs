//! Key-value storage trait.

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
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
    /// * `Ok(Some(value))` - Record found and deserialized
    /// * `Ok(None)` - Record not found
    /// * `Err(_)` - Error during retrieval or deserialization
    async fn get_by_id<T: DeserializeOwned + Send>(&self, id: &str) -> Result<Option<T>>;

    /// Retrieve multiple records by their IDs.
    ///
    /// # Arguments
    ///
    /// * `ids` - List of unique identifiers
    ///
    /// # Returns
    ///
    /// Vector of found records. Missing records are silently omitted.
    async fn get_by_ids<T: DeserializeOwned + Send>(&self, ids: &[String]) -> Result<Vec<T>>;

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
    async fn upsert<T: Serialize + Send + Sync>(&self, data: &[(String, T)]) -> Result<()>;

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
