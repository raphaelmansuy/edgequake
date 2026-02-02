# OODA-22: Loading State UX

**Date**: 2025-01-27
**Focus**: Document Loading Experience

## OBSERVE

### Current Loading Implementation

```typescript
// pdf-viewer.tsx
const [loading, setLoading] = useState(true);

function onDocumentLoadSuccess({ numPages }: { numPages: number }) {
  setNumPages(numPages);
  setLoading(false);
}

{loading && (
  <div className="flex items-center justify-center h-64">
    <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary" />
  </div>
)}
```

### Loading Scenarios

1. **Initial load**: PDF worker initializes + document fetches
2. **Page change**: In paginated mode, new page renders
3. **View mode switch**: Side-by-side ↔ single view
4. **Network slow**: Large PDF over slow connection

### Current State

- ✅ Spinner shown during initial load
- ✅ Loading state managed per-component
- ⚠️ No progress indication for large files
- ⚠️ No skeleton loader for smoother perceived performance

## ORIENT

### First Principle: Perceived Performance

- Users perceive fast loading when:
  1. Immediate feedback (something changes)
  2. Progress indication (how much longer?)
  3. Content appears incrementally

### Options

1. **Skeleton loader**: Show document shape while loading
2. **Progress bar**: Actual download percentage
3. **Optimistic UI**: Show previous content while updating

### react-pdf Loading Events

```typescript
<Document
  onLoadProgress={({ loaded, total }) => {
    // Can show actual progress
    setProgress((loaded / total) * 100);
  }}
  onLoadSuccess={...}
  onLoadError={...}
/>
```

## DECIDE

**Decision**: Current implementation is adequate for MVP

### Rationale

- Most PDFs are small (under 1MB) - load quickly
- Spinner provides clear feedback
- Adding progress bar adds complexity for marginal gain
- Skeleton loader would require knowing document dimensions upfront

### Future Enhancement

Could add:

- `onLoadProgress` for progress indicator on large files
- Skeleton with estimated dimensions

## ACT

### Verification

Tested loading states:

- Fresh page load: Spinner visible briefly
- Cached load: Near-instant display
- Error state: Clear error message shown

### E2E Test Coverage

```typescript
test("shows loading state initially", async ({ page }) => {
  // Loading spinner should appear
  await expect(page.locator('[data-testid="pdf-loading"]')).toBeVisible();
});
```

**Status**: VERIFIED - Loading UX acceptable for current phase
