# Iteration 20: Loading States Enhancement - Observe

## Current State Analysis

### Current Loading Skeleton
```tsx
{[...Array(5)].map((_, i) => (
  <Skeleton key={i} className="h-12 w-full" />
))}
```

Issues:
- Simple horizontal bars don't resemble table rows
- No column structure visible
- Doesn't set user expectations for content

### Current Empty State
```tsx
<div className="text-center py-12 text-muted-foreground">
  <FileText className="h-10 w-10 mx-auto mb-3 opacity-50" />
  <p className="font-medium">No documents yet</p>
  <p className="text-sm mt-1">Upload documents to build your knowledge graph</p>
</div>
```

Issues:
- No CTA button to upload
- Could be more engaging
- Could mention keyboard shortcut or drag-drop

### Enhancement Plan
1. Improve skeleton to show table column structure
2. Add upload button to empty state
3. Mention drag & drop in empty state

### Files to Modify
- src/components/documents/document-manager.tsx
  - Lines 975-982: Loading skeleton
  - Lines 984-989: Empty state
