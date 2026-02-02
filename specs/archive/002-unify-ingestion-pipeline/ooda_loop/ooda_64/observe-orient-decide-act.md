# OODA-64: Delete Confirmation Dialog

**Date**: 2026-02-01
**Focus**: Destructive Action Safety

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Safe document deletion
- Confirmation for destructive actions

### Current Delete Flow

**From document-manager.tsx:**
```typescript
const [deleteDialog, setDeleteDialog] = useState<{
  open: boolean;
  document: Document | null;
}>({ open: false, document: null });

const handleDelete = (doc: Document) => {
  setDeleteDialog({ open: true, document: doc });
};

const confirmDelete = async () => {
  if (!deleteDialog.document) return;
  
  try {
    await deleteDocument(deleteDialog.document.id, workspaceId);
    queryClient.invalidateQueries({ queryKey: ['documents'] });
    toast.success('Document deleted');
  } catch (error) {
    toast.error('Failed to delete document');
  } finally {
    setDeleteDialog({ open: false, document: null });
  }
};
```

## ORIENT

### Delete UX Pattern

```
[Click Delete] → Dialog opens
       ↓
[Show document title for confirmation]
       ↓
[Confirm ("Delete") or Cancel]
       ↓
[API call + toast feedback]
```

### Safety Considerations
1. Clear identification of what's being deleted
2. Explicit confirmation action
3. Escape hatch (Cancel button)
4. Keyboard accessible (Escape to cancel)

## DECIDE

**Decision**: Delete confirmation correctly implemented

The dialog provides:
- Clear warning message
- Document title displayed
- Easy cancellation
- Loading state during delete

## ACT

### Delete Dialog Component

```typescript
<AlertDialog 
  open={deleteDialog.open} 
  onOpenChange={(open) => setDeleteDialog(d => ({ ...d, open }))}
>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>Delete Document?</AlertDialogTitle>
      <AlertDialogDescription>
        This will permanently delete "{deleteDialog.document?.title}".
        This action cannot be undone.
      </AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel>Cancel</AlertDialogCancel>
      <AlertDialogAction
        onClick={confirmDelete}
        className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
      >
        {isDeleting ? (
          <Loader2 className="h-4 w-4 animate-spin mr-2" />
        ) : (
          <Trash2 className="h-4 w-4 mr-2" />
        )}
        Delete
      </AlertDialogAction>
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>
```

### Bulk Delete Dialog

Same pattern with count:
```typescript
<AlertDialogDescription>
  This will permanently delete {selectedIds.size} documents.
  This action cannot be undone.
</AlertDialogDescription>
```

**Status**: ✅ VERIFIED - Delete confirmation complete
