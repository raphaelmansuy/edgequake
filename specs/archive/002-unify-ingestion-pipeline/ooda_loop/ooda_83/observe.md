# OODA-83: Observe

**Date**: 2026-02-01
**Mission Re-read**: ✅ Read ./specs/002-unify-ingestion-pipeline.md

## Focus: SRP and DRY Audit

### File Size Analysis

| File            | Lines | Concern                         |
| --------------- | ----- | ------------------------------- |
| `documents.rs`  | 4142  | ❌ Way too large - violates SRP |
| `pdf_upload.rs` | 1377  | ⚠️ Could be smaller             |
| **Total**       | 5519  | Need refactoring                |

### DRY Violations Found

#### 1. Content Hash Calculation (3 locations)

```rust
// Location 1: documents.rs:521 (upload_document)
let mut hasher = Sha256::new();
hasher.update(request.content.as_bytes());
let content_hash = format!("{:x}", hasher.finalize());

// Location 2: documents.rs:2312 (upload_file)
let mut hasher = Sha256::new();
hasher.update(&content);
let content_hash = hex::encode(hasher.finalize());

// Location 3: documents.rs:2776 (process_single_file)
let mut hasher = Sha256::new();
hasher.update(content);
let content_hash = hex::encode(hasher.finalize());
```

**Issue**: Inconsistent formatting too! One uses `format!("{:x}", ...)`, others use `hex::encode(...)`.

#### 2. Duplicate Check Logic (2 locations)

```rust
// Location 1: documents.rs:~2318
let hash_key = format!("doc:hash:{}:{}", workspace_id_for_storage, content_hash);
if let Some(existing_doc_id) = state.kv_storage.get_by_id(&hash_key).await? {
    // handle duplicate...
}

// Location 2: documents.rs:~2779
let hash_key = format!("doc:hash:{}:{}", workspace_id, content_hash);
if let Some(existing) = state.kv_storage.get_by_id(&hash_key).await? {
    // handle duplicate...
}
```

#### 3. Document Metadata JSON Construction (Multiple locations)

Repeated JSON construction with same fields:

- `id`, `title`, `content_hash`, `track_id`, `created_at`, `status`
- `tenant_id`, `workspace_id`
- `source_type`, `current_stage`, `stage_progress`, `stage_message`

---

## SRP Violations in documents.rs

The file handles too many concerns:

1. **Document Upload** - Text/content upload handler
2. **File Upload** - Multipart file upload handler
3. **Batch Upload** - Multiple files at once
4. **Document Listing** - List with pagination
5. **Document Detail** - Get single document
6. **Document Deletion** - Delete with cascade
7. **Track Status** - Track batch progress
8. **Workspace Vector Storage** - Storage selection logic
9. **Hash Calculation** - Content hashing
10. **Duplicate Detection** - Check for duplicates
11. **Metadata Building** - Construct JSON metadata
12. **Pipeline Processing** - Trigger pipeline

### Recommended Module Extraction

```
edgequake-api/src/
├── handlers/
│   ├── documents/
│   │   ├── mod.rs           # Re-exports
│   │   ├── upload.rs        # upload_document handler
│   │   ├── upload_file.rs   # upload_file handler
│   │   ├── batch_upload.rs  # batch upload handler
│   │   ├── list.rs          # list_documents handler
│   │   ├── detail.rs        # get_document handler
│   │   ├── delete.rs        # delete_document handler
│   │   └── track.rs         # track_status handler
│   └── pdf_upload.rs
├── services/
│   ├── mod.rs
│   ├── content_hasher.rs     # SHA-256 hashing + workspace-scoped key
│   ├── duplicate_checker.rs  # Check duplicate documents
│   └── metadata_builder.rs   # Build document metadata JSON
```

---

## Priority Analysis

Given OODA scope, focus on:

1. **P0**: Extract `ContentHasher` service (fixes DRY for 3 locations)
2. **P1**: Extract `DuplicateChecker` service (fixes DRY for 2 locations)
3. **P2**: Split documents.rs into smaller files (fixes SRP)

For this iteration, focus on P0: ContentHasher extraction.

---

## Next Action

Proceed to **Orient** phase to design ContentHasher service.
