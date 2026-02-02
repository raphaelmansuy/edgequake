# OODA-36: Document Viewer Dialog Architecture

**Date**: 2025-01-27  
**Focus**: Full-Screen Modal Implementation

## OBSERVE

### Component Hierarchy

```
DocumentViewerDialog
├── DialogContent (shadcn/ui)
│   ├── Header
│   │   ├── Title
│   │   ├── View Mode Toggle
│   │   └── Actions Dropdown
│   └── Content Area
│       └── SideBySideViewer
│           ├── PDFViewer (left panel)
│           ├── Divider (resizable)
│           └── MarkdownViewer (right panel)
```

### Props Interface

```typescript
interface DocumentViewerDialogProps {
  documentId: string;
  documentTitle: string;
  isOpen: boolean;
  onClose: () => void;
}
```

### Dialog Sizing

```typescript
<DialogContent className="max-w-[95vw] w-full h-[90vh] p-0 overflow-hidden">
```

- 95% viewport width
- 90% viewport height
- Zero padding (full content use)
- Overflow hidden (internal scroll)

## ORIENT

### First Principle: Focus Mode

- Full-screen dialog removes distractions
- Single document context
- Quick escape (ESC or close button)

### Architecture Strengths

1. ✅ Clear component hierarchy
2. ✅ Props drilling minimal
3. ✅ State encapsulation
4. ✅ Responsive sizing

### shadcn/ui Benefits

- Radix Dialog primitives
- Keyboard handling (ESC)
- Focus trapping
- Animation built-in

## DECIDE

**Decision**: Architecture is well-designed

### Rationale

- shadcn/ui provides accessibility out of box
- Component composition is clean
- State management is localized
- Styling is consistent with design system

## ACT

### Verification

Dialog renders correctly in all scenarios:

- ✅ Opens when triggered
- ✅ Closes on ESC key
- ✅ Closes on backdrop click
- ✅ Closes on close button
- ✅ Focus trapped within dialog

### E2E Test

```typescript
test("dialog opens and closes", async ({ page }) => {
  // Open dialog
  await page.locator('[data-testid="view-document"]').click();
  await expect(page.locator('[role="dialog"]')).toBeVisible();

  // Close with ESC
  await page.keyboard.press("Escape");
  await expect(page.locator('[role="dialog"]')).toBeHidden();
});
```

### Integration Points

- Data fetching via TanStack Query
- State via React useState
- Styling via Tailwind
- Animations via shadcn/ui

**Status**: ✅ COMPLETE - Dialog architecture verified
