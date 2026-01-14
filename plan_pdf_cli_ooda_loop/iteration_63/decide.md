# OODA Iteration 63 - Decide Phase

## Selected Approach

Add individual document cancel capability with minimal UI changes.

## Implementation Decisions

### 1. Add Cancel Button to Document Dropdown

```tsx
// In DropdownMenuContent, before Reprocess
{
  (doc.status === "pending" || doc.status === "processing") && doc.track_id && (
    <DropdownMenuItem
      onClick={() => cancelMutation.mutate(doc.track_id!)}
      className="text-orange-600"
    >
      <StopCircle className="h-4 w-4 mr-2" />
      {t("documents.actions.cancel", "Cancel Extraction")}
    </DropdownMenuItem>
  );
}
```

### 2. Add Cancelled Status Config

```tsx
const statusConfig = {
  // ... existing statuses
  cancelled: {
    icon: StopCircle,
    color: "bg-orange-500",
    label: "Cancelled",
    animate: false,
  },
};
```

### 3. Add Cancel Mutation

```tsx
const cancelMutation = useMutation({
  mutationFn: async (trackId: string) => {
    await cancelTask(trackId);
  },
  onSuccess: () => {
    toast.success(
      t("documents.cancel.success", "Document processing cancelled")
    );
    queryClient.invalidateQueries({ queryKey: ["documents"] });
  },
  onError: (error) => {
    toast.error(t("documents.cancel.failed", "Cancel failed"));
  },
});
```

## Files to Modify

1. `document-manager.tsx`:
   - Import `StopCircle` icon
   - Import `cancelTask` API function
   - Add `cancelled` status to statusConfig
   - Add cancelMutation hook
   - Add Cancel menu item

## Risk Assessment

- **Low Risk**: Leverages existing backend API
- **Graceful Failure**: Cancel fails silently if already completed
- **Rollback**: Easy to remove if issues arise

## Not Included (Future Iterations)

- Bulk cancel for multiple documents
- Cancel confirmation dialog
- Cancel progress indication
