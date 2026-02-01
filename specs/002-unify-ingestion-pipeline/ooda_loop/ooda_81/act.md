# OODA-81: Act

**Date**: 2026-02-01
**Mission Re-read**: ✅

## Changes Implemented

### 1. Database Migration for Workspace-Scoped Uniqueness

**File**: `edgequake/migrations/023_workspace_scoped_content_hash.sql` (NEW)

```sql
-- Drop old global unique index
DROP INDEX IF EXISTS idx_documents_content_hash_unique;

-- Create workspace-scoped unique index
CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_workspace_content_hash_unique
    ON documents(workspace_id, content_hash)
    WHERE workspace_id IS NOT NULL
      AND content_hash IS NOT NULL
      AND status = 'indexed';
```

### 2. Handler Fix - Workspace-Scoped Hash Key

**File**: `edgequake/crates/edgequake-api/src/handlers/documents.rs`

**Lines ~2310-2325** (upload_file handler):

```rust
// BEFORE
let hash_key = format!("doc:hash:{}", content_hash);

// AFTER (OODA-81)
let hash_key = format!("doc:hash:{}:{}", workspace_id_for_storage, content_hash);
```

**Lines ~2760-2785** (process_single_file function):

```rust
// Added workspace_id parameter
async fn process_single_file(
    state: &AppState,
    filename: &str,
    content: &[u8],
    workspace_id: &str,  // NEW parameter
) -> Result<(String, bool), ApiError> {
    // ...
    let hash_key = format!("doc:hash:{}:{}", workspace_id, content_hash);
}
```

### 3. Updated docker/init.sql for Fresh Deployments

**File**: `edgequake/docker/init.sql`

**Lines ~528-535**:

```sql
-- Replaced global unique index with workspace-scoped index
CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_workspace_content_hash_unique
    ON documents(workspace_id, content_hash)
    WHERE workspace_id IS NOT NULL AND content_hash IS NOT NULL AND status = 'indexed';
```

---

## Build Verification

```
$ cargo build --package edgequake-api
   Compiling edgequake-api v0.1.0
warning: `edgequake-api` (lib) generated 4 warnings
    Finished `dev` profile in 20.27s
```

✅ Build successful (warnings are pre-existing, not related to this change)

---

## Behavior Change

| Scenario                              | Before       | After        |
| ------------------------------------- | ------------ | ------------ |
| Upload doc.pdf to Workspace A         | ✅ Success   | ✅ Success   |
| Upload doc.pdf to Workspace B         | ❌ Duplicate | ✅ Success   |
| Upload doc.pdf to Workspace A (again) | ❌ Duplicate | ❌ Duplicate |

---

## Remaining Work

- [ ] OODA-82: PDF + Markdown split view component
- [ ] OODA-83+: SRP/DRY refactoring
- [ ] E2E tests for duplicate detection

---

## Commit

Ready to commit with message:

```
OODA-81: Fix document uniqueness to be workspace-scoped

WHY: Document deduplication was global across all workspaces,
breaking multi-tenancy. Same document in different workspaces
is now allowed, while duplicates within same workspace are rejected.

- Add migration 023_workspace_scoped_content_hash.sql
- Update hash_key format: doc:hash:{workspace_id}:{content_hash}
- Update docker/init.sql for fresh deployments
```
