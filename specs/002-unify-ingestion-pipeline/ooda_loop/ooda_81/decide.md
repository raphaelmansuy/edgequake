# OODA-81: Decide

**Date**: 2026-02-01
**Mission Re-read**: ✅

## Decision: Fix Workspace-Scoped Document Uniqueness

### Specific Changes to Implement

#### 1. Database Migration

**File**: `edgequake/migrations/004_workspace_scoped_hash.sql` (NEW)

```sql
-- Drop global unique index
DROP INDEX IF EXISTS idx_documents_content_hash_unique;

-- Create workspace-scoped unique index
CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_workspace_content_hash_unique
    ON documents(workspace_id, content_hash)
    WHERE content_hash IS NOT NULL AND status = 'indexed';
```

#### 2. Update Handler Hash Key

**File**: `edgequake/crates/edgequake-api/src/handlers/documents.rs`

**Lines 2316-2318 (file upload)**:

```rust
// BEFORE
let hash_key = format!("doc:hash:{}", content_hash);

// AFTER
let hash_key = format!("doc:hash:{}:{}", workspace_id_for_storage, content_hash);
```

**Similar change needed for text upload handler.**

#### 3. Also update docker/init.sql

For fresh deployments, update the index in the init script.

---

### Implementation Order

1. Create migration file
2. Update documents.rs hash_key construction
3. Update docker/init.sql for fresh deployments
4. Run tests to verify
5. Create E2E test for duplicate detection

---

## Acceptance Criteria

- [ ] Same document in different workspaces → Both allowed
- [ ] Same document in same workspace → Second rejected with 409/duplicate
- [ ] All existing tests pass
- [ ] New E2E test validates workspace-scoped uniqueness

---

## Next Action

Proceed to **Act** phase to implement these changes.
