# OODA-41: Document Navigation to Detail Page

**Date**: 2026-02-01
**Focus**: Double-click and View Details Navigation

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)

- Double-clicking document row MUST navigate to `/documents/[id]` page (NOT dialog)
- Each document MUST have visible "View Details" link/button
- No code duplication between list and detail views
- Use Next.js router navigation

### Current State Analysis

```typescript
// Previous: document-manager.tsx line 792
const handleDocumentDoubleClick = useCallback(
  (doc: Document) => {
    if (doc.status === "completed") {
      router.push(`/graph?entity=${encodeURIComponent(doc.id)}`);
    }
  },
  [router],
);
```

**Issues Found:**

1. Double-click navigated to `/graph`, not document detail
2. Only worked for 'completed' documents
3. No "View Details" button in document row

### Document Detail Page Exists

```
edgequake_webui/src/app/(dashboard)/documents/[id]/page.tsx
```

- Already fully implemented with content viewer
- Supports markdown/code/plain text rendering
- Has metadata sidebar
- No PDF viewing (uses ContentRenderer)

## ORIENT

### First Principle: Direct Navigation

- Users expect double-click = open item
- Graph view is secondary action
- Detail page provides full document experience

### Changes Required

1. Change `handleDocumentDoubleClick` to navigate to `/documents/${doc.id}`
2. Remove status check - allow navigation to any document
3. Add `handleViewDetails` callback
4. Add "View Details" button with ExternalLink icon

## DECIDE

**Decision**: Implement direct navigation to document detail page

### Implementation Plan

1. Import `ExternalLink` icon from lucide-react
2. Create `handleViewDetails` callback
3. Update `handleDocumentDoubleClick` to use `/documents/${doc.id}`
4. Add View Details button before Preview button in action cell
5. Update `onViewFull` in preview panel to navigate to detail page

## ACT

### Changes Made

**File**: `edgequake_webui/src/components/documents/document-manager.tsx`

1. **Added import** (line 67):

```typescript
import {
    ...
    ExternalLink,
    ...
} from 'lucide-react';
```

2. **Updated handleDocumentDoubleClick** (lines 820-828):

```typescript
/**
 * OODA-41: Double-click to navigate to document detail page
 * WHY: Power users expect double-click for primary navigation action
 * SPEC-002: Navigate to dedicated document detail page, not dialog
 */
const handleDocumentDoubleClick = useCallback(
  (doc: Document) => {
    router.push(`/documents/${doc.id}`);
  },
  [router],
);
```

3. **Added handleViewDetails** (lines 830-836):

```typescript
/**
 * OODA-41: Navigate to document detail page (for View Details button)
 * WHY: Users need explicit link to dedicated document view
 */
const handleViewDetails = useCallback(
  (doc: Document) => {
    router.push(`/documents/${doc.id}`);
  },
  [router],
);
```

4. **Added View Details button** (lines 1458-1472):

```typescript
{/* OODA-41: View Details button - navigates to document detail page */}
<TooltipProvider delayDuration={300}>
  <Tooltip>
    <TooltipTrigger asChild>
      <Button
        variant="ghost"
        size="icon"
        className="h-8 w-8"
        onClick={() => handleViewDetails(doc)}
      >
        <ExternalLink className="h-4 w-4" />
      </Button>
    </TooltipTrigger>
    <TooltipContent>View Details</TooltipContent>
  </Tooltip>
</TooltipProvider>
```

5. **Updated onViewFull in preview panel** (lines 1638-1641):

```typescript
onViewFull={(doc) => {
  // OODA-41: Always navigate to document detail page
  router.push(`/documents/${doc.id}`);
}}
```

### Evidence

- Double-click now navigates to `/documents/[id]`
- View Details button visible in each row
- Preview panel "View Full" goes to detail page
- No code duplication - reuses existing detail page

**Status**: ✅ COMPLETE - Navigation implemented
