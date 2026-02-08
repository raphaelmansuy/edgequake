# OODA-15 Observe: DocumentTableRow Component Extraction

## Mission Brief Re-Read

- Target: DocumentManager < 300 lines
- Current: 988 lines (45.8% reduction achieved)
- Remaining: ~688 lines to reduce

## Code Analysis

### Target: Table Row Rendering (Lines ~813-914)

The table row inside `documents.map()` renders:

1. Checkbox cell for selection
2. Title cell with:
   - File type icon
   - Highlighted search matches
   - Error message popover for failed docs
3. Status cell with EnhancedStatusBadge
4. Entity count cell
5. Cost cell
6. Created date cell with "NEW" indicator
7. Actions cell with QuickActionButtons + DocumentActionsMenu

Total: ~100 lines

### Dependencies

- `doc`, `index` from map
- `selectedIds`, `selectedDocument` for selection state
- `searchQuery` for highlighting
- `handleSelectOne`, `handleDocumentClick`, `handleDocumentDoubleClick`
- `handleViewDetails`, `handleViewInGraph`, `handleViewPdf`
- `reprocessMutation`, `cancelMutation`, `deleteMutation`
- Helper functions: `getFileTypeIcon`, `highlightMatches`
- Translation: `t`

### Props Interface

```typescript
interface DocumentTableRowProps {
  doc: Document;
  index: number;
  isSelected: boolean;
  isActive: boolean;
  searchQuery: string;
  onSelect: (checked: boolean) => void;
  onClick: () => void;
  onDoubleClick: () => void;
  onViewDetails: () => void;
  onViewInGraph: () => void;
  onViewPdf: () => void;
  onRetry: () => void;
  onCancel: (trackId: string) => void;
  onDelete: () => void;
  isRetrying: boolean;
  isCancelling: boolean;
}
```

## Line Count Estimation

- Lines removed from DocumentManager: ~95
- New component file: ~200 lines (includes docs, helpers, types)
- Net reduction: ~95 lines

## Next: Orient
