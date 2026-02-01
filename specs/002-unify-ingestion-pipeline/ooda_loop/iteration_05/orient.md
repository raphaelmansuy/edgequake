# OODA-05: Orient

## Date: 2026-02-01

## Analysis Summary

The PDF pipeline processes documents correctly but fails to propagate tenant/workspace context to document metadata, causing documents to be invisible in workspace-filtered queries.

## First Principles Analysis

### Principle: Data Must Always Have Ownership Context

In a multi-tenant system, every data artifact MUST carry:
1. `tenant_id` - isolation boundary
2. `workspace_id` - logical grouping within tenant

Without these, data becomes orphaned and inaccessible.

### Current State vs Desired State

| Component | Current State | Desired State |
|-----------|--------------|---------------|
| Task | ✓ Has workspace_id/tenant_id | No change needed |
| PDF record | ✓ Has workspace_id | No change needed |
| Entity nodes | ✓ Has workspace_id | No change needed |
| Embeddings | ✓ Has workspace_id | No change needed |
| **Document metadata** | ✗ Missing workspace_id/tenant_id | **FIX NEEDED** |

### Why This Bug Exists

The code has two paths for document creation:

1. **Markdown upload** (via `documents.rs`):
   - Creates metadata explicitly with tenant/workspace from request context
   - Works correctly

2. **PDF upload** (via `processor.rs`):
   - Creates TextInsertData with workspace_id in struct
   - But metadata JSON object doesn't include these fields
   - When `ensure_document_source_type` creates NEW metadata, it doesn't have access to context

## Possible Solutions

### Option A: Pass Context Through Metadata JSON
**Approach**: Include tenant_id/workspace_id in the metadata JSON at all stages

**Pros**:
- Simple change
- Metadata is self-contained
- Works with existing code structure

**Cons**:
- Slight duplication (workspace_id in struct + metadata)

### Option B: Change Signature of Helper Methods
**Approach**: Pass tenant/workspace as explicit parameters to `update_document_status`, `ensure_document_source_type`

**Pros**:
- Type-safe
- Can't forget to pass

**Cons**:
- Breaking change to many call sites
- More invasive

### Option C: Unified Context Struct
**Approach**: Create `DocumentContext` struct with all required fields, pass everywhere

**Pros**:
- Clean design
- Extensible

**Cons**:
- Largest change
- May be overkill for this fix

## Recommendation

**Option A** - Include tenant_id/workspace_id in metadata JSON

Rationale:
1. Minimal code change
2. Fixes the immediate bug
3. Consistent with existing pattern (other fields already in metadata)
4. Can be validated - metadata is the source of truth for document queries

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Missing context in edge cases | Add validation in processor to ensure context is always set |
| Breaking existing documents | Document doesn't modify existing data, only new uploads |
| Query performance | No impact - just adds fields to filter on |

## Validation Criteria

After fix:
1. Upload PDF via frontend
2. Task completes successfully
3. Document appears in document list for correct workspace
4. Document metadata includes tenant_id and workspace_id
5. Query on document returns results
