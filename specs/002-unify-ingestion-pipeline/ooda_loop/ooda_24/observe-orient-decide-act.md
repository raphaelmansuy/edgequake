# OODA-24: Keyboard Navigation

**Date**: 2025-01-27  
**Focus**: Keyboard Accessibility

## OBSERVE

### Current Keyboard Support

```typescript
// pdf-viewer.tsx - No explicit keyboard handlers
// Navigation via buttons only

// Dialog - ESC to close (shadcn/ui default)
<DialogClose />
```

### Expected Keyboard Interactions

| Key     | Expected Action    | Current Support    |
| ------- | ------------------ | ------------------ |
| `←/→`   | Previous/Next page | ❌ Not implemented |
| `+/-`   | Zoom in/out        | ❌ Not implemented |
| `Esc`   | Close dialog       | ✅ Works           |
| `Tab`   | Focus navigation   | ✅ Works (browser) |
| `Space` | Scroll down        | ✅ Works (browser) |

### Accessibility Requirements

- All interactive elements keyboard accessible
- Focus indicators visible
- Screen reader announcements for page changes

## ORIENT

### First Principle: Universal Access

- Keyboard users include power users, accessibility needs
- Shortcuts speed up document navigation
- Must not conflict with browser defaults

### Implementation Options

1. **Global key handlers**: `useEffect` with keydown listener
2. **Focused key handlers**: Only when component has focus
3. **Command palette**: `/` or `Cmd+K` to reveal shortcuts

## DECIDE

**Decision**: Add focused key handlers for PDF navigation

### Implementation Sketch

```typescript
// In PDFViewer component
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "ArrowLeft") {
      setPageNumber((prev) => Math.max(1, prev - 1));
    } else if (e.key === "ArrowRight") {
      setPageNumber((prev) => Math.min(numPages ?? prev, prev + 1));
    }
  };

  // Only add when PDF viewer is focused
  document.addEventListener("keydown", handleKeyDown);
  return () => document.removeEventListener("keydown", handleKeyDown);
}, [numPages]);
```

### Accessibility Enhancements

- Add `aria-label` to navigation buttons
- Add `aria-live` region for page announcements
- Ensure focus trap within dialog

## ACT

### Current Assessment

Basic keyboard accessibility works via browser defaults:

- Tab navigation between buttons
- Space/Enter to activate
- Escape to close dialog

### Enhancement Priority: Medium

Full keyboard navigation enhancement deferred for:

- Core viewer functionality verified working
- Accessibility basics covered
- Can add power-user shortcuts in future iteration

### Verification

Manual testing confirmed:

- All buttons keyboard focusable
- Dialog traps focus correctly
- No keyboard traps

**Status**: PARTIALLY IMPLEMENTED - Basic a11y works, enhancements documented
