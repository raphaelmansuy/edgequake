# OODA-62: Row Selection

**Date**: 2026-02-01
**Focus**: Multi-select Document Actions

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Bulk document actions
- Multi-select UI pattern

### Current Selection Implementation

**From document-manager.tsx:**
```typescript
const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

const toggleSelection = (id: string) => {
  setSelectedIds(current => {
    const next = new Set(current);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    return next;
  });
};

const selectAll = () => {
  setSelectedIds(new Set(paginatedDocuments.map(d => d.id)));
};

const clearSelection = () => {
  setSelectedIds(new Set());
};
```

## ORIENT

### Selection Features

| Feature | Status | Notes |
|---------|--------|-------|
| Single select | ✅ | Checkbox per row |
| Select all (page) | ✅ | Header checkbox |
| Bulk delete | ✅ | Action bar appears |
| Bulk download | ❌ | Future enhancement |

### Selection UX
```
[Check row] → Row highlights, selection count shows
[Check header] → All visible rows selected
[Bulk action] → Confirmation dialog
[Action complete] → Selection cleared
```

## DECIDE

**Decision**: Selection implementation is correct

The pattern provides:
- Clear visual feedback
- Efficient state management
- Safe bulk actions with confirmation

## ACT

### Checkbox Column

```typescript
<TableRow className={cn(selectedIds.has(doc.id) && "bg-muted/50")}>
  <TableCell>
    <Checkbox
      checked={selectedIds.has(doc.id)}
      onCheckedChange={() => toggleSelection(doc.id)}
    />
  </TableCell>
  {/* ... other cells */}
</TableRow>
```

### Bulk Action Bar

```typescript
{selectedIds.size > 0 && (
  <div className="fixed bottom-4 left-1/2 -translate-x-1/2 flex items-center gap-4 bg-background border rounded-lg shadow-lg px-4 py-2">
    <span className="text-sm">
      {selectedIds.size} document{selectedIds.size > 1 ? 's' : ''} selected
    </span>
    <Button
      variant="destructive"
      size="sm"
      onClick={handleBulkDelete}
    >
      <Trash2 className="h-4 w-4 mr-2" />
      Delete
    </Button>
    <Button
      variant="ghost"
      size="sm"
      onClick={clearSelection}
    >
      Cancel
    </Button>
  </div>
)}
```

**Status**: ✅ VERIFIED - Row selection complete
