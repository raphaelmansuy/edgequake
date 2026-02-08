# OODA-14 Observe: useDocumentMutations Hook Extraction

## Mission Brief Re-Read

- Target: DocumentManager < 300 lines
- Current: 1064 lines (41.6% reduction achieved)
- Remaining: ~764 lines to reduce

## Code Analysis

### Target: Mutation Definitions (Lines ~280-370)

Four React Query mutations currently inline in DocumentManager:

```typescript
// deleteMutation (~25 lines)
const deleteMutation = useMutation({
  mutationFn: deleteDocument,
  onSuccess: () => { toast.success(...); queryClient.invalidateQueries(...); },
  onError: (error) => { toast.error(...); },
});

// deleteAllMutation (~20 lines)
const deleteAllMutation = useMutation({
  mutationFn: deleteAllDocuments,
  onSuccess: (data) => { toast.success(...); queryClient.invalidateQueries(...); },
  onError: (error) => { toast.error(...); },
});

// reprocessMutation (~25 lines)
const reprocessMutation = useMutation({
  mutationFn: (documentId: string) => reprocessDocument(documentId, true),
  onSuccess: () => { toast.success(...); queryClient.invalidateQueries(...); },
  onError: (error) => { toast.error(...); },
});

// cancelMutation (~20 lines)
const cancelMutation = useMutation({
  mutationFn: async (trackId: string) => { await cancelTask(trackId); },
  onSuccess: () => { toast.success(...); queryClient.invalidateQueries(...); },
  onError: (error) => { toast.error(...); },
});
```

Total: ~90 lines of mutation definitions

### Dependencies

- `toast` from sonner
- `queryClient` from useQueryClient
- `t` from useTranslation
- `setPipelineDialogOpen` callback (for reprocess success action)

### Usage Sites

1. `deleteMutation.mutate(id)` - Delete single document
2. `deleteAllMutation.mutate()` - Not directly used (Clear Documents Dialog handles its own)
3. `reprocessMutation.mutate(id)` - Retry failed document
4. `cancelMutation.mutate(trackId)` - Cancel processing
5. `deleteMutation.isPending` - Loading state
6. `reprocessMutation.isPending` - Loading state
7. `cancelMutation.isPending` - Loading state

### Extract Pattern (Matches useFileUpload)

```typescript
// hooks/use-document-mutations.ts
export function useDocumentMutations(options: {
  onPipelineDialogOpen?: () => void;
}) {
  // Define mutations
  return {
    deleteMutation,
    deleteAllMutation,
    reprocessMutation,
    cancelMutation,
  };
}
```

## Line Count Estimation

- Lines removed from DocumentManager: ~90
- New hook file: ~150 lines (includes docs, types, error handling)
- Net reduction: ~90 lines

## Next: Orient
