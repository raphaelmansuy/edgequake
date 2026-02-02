# OODA-83: Orient

**Date**: 2026-02-01
**Mission Re-read**: ✅

## Analysis: Extract ContentHasher Service

### First Principles

1. **Single Responsibility**: ContentHasher does ONE thing - compute content hashes
2. **DRY**: All hash computation goes through ContentHasher
3. **Consistency**: Same output format everywhere (hex string)

### Service Design

```rust
/// Service for computing content hashes with workspace scoping.
///
/// WHY-OODA83: Consolidates hash computation (DRY) and ensures
/// consistent workspace-scoped duplicate detection.
pub struct ContentHasher;

impl ContentHasher {
    /// Compute SHA-256 hash of content bytes.
    /// Returns lowercase hex-encoded hash string.
    pub fn hash_bytes(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        hex::encode(hasher.finalize())
    }

    /// Compute SHA-256 hash of content string.
    pub fn hash_str(content: &str) -> String {
        Self::hash_bytes(content.as_bytes())
    }

    /// Generate workspace-scoped KV key for duplicate detection.
    /// Format: doc:hash:{workspace_id}:{content_hash}
    pub fn workspace_hash_key(workspace_id: &str, content_hash: &str) -> String {
        format!("doc:hash:{}:{}", workspace_id, content_hash)
    }
}
```

### Integration Points

| File                | Current Code                         | Replace With                                |
| ------------------- | ------------------------------------ | ------------------------------------------- |
| `documents.rs:521`  | `format!("{:x}", hasher.finalize())` | `ContentHasher::hash_str(&request.content)` |
| `documents.rs:2312` | `hex::encode(hasher.finalize())`     | `ContentHasher::hash_bytes(&content)`       |
| `documents.rs:2776` | `hex::encode(hasher.finalize())`     | `ContentHasher::hash_bytes(content)`        |

### File Location

Create new file: `edgequake-api/src/services/content_hasher.rs`

Also create: `edgequake-api/src/services/mod.rs`

---

## Benefits

1. **Consistency**: All hashes use same format (hex::encode)
2. **Testable**: Can unit test hash function independently
3. **Maintainable**: One place to change hash algorithm if needed
4. **Documented**: Clear WHY comments in one location

---

## Next Action

Proceed to **Decide** phase to finalize implementation steps.
