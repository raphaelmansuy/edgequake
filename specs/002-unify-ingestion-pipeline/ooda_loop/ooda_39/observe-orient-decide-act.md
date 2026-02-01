# OODA-39: View Mode Toggle UX

**Date**: 2025-01-27  
**Focus**: PDF/Markdown View Switching

## OBSERVE

### Current Toggle Implementation

```typescript
// side-by-side-viewer.tsx
type ViewMode = 'pdf-only' | 'side-by-side' | 'markdown-only';

const [viewMode, setViewMode] = useState<ViewMode>('side-by-side');

// Toggle buttons in header
<div className="flex gap-1">
  <Button
    variant={viewMode === 'pdf-only' ? 'default' : 'ghost'}
    size="sm"
    onClick={() => setViewMode('pdf-only')}
  >
    PDF
  </Button>
  <Button
    variant={viewMode === 'side-by-side' ? 'default' : 'ghost'}
    size="sm"
    onClick={() => setViewMode('side-by-side')}
  >
    Split
  </Button>
  <Button
    variant={viewMode === 'markdown-only' ? 'default' : 'ghost'}
    size="sm"
    onClick={() => setViewMode('markdown-only')}
  >
    Markdown
  </Button>
</div>
```

### Visual States

| Mode            | Active Button        | Content Layout  |
| --------------- | -------------------- | --------------- |
| `pdf-only`      | PDF highlighted      | 100% PDF viewer |
| `side-by-side`  | Split highlighted    | 50/50 or custom |
| `markdown-only` | Markdown highlighted | 100% markdown   |

## ORIENT

### First Principle: Clear Affordance

- User must know current mode
- Switching must be obvious
- Animation provides feedback

### UX Quality Assessment

1. ✅ Clear visual distinction (variant styling)
2. ✅ Logical grouping (side by side)
3. ✅ Single click to switch
4. ✅ Immediate visual feedback

### Potential Improvements

- Add keyboard shortcuts (1, 2, 3)
- Add icons to buttons
- Persist preference

## DECIDE

**Decision**: Toggle UX is good

### Rationale

- Three-button approach is intuitive
- Active state clearly visible
- No confusion about current mode

## ACT

### Verification

Manual testing confirmed:

- ✅ PDF-only shows only PDF
- ✅ Side-by-side shows both
- ✅ Markdown-only shows only markdown
- ✅ Active button visually distinct

### E2E Test

```typescript
test("view mode toggle works", async ({ page }) => {
  // Click PDF only
  await page.locator('button:has-text("PDF")').click();
  await expect(page.locator(".pdf-viewer")).toBeVisible();
  await expect(page.locator(".markdown-viewer")).toBeHidden();

  // Click Split
  await page.locator('button:has-text("Split")').click();
  await expect(page.locator(".pdf-viewer")).toBeVisible();
  await expect(page.locator(".markdown-viewer")).toBeVisible();

  // Click Markdown
  await page.locator('button:has-text("Markdown")').click();
  await expect(page.locator(".pdf-viewer")).toBeHidden();
  await expect(page.locator(".markdown-viewer")).toBeVisible();
});
```

**Status**: ✅ VERIFIED - View mode toggle works correctly
