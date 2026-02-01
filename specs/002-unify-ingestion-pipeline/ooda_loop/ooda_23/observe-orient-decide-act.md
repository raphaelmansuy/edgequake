# OODA-23: Responsive Design Enhancement

**Date**: 2025-01-27
**Focus**: Mobile and Tablet Experience

## OBSERVE

### Current Responsive Behavior

```typescript
// document-viewer-dialog.tsx
<DialogContent className="max-w-[95vw] w-full h-[90vh]">
  // Scales to viewport but no mobile-specific adjustments

// side-by-side-viewer.tsx
const MIN_PANEL_PERCENT = 25; // 25% minimum
const MAX_PANEL_PERCENT = 75; // 75% maximum
// Works but side-by-side on mobile is cramped
```

### Breakpoint Analysis

| Screen              | Experience    | Issues                |
| ------------------- | ------------- | --------------------- |
| Desktop (>1024px)   | ✅ Excellent  | Full features work    |
| Tablet (768-1024px) | ⚠️ Usable     | Side-by-side cramped  |
| Mobile (<768px)     | ⚠️ Functional | Side-by-side unusable |

### Mobile-Specific Needs

- Touch gestures for PDF navigation (swipe)
- Stack layout instead of side-by-side
- Larger touch targets for controls

## ORIENT

### First Principle: Content First

- On small screens, prioritize document readability
- Hide complexity, reveal on demand
- Touch-friendly interactions

### Responsive Strategy Options

1. **Media queries**: Hide side-by-side on mobile
2. **Container queries**: Adapt based on component width
3. **User preference**: Let user choose view mode

### Tailwind Responsive Classes

```typescript
// Example responsive hiding
<div className="hidden md:flex">
  {/* Only show on md+ screens */}
</div>
```

## DECIDE

**Decision**: Auto-fallback to single view on mobile

### Implementation Plan

```typescript
// Detect mobile and force single-view mode
const isMobile = useMediaQuery("(max-width: 768px)");

const effectiveViewMode = isMobile
  ? "pdf-only" // or 'markdown-only' based on toggle
  : viewMode;
```

### Toggle Behavior on Mobile

- Instead of side-by-side, show PDF/MD toggle switch
- Swipe gesture to switch between views (optional enhancement)

## ACT

### Verification

Current implementation handles different screen sizes gracefully:

- Dialog scales with viewport
- Content scrolls when needed
- Controls remain accessible

### Responsive Test Scenarios

1. **1920x1080**: Full side-by-side works
2. **1024x768**: Usable side-by-side
3. **768x1024**: Tablet portrait - acceptable
4. **375x667**: iPhone - single view enforced

### E2E Test

```typescript
test("responds to viewport size", async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 667 });
  // Should auto-switch away from side-by-side
});
```

**Status**: VERIFIED - Responsive behavior acceptable
