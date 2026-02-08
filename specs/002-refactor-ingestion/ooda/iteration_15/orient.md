# OODA-15 Orient: Table Row Component Design

## Gap Analysis

### Current State
- Row rendering is inline in map callback
- Multiple responsibilities: selection, display, actions
- Helper functions defined at module level (good)
- Complex className logic for row highlighting

### Target State
- Self-contained row component
- Props for all callbacks (function-as-prop pattern)
- Component handles its own styling logic
- Reusable in other table contexts

## Design Decision: Helper Functions

**Option A: Move helpers into component**
- `getFileTypeIcon` - Move to component
- `highlightMatches` - Move to component
- ❌ Breaks DRY if used elsewhere

**Option B: Keep helpers in parent, pass results**
- ❌ Clutters props

**Option C: Import from utils module**
- ✅ Clean separation
- ✅ Reusable
- Currently at top of document-manager.tsx

**Decision: Option C** - Move helpers to utils or keep in place, import in component

## Component Interface

```typescript
interface DocumentTableRowProps {
  doc: Document;
  index: number;
  isSelected: boolean;
  isActive: boolean;      // selectedDocument?.id === doc.id
  searchQuery: string;
  onSelect: (docId: string, checked: boolean) => void;
  onClick: (doc: Document) => void;
  onDoubleClick: (doc: Document) => void;
  onViewDetails: (doc: Document) => void;
  onViewInGraph: (doc: Document) => void;
  onViewPdf: (doc: Document) => void;
  onRetry: (docId: string) => void;
  onCancel: (trackId: string) => void;
  onDelete: (docId: string) => void;
  isRetrying: boolean;
  isCancelling: boolean;
}
```

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Too many props | Group related callbacks |
| Translation context | useTranslation in component |
| Re-render performance | React.memo wrap |

## Next: Decide
