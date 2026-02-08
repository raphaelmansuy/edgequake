# OODA-14 Orient: Mutation Hook Design Analysis

## Gap Analysis

### Current State
- 4 mutations defined inline in DocumentManager
- Each mutation has:
  - `mutationFn`: API call
  - `onSuccess`: Toast + cache invalidation
  - `onError`: Toast with error message
- Mutations tightly coupled to component state (setPipelineDialogOpen)

### Target State
- Mutations extracted to reusable hook
- Component only consumes mutation functions and states
- Toast messages centralized for consistency
- Cache invalidation pattern standardized

## Design Decision: Callback vs Direct State

**Option A: Pass callback for side effects**
```typescript
useDocumentMutations({
  onPipelineDialogOpen: () => setPipelineDialogOpen(true),
});
```
- ✅ Pure separation
- ✅ Hook is fully reusable
- ❌ Callback plumbing

**Option B: Return success handler**
```typescript
const { reprocessMutation, showPipelineDialog } = useDocumentMutations();
// Component decides when to show dialog
```
- ✅ More control for component
- ❌ Exposes internal state

**Decision: Option A** - Callback pattern matches useFileUpload approach

## Hook Interface Design

```typescript
interface UseDocumentMutationsOptions {
  onReprocessSuccess?: () => void; // Open pipeline dialog
}

interface UseDocumentMutationsReturn {
  // Delete single document
  deleteMutation: UseMutationResult<void, Error, string>;
  
  // Delete all documents
  deleteAllMutation: UseMutationResult<{ deleted_count: number }, Error, void>;
  
  // Reprocess document
  reprocessMutation: UseMutationResult<void, Error, string>;
  
  // Cancel processing
  cancelMutation: UseMutationResult<void, Error, string>;
  
  // Convenience: Check if any mutation is pending
  isAnyMutationPending: boolean;
}
```

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Translation context diff | useTranslation works in hooks |
| Toast context diff | sonner toast is global |
| QueryClient scope | useQueryClient works in hooks |

## Next: Decide
