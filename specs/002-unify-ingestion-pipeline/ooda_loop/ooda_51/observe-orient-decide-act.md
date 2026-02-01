# OODA-51: Keyboard Navigation Enhancement

**Date**: 2026-02-01
**Focus**: Keyboard Accessibility in Document Views

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Keyboard-first navigation support
- Accessibility compliance (WCAG 2.1)

### Current Keyboard Support

**Document List:**
- Tab: Move between rows
- Enter: Open preview panel
- Escape: Close preview panel

**Document Detail:**
- Tab: Navigate between controls
- Arrow keys: Scroll content

### Missing Features
1. Page up/down for PDF navigation
2. +/- for zoom control
3. Left/Right for tab switching

## ORIENT

### Keyboard Navigation Matrix

| Key | Document List | Detail Page |
|-----|--------------|-------------|
| Tab | Next row | Next control |
| Enter | Open preview | Activate button |
| Escape | Close preview | Go back |
| Arrow Up/Down | Scroll list | Scroll content |
| Page Up/Down | N/A | PDF page jump |
| +/- | N/A | Zoom in/out |

## DECIDE

**Decision**: Add keyboard handlers for PDF viewer

Priority enhancements:
1. Page Up/Down for page navigation
2. +/- for zoom control
3. Home/End for first/last page

## ACT

### Implementation Plan

**PDF Viewer Enhancement:**
```typescript
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'PageDown') {
      // Scroll to next page
      e.preventDefault();
    } else if (e.key === 'PageUp') {
      // Scroll to previous page
      e.preventDefault();
    } else if (e.key === '+' || e.key === '=') {
      handleZoomIn();
    } else if (e.key === '-') {
      handleZoomOut();
    }
  };
  
  document.addEventListener('keydown', handleKeyDown);
  return () => document.removeEventListener('keydown', handleKeyDown);
}, []);
```

### Future Implementation Notes
- Use pageRefs array to track page positions
- Calculate visible page from scroll position
- Implement smooth scroll on key press

**Status**: 📋 DOCUMENTED - Enhancement path defined
