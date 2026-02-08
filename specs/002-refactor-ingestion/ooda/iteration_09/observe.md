# OODA Iteration 09 - OBSERVE

## Analysis

The table row section (lines 1286-1505) is ~220 lines with many dependencies:

### Row Dependencies
- doc: Document object
- selectedDocument?.id: for highlighting
- selectedIds: Set for checkbox state
- index: for zebra striping
- searchQuery: for text highlighting
- handlers: handleDocumentClick, handleDocumentDoubleClick, handleSelectOne, handleViewDetails, handleViewInGraph, handleViewPdf
- mutations: reprocessMutation, cancelMutation, deleteMutation
- t: translation function

## Decision: Extract Actions Menu First

The dropdown menu (lines 1446-1502) is ~56 lines and relatively self-contained. It needs:
- doc: Document
- onCopy, onViewPdf, onCancel, onReprocess, onDelete
- cancelMutation.isPending
- t: translation

## Component Design

```typescript
interface DocumentActionsMenuProps {
  doc: Document;
  onCopy: (id: string) => void;
  onViewPdf: (doc: Document) => void;
  onCancel: (trackId: string) => void;
  onReprocess: (id: string) => void;
  onDelete: (id: string) => void;
  isCancelling: boolean;
}
```

## Lines to Extract: ~56
