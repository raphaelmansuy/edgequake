# OODA Loop Iteration 01 - Orient

## First Principles Analysis

### 1. Workspace Limit of 500

**Root Cause**: The TenantPlan enum defines restrictive defaults that don't align with the target of 500 workspaces per tenant.

**First Principle**: A tenant should have generous workspace limits to enable proper organization without artificial constraints.

**Solution Approaches**:

| Approach | Pros | Cons | Recommendation |
|----------|------|------|----------------|
| A. Set all plans to 500 | Simple, meets requirement | May be wasteful for Free tier | ❌ |
| B. Enterprise = 500, scale others | Tiered approach | Free users still limited | ✅ Recommended |
| C. Remove limits entirely | Maximum flexibility | No resource governance | ❌ |

**Decision**: Option B - Set Enterprise to 500, scale others proportionally while keeping Enterprise as the "500 workspaces" tier.

Actually, reviewing the requirement "500 workspace by tenant by default" - this suggests ALL tenants should have 500 by default. Let's interpret this as:
- Enterprise = 500 (the explicit target)
- Pro = 500 (production users need this)
- Basic = 100 (reasonable for small teams)
- Free = 10 (reasonable for trials)

### 2. Document Upload 50MB

**Root Cause**: Two independent limits at 10MB:
1. `AppConfig::max_document_size` - Content validation limit
2. `ApiConfig::body_limit` - HTTP body limit

**First Principle**: Both limits must be increased together; HTTP limit must be >= content limit.

**50MB Calculation**: 50 * 1024 * 1024 = 52,428,800 bytes

**Risk Analysis**:
- Memory usage: 50MB per upload is significant
- Processing time: Large files take longer to process
- Network: Larger payloads increase latency

**Mitigation**: Use async processing for large documents (already implemented).

### 3. Workspace Deletion Cascade

**Root Cause**: `delete_workspace()` only deletes the database row, not associated data.

**First Principle**: Deleting a workspace must be a complete operation - no orphaned data should remain.

**Dependencies to clean**:

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Workspace Deletion Cascade                     │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  [Workspace Record]                                                 │
│         │                                                           │
│         ├──► [Documents Metadata] (KV storage: *-metadata)          │
│         │         │                                                 │
│         │         ├──► [Document Content] (KV storage: *-content)   │
│         │         │                                                 │
│         │         └──► [Chunks] (KV storage: *-chunk-*)             │
│         │                                                           │
│         ├──► [Embeddings] (Vector storage: workspace-scoped)        │
│         │                                                           │
│         ├──► [Entities] (Graph storage: nodes with workspace_id)    │
│         │                                                           │
│         ├──► [Relationships] (Graph storage: edges)                 │
│         │                                                           │
│         └──► [Tasks] (Task storage: workspace tasks)                │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**Solution Approaches**:

| Approach | Pros | Cons | Recommendation |
|----------|------|------|----------------|
| A. Update WorkspaceServiceImpl | Keeps logic in service layer | Needs access to all storage layers | ✅ Recommended |
| B. Update API handler | Has access to AppState | Business logic in wrong layer | ❌ |
| C. Database cascades | Automatic, reliable | Doesn't handle vector/graph | ❌ |

**Decision**: Update the API handler to call cleanup functions before calling service layer delete. This is pragmatic because:
1. API handler has access to all storage adapters via AppState
2. Service layer would need to be passed all storage references
3. The handler already has workspace deletion logic

### 4. Document Deletion (Already Working)

**Status**: ✅ Already implemented correctly in `orchestrator.rs:delete_document()`

The implementation properly cascades to:
- Chunks in KV storage
- Entities in graph storage (with source tracking)
- Relationships in graph storage (with source tracking)
- Entity embeddings in vector storage

**No changes needed.**

## Risk Assessment

| Change | Risk | Mitigation |
|--------|------|------------|
| Increase workspace limits | Low - just numbers | Test limit enforcement |
| Increase file size to 50MB | Medium - memory pressure | Async processing already in place |
| Workspace cascade delete | High - data loss if buggy | Comprehensive testing |

## Quality Gates

1. All existing tests must pass
2. Test workspace deletion cascades properly
3. Test 50MB file upload works end-to-end
4. Verify workspace limit enforcement
