//! Content hashing service for document deduplication.
//!
//! @implements FEAT0809 - Content hash computation
//! @implements SPEC-002 - Unified Ingestion Pipeline
//!
//! # WHY-OODA83: DRY Principle
//!
//! This service consolidates content hash computation that was previously
//! duplicated in 3 locations within documents.rs. Benefits:
//!
//! 1. **Consistency**: All hashes use same format (lowercase hex)
//! 2. **Testable**: Hash logic can be unit tested independently
//! 3. **Maintainable**: Single place to modify hash algorithm
//! 4. **Documented**: Clear API with workspace-scoped key generation
//!
//! # Usage
//!
//! ```rust,ignore
//! use edgequake_api::services::ContentHasher;
//!
//! // Hash string content
//! let hash = ContentHasher::hash_str("document content");
//!
//! // Hash binary content
//! let hash = ContentHasher::hash_bytes(&file_bytes);
//!
//! // Generate workspace-scoped KV key for duplicate detection
//! let key = ContentHasher::workspace_hash_key("workspace-123", &hash);
//! // => "doc:hash:workspace-123:abc123..."
//! ```

use sha2::{Digest, Sha256};

/// Content hasher for document deduplication.
///
/// Provides consistent SHA-256 hashing with workspace-scoped key generation
/// for duplicate detection.
pub struct ContentHasher;

impl ContentHasher {
    /// Compute SHA-256 hash of raw bytes.
    ///
    /// Returns lowercase hex-encoded 64-character string.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let hash = ContentHasher::hash_bytes(b"hello world");
    /// assert_eq!(hash.len(), 64);  // 256 bits = 64 hex chars
    /// ```
    #[inline]
    pub fn hash_bytes(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        hex::encode(hasher.finalize())
    }

    /// Compute SHA-256 hash of string content.
    ///
    /// Convenience wrapper that converts string to bytes.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let hash = ContentHasher::hash_str("document text");
    /// ```
    #[inline]
    pub fn hash_str(content: &str) -> String {
        Self::hash_bytes(content.as_bytes())
    }

    /// Generate workspace-scoped KV key for duplicate detection.
    ///
    /// # WHY-OODA81: Workspace-Level Uniqueness
    ///
    /// Document uniqueness is scoped to workspace, not global.
    /// Same document can exist in different workspaces (multi-tenancy).
    ///
    /// # Format
    ///
    /// `doc:hash:{workspace_id}:{content_hash}`
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let key = ContentHasher::workspace_hash_key("ws-123", "abc...def");
    /// // => "doc:hash:ws-123:abc...def"
    /// ```
    #[inline]
    pub fn workspace_hash_key(workspace_id: &str, content_hash: &str) -> String {
        format!("doc:hash:{}:{}", workspace_id, content_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_bytes_consistency() {
        let content = b"hello world";
        let hash1 = ContentHasher::hash_bytes(content);
        let hash2 = ContentHasher::hash_bytes(content);
        assert_eq!(hash1, hash2, "Same content should produce same hash");
    }

    #[test]
    fn test_hash_str_consistency() {
        let content = "hello world";
        let hash1 = ContentHasher::hash_str(content);
        let hash2 = ContentHasher::hash_str(content);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_bytes_equals_hash_str() {
        let content = "test content";
        let hash_bytes = ContentHasher::hash_bytes(content.as_bytes());
        let hash_str = ContentHasher::hash_str(content);
        assert_eq!(hash_bytes, hash_str, "hash_bytes and hash_str should match");
    }

    #[test]
    fn test_hash_length() {
        let hash = ContentHasher::hash_str("anything");
        assert_eq!(hash.len(), 64, "SHA-256 produces 64 hex characters");
    }

    #[test]
    fn test_hash_lowercase() {
        let hash = ContentHasher::hash_str("TEST");
        assert_eq!(hash, hash.to_lowercase(), "Hash should be lowercase");
    }

    #[test]
    fn test_workspace_hash_key_format() {
        let key = ContentHasher::workspace_hash_key("workspace-123", "abc123");
        assert_eq!(key, "doc:hash:workspace-123:abc123");
    }

    #[test]
    fn test_different_content_different_hash() {
        let hash1 = ContentHasher::hash_str("content A");
        let hash2 = ContentHasher::hash_str("content B");
        assert_ne!(
            hash1, hash2,
            "Different content should produce different hash"
        );
    }

    #[test]
    fn test_empty_content() {
        let hash = ContentHasher::hash_str("");
        assert_eq!(hash.len(), 64, "Empty content still produces valid hash");
        // SHA-256 of empty string is known value
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
