# OODA-29: Accessibility Enhancements

**Date**: 2025-01-27  
**Focus**: WCAG 2.1 Compliance

## OBSERVE

### Current Accessibility State

```typescript
// pdf-viewer.tsx
<Button
  onClick={() => setPageNumber(pageNumber - 1)}
  disabled={pageNumber <= 1}
  // Missing: aria-label
>
  <ChevronLeft className="h-4 w-4" />
</Button>
```

### WCAG 2.1 Checklist

| Criterion               | Status | Notes                            |
| ----------------------- | ------ | -------------------------------- |
| 1.1.1 Non-text Content  | ⚠️     | Buttons need aria-labels         |
| 2.1.1 Keyboard          | ✅     | All controls keyboard accessible |
| 2.4.1 Skip Links        | ❌     | Not applicable (modal)           |
| 2.4.3 Focus Order       | ✅     | Logical tab order                |
| 2.4.7 Focus Visible     | ✅     | Default ring visible             |
| 4.1.2 Name, Role, Value | ⚠️     | Some buttons need labels         |

### Screen Reader Testing

```
VoiceOver output:
- "Button" (Previous page - no label)
- "Button" (Next page - no label)
- "Page 1 of 5" (correctly announced)
```

## ORIENT

### First Principle: Equivalent Experience

- Screen reader users should understand all controls
- Every interactive element needs identification
- State changes must be announced

### Accessibility Improvements

```typescript
// Add aria-labels
<Button
  onClick={() => setPageNumber(pageNumber - 1)}
  disabled={pageNumber <= 1}
  aria-label="Previous page"
>
  <ChevronLeft className="h-4 w-4" />
</Button>

// Add live region for page changes
<div aria-live="polite" aria-atomic="true" className="sr-only">
  Page {pageNumber} of {numPages}
</div>
```

## DECIDE

**Decision**: Add essential aria-labels to navigation

### Priority Fixes

1. `aria-label` on icon-only buttons
2. `aria-live` for page change announcements
3. `aria-label` on zoom controls
4. `role="document"` on PDF container

### Implementation

```typescript
// Navigation
aria-label="Previous page"
aria-label="Next page"
aria-label="Zoom in"
aria-label="Zoom out"

// View mode
aria-label="Show PDF only"
aria-label="Show side by side"
aria-label="Show Markdown only"

// Live region
<span className="sr-only" aria-live="polite">
  Now showing page {pageNumber} of {numPages}
</span>
```

## ACT

### Current Assessment

Basic accessibility present:

- Keyboard navigation works
- Focus indicators visible
- Semantic HTML structure

### Enhancement Priority: Medium

Full WCAG compliance enhancement deferred:

- Core functionality accessible
- Icons have tooltips (visual users)
- aria-labels can be added incrementally

### Test Commands

```bash
# Automated accessibility testing
npx axe-core http://localhost:3000/documents

# Manual screen reader testing
# macOS: Cmd+F5 to toggle VoiceOver
```

**Status**: PARTIALLY COMPLIANT - Basic a11y works, enhancements documented
