# OODA Iteration 07 - OBSERVE

## Finding

The `DocumentFilters` component already exists! It's imported from `./document-filters`.

## New Target: BatchActionsBar

Instead, I'll extract the bulk actions bar (lines 1223-1247):

```tsx
{selectedIds.size > 0 && (
  <div className="shrink-0 px-4 py-2 bg-muted/50 border-b flex items-center justify-between">
    <div className="flex items-center gap-3">
      <span className="text-sm font-medium">
        {t('documents.bulk.selected', { count: selectedIds.size }) || `${selectedIds.size} document(s) selected`}
      </span>
      {/* Keyboard hint */}
      <span className="text-xs text-muted-foreground hidden sm:inline">
        Press <kbd className="px-1 py-0.5 bg-muted rounded text-[10px]">Esc</kbd> to clear
      </span>
    </div>
    <div className="flex items-center gap-2">
      <Button variant="outline" size="sm" onClick={handleBulkReprocess}>...</Button>
      <Button variant="outline" size="sm" onClick={handleBulkDelete}>...</Button>
      <Button variant="ghost" size="sm" onClick={() => setSelectedIds(new Set())}>...</Button>
    </div>
  </div>
)}
```

## Dependencies

- selectedIds.size: number
- handleBulkReprocess: () => void
- handleBulkDelete: () => void
- setSelectedIds: (Set) => void  
- t: translation function

## Component Design

```typescript
interface BatchActionsBarProps {
  selectedCount: number;
  onReprocess: () => void;
  onDelete: () => void;
  onClear: () => void;
}
```

## Lines to Extract: ~25
