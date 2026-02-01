# OODA-83: Act

**Date**: 2026-02-01
**Mission Re-read**: ✅

## Changes Implemented

### 1. Created ContentHasher Service

**File**: `edgequake-api/src/services/content_hasher.rs` (NEW)

Features:

- `hash_bytes(&[u8]) -> String` - Hash raw bytes
- `hash_str(&str) -> String` - Hash string content
- `workspace_hash_key(workspace_id, hash) -> String` - Generate scoped KV key
- 8 unit tests covering all functionality

```rust
pub struct ContentHasher;

impl ContentHasher {
    pub fn hash_bytes(content: &[u8]) -> String { ... }
    pub fn hash_str(content: &str) -> String { ... }
    pub fn workspace_hash_key(workspace_id: &str, content_hash: &str) -> String { ... }
}
```

### 2. Created Services Module

**File**: `edgequake-api/src/services/mod.rs` (NEW)

```rust
pub mod content_hasher;
pub use content_hasher::ContentHasher;
```

### 3. Updated lib.rs

**File**: `edgequake-api/src/lib.rs`

Added: `pub mod services;`

### 4. Updated documents.rs - DRY Refactoring

**File**: `edgequake-api/src/handlers/documents.rs`

| Location   | Before                               | After                                    |
| ---------- | ------------------------------------ | ---------------------------------------- |
| Line ~521  | `format!("{:x}", hasher.finalize())` | `ContentHasher::hash_str(...)`           |
| Line ~2312 | `hex::encode(hasher.finalize())`     | `ContentHasher::hash_bytes(...)`         |
| Line ~2776 | `hex::encode(hasher.finalize())`     | `ContentHasher::hash_bytes(...)`         |
| Line ~2325 | `format!("doc:hash:...`              | `ContentHasher::workspace_hash_key(...)` |
| Line ~2779 | `format!("doc:hash:...`              | `ContentHasher::workspace_hash_key(...)` |

Removed: `use sha2::{Digest, Sha256};` (no longer needed in documents.rs)

---

## Test Results

```bash
$ cargo test --package edgequake-api --lib content_hasher

running 8 tests
test services::content_hasher::tests::test_hash_bytes_consistency ... ok
test services::content_hasher::tests::test_different_content_different_hash ... ok
test services::content_hasher::tests::test_empty_content ... ok
test services::content_hasher::tests::test_workspace_hash_key_format ... ok
test services::content_hasher::tests::test_hash_lowercase ... ok
test services::content_hasher::tests::test_hash_length ... ok
test services::content_hasher::tests::test_hash_bytes_equals_hash_str ... ok
test services::content_hasher::tests::test_hash_str_consistency ... ok

test result: ok. 8 passed; 0 failed
```

✅ All tests pass

---

## DRY Improvement Summary

| Metric                     | Before | After |
| -------------------------- | ------ | ----- |
| Hash computation locations | 3      | 1     |
| Format inconsistencies     | Yes    | No    |
| Unit tests for hashing     | 0      | 8     |
| Single point of change     | No     | Yes   |

---

## Files Changed

| Action | File                         | Purpose                  |
| ------ | ---------------------------- | ------------------------ |
| CREATE | `services/mod.rs`            | Module registration      |
| CREATE | `services/content_hasher.rs` | Hash computation service |
| MODIFY | `lib.rs`                     | Add services module      |
| MODIFY | `handlers/documents.rs`      | Use ContentHasher        |

---

## Commit

Ready for commit:

```
OODA-83: Extract ContentHasher service for DRY compliance

WHY: Content hash computation was duplicated in 3 locations with
inconsistent formatting. ContentHasher consolidates this:

- Single implementation of SHA-256 hashing
- Consistent lowercase hex output
- Workspace-scoped key generation helper
- 8 unit tests for verification
```
