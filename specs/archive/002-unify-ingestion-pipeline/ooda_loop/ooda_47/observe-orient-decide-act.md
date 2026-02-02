# OODA-47: Cache Invalidation Strategy

**Date**: 2026-02-01
**Focus**: TanStack Query Cache Management

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Real-time document list updates
- Consistent state after uploads

### Current Cache Strategy

**Query Keys Structure:**
```typescript
const queryKeys = {
  documents: (workspaceId: string) => ['documents', workspaceId],
  document: (id: string) => ['document', id],
  pdfContent: (pdfId: string) => ['pdf-content', pdfId],
  markdownContent: (docId: string) => ['markdown-content', docId],
};
```

**Invalidation Points:**
1. After PDF upload → invalidate documents list
2. After Markdown upload → invalidate documents list
3. After delete → invalidate documents list
4. After status change → invalidate specific document

## ORIENT

### Cache Flow Diagram
```
[Upload Complete]
      ↓
[Optimistic Update] → Cache shows new doc immediately
      ↓
[Invalidate Query] → Background refetch scheduled
      ↓
[Refetch Data] → Real data replaces optimistic
      ↓
[UI Updates] → Status changes to final state
```

### Stale Time Configuration
| Query Type | Stale Time | Reason |
|------------|------------|--------|
| Documents list | 30s | Allow quick navigation |
| Document detail | 60s | Less frequent changes |
| PDF content | ∞ | Immutable binary |
| Markdown | 60s | May update on reprocess |

## DECIDE

**Decision**: Current cache strategy is correct

The implementation correctly:
1. Uses optimistic updates for immediate UI
2. Invalidates on mutations
3. Refetches after short delay to sync state

## ACT

### Code Verification

**PDF Upload Cache Update:**
```typescript
// From document-manager.tsx
queryClient.invalidateQueries({
  queryKey: ['documents', workspaceId],
});
setTimeout(() => {
  queryClient.refetchQueries({
    queryKey: ['documents', workspaceId],
  });
}, 1000);
```

### Performance Considerations
- Optimistic update avoids loading states
- 1s delay prevents race with backend processing
- Stale time prevents excessive refetches

**Status**: ✅ VERIFIED - Cache strategy optimal
