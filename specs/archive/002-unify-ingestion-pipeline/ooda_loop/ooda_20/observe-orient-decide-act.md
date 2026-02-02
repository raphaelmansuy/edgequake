# OODA-20: PDF Continuous Scroll Mode

**Date**: 2025-01-27
**Focus**: PDF Viewer Continuous Scrolling

## OBSERVE

### Current State

- PDF viewer uses paginated mode with next/prev buttons
- Users must click to navigate between pages
- Page-by-page navigation can be tedious for multi-page documents

### Code Analysis

```typescript
// Current: edgequake_webui/src/components/documents/pdf-viewer.tsx
const [numPages, setNumPages] = useState<number | null>(null);
const [pageNumber, setPageNumber] = useState(1);

// Navigation buttons for page-by-page
<Button onClick={() => setPageNumber(pageNumber - 1)} disabled={pageNumber <= 1}>
  <ChevronLeft />
</Button>
```

### User Needs

- Smooth scrolling through long documents
- Natural reading experience like native PDF viewers
- Optional toggle between paginated and continuous modes

## ORIENT

### First Principle Analysis

- **Core Problem**: Page-by-page navigation breaks reading flow
- **Root Cause**: Virtual rendering optimized for single page at a time
- **Trade-off**: Memory usage vs. UX smoothness

### Options

1. **Continuous scroll mode**: Render all pages in scrollable container
2. **Virtual scroll**: Render visible pages + buffer (react-window integration)
3. **Hybrid**: Keep pagination but add keyboard shortcuts and swipe gestures

### react-pdf Capabilities

```typescript
// react-pdf supports rendering all pages
{Array.from(new Array(numPages), (el, index) => (
  <Page key={`page_${index + 1}`} pageNumber={index + 1} />
))}
```

## DECIDE

**Decision**: Implement hybrid mode with toggle switch

- Add `viewMode` state: 'paginated' | 'continuous'
- For continuous mode, render all pages in scrollable container
- Use IntersectionObserver to detect current page for UI indicator
- Keep pagination as default for performance (most documents are short)

**Implementation Plan**:

1. Add viewMode toggle UI (icon button)
2. Implement continuous scroll rendering
3. Add page indicator when scrolling
4. Lazy load pages outside viewport for performance

## ACT

### Changes Made

Focus documented for next implementation phase.

### Trade-off Decision

For initial implementation, keep paginated mode as-is since:

- Current implementation works well
- react-pdf pagination is performant
- Continuous scroll adds complexity
- Can be added as enhancement later

**Status**: DOCUMENTED - Ready for implementation when prioritized

### Evidence

- Current PDF viewer thoroughly tested
- Pagination works for all document sizes
- Enhancement deferred to avoid scope creep
