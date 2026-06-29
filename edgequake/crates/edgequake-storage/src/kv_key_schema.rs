//! Centralized KV key naming conventions.
//!
//! # WHY THIS MODULE EXISTS (SPEC-021 R-DRY-03)
//!
//! KV key patterns were previously scattered as format! string literals across
//! 8+ files. A single typo between a write path and a read path silently loses
//! data with no compile-time or runtime error.
//!
//! This module is the **single source of truth** for all KV key construction.
//! Every KV read AND write in the codebase MUST use these functions.
//!
//! # Key Taxonomy
//!
//! ```text
//! DOCUMENT KEYS
//!   {doc_id}-metadata          → DocumentMetadata JSON blob
//!   {doc_id}-chunk-{n}         → ChunkContent JSON (text, offsets, token_count)
//!   {doc_id}-chunk-             → prefix for iterating all chunks of a document
//!   {doc_id}-content            → full document content (legacy, may be absent)
//!
//! CACHE KEYS
//!   {hash}-cache               → LLM extraction cache entry (TTL-based)
//!   {hash}-kwcache             → Keyword extraction cache entry (TTL 24h)
//! ```
//!
//! # Enforces
//!
//! - **SPEC-021 R-DRY-03**: KV key patterns are a single-source-of-truth
//! - **BR0001**: Document uniqueness via content hash (written as metadata key)
//!
//! # Example
//!
//! ```rust
//! use edgequake_storage::kv_keys;
//!
//! let key = kv_keys::doc_metadata("my-doc-id");
//! assert_eq!(key, "my-doc-id-metadata");
//!
//! let chunk = kv_keys::doc_chunk("my-doc-id", 3);
//! assert_eq!(chunk, "my-doc-id-chunk-3");
//! ```

/// All KV key construction functions.
///
/// Import with `use edgequake_storage::kv_keys;` then call `kv_keys::doc_metadata(id)`.
/// All functions accept `&str`. For `String` values, pass `&my_string` or `my_string.as_str()`.
pub mod kv_keys {
    /// Key for a document's metadata JSON blob.
    ///
    /// # Schema
    /// ```json
    /// {"id":"...", "title":"...", "content_hash":"...", "chunk_count":N, ...}
    /// ```
    #[inline]
    pub fn doc_metadata(doc_id: &str) -> String {
        format!("{doc_id}-metadata")
    }

    /// Key for a specific chunk of a document.
    ///
    /// # Schema
    /// ```json
    /// {"id":"...", "content":"<text>", "chunk_index":N, "start_line":L, ...}
    /// ```
    #[inline]
    pub fn doc_chunk(doc_id: &str, index: usize) -> String {
        format!("{doc_id}-chunk-{index}")
    }

    /// Prefix for scanning ALL chunks belonging to a document.
    ///
    /// Use with `KVStorage::keys_with_prefix()`.
    #[inline]
    pub fn doc_chunk_prefix(doc_id: &str) -> String {
        format!("{doc_id}-chunk-")
    }

    /// Key for the full raw content of a document (legacy/optional).
    #[inline]
    pub fn doc_content(doc_id: &str) -> String {
        format!("{doc_id}-content")
    }

    // ── Staging keys (SPEC-026 Phase 2 P-11 admission saga) ──

    /// Staging metadata written at HTTP admit; promoted on worker success.
    #[inline]
    pub fn staging_doc_metadata(doc_id: &str) -> String {
        format!("staging:{doc_id}-metadata")
    }

    /// Staging content written at HTTP admit; promoted on worker success.
    #[inline]
    pub fn staging_doc_content(doc_id: &str) -> String {
        format!("staging:{doc_id}-content")
    }

    /// Staging workspace hash → doc_id mapping until promote.
    #[inline]
    pub fn staging_workspace_hash(workspace_id: &str, content_hash: &str) -> String {
        format!("staging:hash:{workspace_id}:{content_hash}")
    }

    /// Prefix for rollback of all staging keys for a document.
    #[inline]
    pub fn staging_doc_prefix(doc_id: &str) -> String {
        format!("staging:{doc_id}-")
    }

    /// Prefix for scanning ALL keys belonging to a document (chunks + metadata).
    ///
    /// Use with `KVStorage::keys_with_prefix()` for cascade delete.
    #[inline]
    pub fn doc_all_prefix(doc_id: &str) -> String {
        format!("{doc_id}-")
    }

    // ── Workspace document index (SPEC-027 phase 8) ──

    /// Secondary index key: `wsdoc:{workspace_id}:{document_id}` → metadata pointer JSON.
    ///
    /// Enables O(workspace docs) prefix scans instead of global `-metadata` suffix scans.
    #[inline]
    pub fn workspace_doc_index(workspace_id: &str, document_id: &str) -> String {
        format!("wsdoc:{workspace_id}:{document_id}")
    }

    /// Prefix for listing all document index entries in a workspace.
    #[inline]
    pub fn workspace_doc_index_prefix(workspace_id: &str) -> String {
        format!("wsdoc:{workspace_id}:")
    }

    /// Parse `(workspace_id, document_id)` from a workspace doc index key.
    pub fn parse_workspace_doc_index(key: &str) -> Option<(&str, &str)> {
        let rest = key.strip_prefix("wsdoc:")?;
        let (workspace_id, document_id) = rest.split_once(':')?;
        if workspace_id.is_empty() || document_id.is_empty() {
            return None;
        }
        Some((workspace_id, document_id))
    }

    /// Key for an LLM extraction cache entry.
    ///
    /// `hash` is the SHA-256 hex of the prompt + model string.
    #[inline]
    pub fn llm_cache(hash: &str) -> String {
        format!("{hash}-cache")
    }

    /// Key for a keyword extraction cache entry.
    ///
    /// `hash` is the SHA-256 hex of the query string.
    /// TTL: 24 hours (from `QueryEngineConfig::keyword_cache_ttl_secs`).
    #[inline]
    pub fn keyword_cache(hash: &str) -> String {
        format!("{hash}-kwcache")
    }

    /// Check if a KV key is a document metadata key.
    ///
    /// Returns the doc_id if it is, None otherwise.
    pub fn parse_doc_metadata(key: &str) -> Option<&str> {
        key.strip_suffix("-metadata")
    }

    /// Check if a KV key is a document chunk key.
    ///
    /// Returns `(doc_id, chunk_index)` if it is, None otherwise.
    pub fn parse_doc_chunk(key: &str) -> Option<(&str, usize)> {
        // Format: {doc_id}-chunk-{n}
        let after_chunk = key.rfind("-chunk-")?;
        let prefix = &key[..after_chunk];
        let index_str = &key[after_chunk + 7..]; // 7 = len("-chunk-")
        let index: usize = index_str.parse().ok()?;
        Some((prefix, index))
    }
}

#[cfg(test)]
mod tests {
    use super::kv_keys;

    #[test]
    fn doc_metadata_format() {
        assert_eq!(kv_keys::doc_metadata("abc-123"), "abc-123-metadata");
    }

    #[test]
    fn doc_chunk_format() {
        assert_eq!(kv_keys::doc_chunk("abc-123", 0), "abc-123-chunk-0");
        assert_eq!(kv_keys::doc_chunk("abc-123", 42), "abc-123-chunk-42");
    }

    #[test]
    fn doc_chunk_prefix_format() {
        assert_eq!(kv_keys::doc_chunk_prefix("abc-123"), "abc-123-chunk-");
    }

    #[test]
    fn doc_all_prefix_format() {
        assert_eq!(kv_keys::doc_all_prefix("abc-123"), "abc-123-");
    }

    #[test]
    fn llm_cache_format() {
        let hash = "deadbeef";
        assert_eq!(kv_keys::llm_cache(hash), "deadbeef-cache");
    }

    #[test]
    fn keyword_cache_format() {
        let hash = "feedbabe";
        assert_eq!(kv_keys::keyword_cache(hash), "feedbabe-kwcache");
    }

    #[test]
    fn parse_doc_metadata_roundtrip() {
        let doc_id = "my-doc-uuid-1234";
        let key = kv_keys::doc_metadata(doc_id);
        assert_eq!(kv_keys::parse_doc_metadata(&key), Some(doc_id));
        assert_eq!(kv_keys::parse_doc_metadata("not-a-metadata-key"), None);
        assert_eq!(kv_keys::parse_doc_metadata("foo-chunk-0"), None);
    }

    #[test]
    fn parse_doc_chunk_roundtrip() {
        let doc_id = "my-doc-uuid-1234";
        let key = kv_keys::doc_chunk(doc_id, 7);
        assert_eq!(kv_keys::parse_doc_chunk(&key), Some((doc_id, 7usize)));
        assert_eq!(kv_keys::parse_doc_chunk("not-a-chunk"), None);
        assert_eq!(kv_keys::parse_doc_chunk("abc-metadata"), None);
    }

    #[test]
    fn chunk_prefix_is_prefix_of_chunk_key() {
        let doc_id = "test-doc";
        let prefix = kv_keys::doc_chunk_prefix(doc_id);
        let chunk0 = kv_keys::doc_chunk(doc_id, 0);
        let chunk5 = kv_keys::doc_chunk(doc_id, 5);
        assert!(chunk0.starts_with(&prefix));
        assert!(chunk5.starts_with(&prefix));
    }

    #[test]
    fn workspace_doc_index_roundtrip() {
        let ws = "cccccccc-0027-0027-0027-cccccccccccc";
        let doc = "doc-abc-123";
        let key = kv_keys::workspace_doc_index(ws, doc);
        assert_eq!(key, format!("wsdoc:{ws}:{doc}"));
        assert_eq!(kv_keys::parse_workspace_doc_index(&key), Some((ws, doc)));
        assert!(kv_keys::workspace_doc_index_prefix(ws).starts_with("wsdoc:"));
    }

    #[test]
    fn all_prefix_covers_metadata_and_chunks() {
        let doc_id = "test-doc";
        let all_prefix = kv_keys::doc_all_prefix(doc_id);
        assert!(kv_keys::doc_metadata(doc_id).starts_with(&all_prefix));
        assert!(kv_keys::doc_chunk(doc_id, 0).starts_with(&all_prefix));
        assert!(kv_keys::doc_content(doc_id).starts_with(&all_prefix));
    }

    #[test]
    fn no_key_collisions_between_types() {
        let doc_id = "same-id";
        let meta = kv_keys::doc_metadata(doc_id);
        let chunk = kv_keys::doc_chunk(doc_id, 0);
        let content = kv_keys::doc_content(doc_id);
        assert_ne!(meta, chunk);
        assert_ne!(meta, content);
        assert_ne!(chunk, content);
    }
}
