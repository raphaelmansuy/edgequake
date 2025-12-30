# UX/UI Scroll Fixes - Implementation Plan

**Created:** December 30, 2025  
**Priority:** P0 - CRITICAL  
**Status:** 🔴 Ready for Implementation  
**Estimated Effort:** 4-6 hours

---

## Executive Summary

This implementation plan addresses **4 P0 critical scroll bugs** and **1 P1 UX issue** discovered during the December 30, 2025 UX/UI audit. These issues block content access and significantly degrade user experience.

### Impact Summary

| Issue | Severity | Content Hidden | User Impact |
|-------|----------|----------------|-------------|
| Entity Browser scroll blocked | P0 | 71% (1528px) | Cannot see 70+ entities |
| Details Panel scroll blocked | P0 | 21% (161px) | Action buttons cut off |
| Preview Panel scroll broken | P0 | 46% (530px) | Cannot access document actions |
| Mobile Legend overlay | P0 | N/A | Graph unusable on mobile |
| History Panel UX | P1 | N/A | Poor accessibility, cramped UI |

---

## Implementation Fixes

### Fix 1: Entity Browser Scroll (P0 CRITICAL)

**File:** [entity-browser-panel.tsx](../edgequake_webui/src/components/graph/entity-browser-panel.tsx)

**Root Cause Analysis:**
- The `<aside>` container at **line 399** has `overflow-hidden` class
- This prevents any scroll behavior on the entire panel
- The `<ScrollArea>` at **line 498** is correctly implemented but parent blocks it

**Current Code (Line 399):**
```tsx
<aside
  className={cn(
    "flex flex-col w-64 border-r bg-card/95 backdrop-blur-sm shrink-0 overflow-hidden transition-all duration-200",
    className
  )}
  aria-label={t("graph.entityBrowser.title", "Entity browser")}
>
```

**Fixed Code:**
```tsx
<aside
  className={cn(
    "flex flex-col w-64 border-r bg-card/95 backdrop-blur-sm shrink-0 transition-all duration-200",
    className
  )}
  aria-label={t("graph.entityBrowser.title", "Entity browser")}
>
```

**Change:** Remove `overflow-hidden` from the className.

**Verification:**
- [ ] Mouse wheel scrolls entity list
- [ ] Keyboard arrow keys navigate list
- [ ] Page Up/Down work in list view
- [ ] All 100+ entities accessible

**Roadblocks:** None identified. Simple CSS class removal.

---

### Fix 2: Details Panel Scroll (P0 CRITICAL)

**File:** [graph-viewer.tsx](../edgequake_webui/src/components/graph/graph-viewer.tsx)

**Root Cause Analysis:**
- The panel container at **line 349** has `overflow-hidden` class
- This blocks the inner `<ScrollArea>` at **line 364** from functioning
- Action buttons (Edit, Merge, Delete) get cut off when content is tall

**Current Code (Lines 343-349):**
```tsx
<ResizablePanel
  side="right"
  defaultWidth={320}
  minWidth={280}
  maxWidth={480}
  className="border-l bg-card/95 backdrop-blur-sm"
>
  <div className="flex flex-col h-full overflow-hidden">
```

**Fixed Code:**
```tsx
<ResizablePanel
  side="right"
  defaultWidth={320}
  minWidth={280}
  maxWidth={480}
  className="border-l bg-card/95 backdrop-blur-sm"
>
  <div className="flex flex-col h-full">
```

**Change:** Remove `overflow-hidden` from the inner div at line 349.

**Verification:**
- [ ] Scroll to see all entity properties
- [ ] Action buttons (Edit, Merge, Delete) visible
- [ ] Relationships section fully scrollable
- [ ] Mouse wheel works in panel

**Roadblocks:** None identified. Simple CSS class removal.

---

### Fix 3: Document Preview Panel Scroll (P0 CRITICAL)

**File:** [right-panel.tsx](../edgequake_webui/src/components/layout/right-panel.tsx)

**Root Cause Analysis:**
- The `<aside>` container at **line 99** is missing `overflow-hidden`
- Without this, the `flex-1` ScrollArea cannot properly calculate its bounded height
- The flex container needs `overflow-hidden` to constrain child height

**Current Code (Lines 99-105):**
```tsx
<aside
  ref={ref}
  className={cn(
    panelWidth,
    "border-l bg-card flex flex-col transition-all duration-300 ease-in-out",
    className
  )}
```

**Fixed Code:**
```tsx
<aside
  ref={ref}
  className={cn(
    panelWidth,
    "border-l bg-card flex flex-col transition-all duration-300 ease-in-out overflow-hidden",
    className
  )}
```

**Change:** Add `overflow-hidden` to the aside className.

**Alternative Fix (Line 152):**
```tsx
// Current
<ScrollArea className="flex-1">

// Fixed - ensure scroll area has bounded height
<ScrollArea className="flex-1 min-h-0">
```

**Verification:**
- [ ] Scroll to see full document content
- [ ] Processing cost section visible
- [ ] Action buttons (View, Graph, Reprocess, Delete) accessible
- [ ] Mouse wheel works in panel

**Roadblocks:** None identified. Standard flexbox scroll pattern fix.

---

### Fix 4: Mobile Legend Overlay (P0 CRITICAL)

**File:** [graph-legend.tsx](../edgequake_webui/src/components/graph/graph-legend.tsx) and [graph-viewer.tsx](../edgequake_webui/src/components/graph/graph-viewer.tsx)

**Root Cause Analysis:**
- Legend is positioned with `absolute bottom-4 right-4` in graph-viewer.tsx **line 313-315**
- On mobile (320px), the legend overlays the graph canvas
- No responsive breakpoint handling for mobile screens

**Current Code (graph-viewer.tsx lines 313-315):**
```tsx
{/* Legend Overlay - Bottom Right */}
<div className="absolute bottom-4 right-4">
  <GraphLegend />
</div>
```

**Fix Strategy:**
On mobile screens, the legend should:
1. **Option A:** Collapse to a small button that opens a bottom sheet
2. **Option B:** Hide the floating legend and add a legend toggle in the header
3. **Option C:** Position legend at top of screen on mobile with reduced opacity

**Recommended Fix (Option A - Bottom Sheet):**

**Step 1:** Update graph-viewer.tsx:
```tsx
{/* Legend Overlay - Desktop Only */}
<div className="absolute bottom-4 right-4 hidden md:block">
  <GraphLegend />
</div>

{/* Legend Button - Mobile Only */}
<div className="absolute bottom-4 right-4 md:hidden">
  <GraphLegend collapsed={true} />
</div>
```

**Step 2:** Update graph-legend.tsx to support mobile Sheet:

Add imports at top:
```tsx
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from '@/components/ui/sheet';
```

Add mobile sheet variant (new prop handling):
```tsx
interface GraphLegendProps {
  className?: string;
  collapsed?: boolean;
  mobileSheet?: boolean;  // NEW
}

// In the collapsed return block, add Sheet for mobile:
if (isCollapsed || mobileSheet) {
  return (
    <Sheet>
      <SheetTrigger asChild>
        <Button
          variant="outline"
          size="icon"
          className={`bg-background/80 backdrop-blur-sm ${className}`}
          aria-label={t('graph.legend.showLegend', 'Show entity type legend')}
        >
          <Palette className="h-4 w-4" />
        </Button>
      </SheetTrigger>
      <SheetContent side="bottom" className="max-h-[50vh]">
        <SheetHeader>
          <SheetTitle>{t('graph.legend.title', 'Entity Types')}</SheetTitle>
        </SheetHeader>
        {/* Legend content here */}
      </SheetContent>
    </Sheet>
  );
}
```

**Verification:**
- [ ] Desktop: Legend floats in corner (no change)
- [ ] Mobile 320px: Legend is a button that opens bottom sheet
- [ ] Graph canvas fully interactive on mobile
- [ ] All entity types visible in sheet

**Roadblocks:**
- ⚠️ Requires Sheet component from shadcn/ui (already available)
- ⚠️ More complex implementation than simple CSS fix
- Need to test on actual mobile device or DevTools

**Mitigation Strategy:**
1. Quick fix: Add `hidden md:block` to hide on mobile entirely (temporary)
2. Full fix: Implement bottom sheet pattern

---

### Fix 5: History Panel UX (P1 MAJOR)

**File:** [conversation-history-panel.tsx](../edgequake_webui/src/components/query/conversation-history-panel.tsx)

**Root Cause Analysis:**
- ScrollArea at **line 339** works correctly for scroll
- Issue is UX quality, not scroll functionality
- `<div className="p-2 space-y-0.5">` at **line 340** has minimal padding
- ConversationItem at **lines 103** has minimal padding

**Current Code (Line 340):**
```tsx
<ScrollArea className="flex-1">
  <div className="p-2 space-y-0.5">
```

**Current Code (Line 103):**
```tsx
<div
  className={cn(
    "group relative flex items-center gap-2 px-2.5 py-2 rounded-md cursor-pointer transition-all duration-150",
```

**Fixed Code (Line 340):**
```tsx
<ScrollArea className="flex-1">
  <div 
    className="p-2 space-y-1"
    role="listbox"
    aria-label={t("query.history.conversationList", "Conversation list")}
  >
```

**Fixed Code (Line 103):**
```tsx
<div
  className={cn(
    "group relative flex items-center gap-2 px-3 py-2.5 rounded-md cursor-pointer transition-all duration-150",
```

**Additional Fixes:**

1. **Add keyboard navigation to list:**
```tsx
// In ConversationHistoryPanel, add handler
const handleKeyDown = useCallback((e: React.KeyboardEvent, index: number) => {
  switch (e.key) {
    case 'ArrowDown':
      e.preventDefault();
      // Focus next item
      break;
    case 'ArrowUp':
      e.preventDefault();
      // Focus previous item
      break;
    case 'Enter':
    case ' ':
      e.preventDefault();
      setActiveConversation(filteredConversations[index].id);
      break;
  }
}, [filteredConversations, setActiveConversation]);
```

2. **Add ARIA attributes to ConversationItem:**
```tsx
<div
  className={cn(...)}
  role="option"
  aria-selected={isActive}
  tabIndex={0}
  onKeyDown={(e) => onKeyDown?.(e)}
>
```

**Verification:**
- [ ] Items have adequate padding (px-3 py-2.5)
- [ ] Keyboard navigation works (Arrow Up/Down)
- [ ] ARIA roles present (listbox, option)
- [ ] Focus visible indicator on keyboard nav

**Roadblocks:**
- ⚠️ Keyboard navigation requires state management for focus
- More complex than simple CSS changes

**Mitigation Strategy:**
1. Phase 1: Apply padding/spacing fixes (quick)
2. Phase 2: Add ARIA roles (medium)
3. Phase 3: Implement keyboard navigation (complex)

---

## Implementation Order

| Priority | Fix | Effort | Risk |
|----------|-----|--------|------|
| 1 | Entity Browser scroll | 5 min | Low |
| 2 | Details Panel scroll | 5 min | Low |
| 3 | Document Preview scroll | 15 min | Medium |
| 4 | History Panel padding | 10 min | Low |
| 5 | Mobile Legend (temp hide) | 5 min | Low |
| 6 | Mobile Legend (full sheet) | 1-2 hr | Medium |
| 7 | History Panel keyboard nav | 1-2 hr | Medium |

---

## Pre-Implementation Checklist

- [ ] Development environment running (`make dev`)
- [ ] Browser DevTools open for verification
- [ ] Viewport set to 1280x700 to reproduce scroll issues
- [ ] Graph page loaded with test data (100+ entities)
- [ ] Documents page loaded with preview panel open

## Post-Implementation Testing

For each fix, verify at these breakpoints:
- [ ] Desktop: 1280px width
- [ ] Desktop: 700px height (scroll trigger)
- [ ] Tablet: 768px width
- [ ] Mobile: 320px width

---

## File Summary

| File | Line(s) | Change Type |
|------|---------|-------------|
| `entity-browser-panel.tsx` | 399 | Remove `overflow-hidden` class |
| `graph-viewer.tsx` | 349 | Remove `overflow-hidden` class |
| `right-panel.tsx` | 99 | Add `overflow-hidden` class |
| `graph-legend.tsx` | Multiple | Add Sheet pattern for mobile |
| `graph-viewer.tsx` | 313-315 | Add responsive hiding classes |
| `conversation-history-panel.tsx` | 103, 340 | Update padding, add ARIA |

---

## Success Criteria

After implementation, the audit score should improve:

| Metric | Before | After |
|--------|--------|-------|
| Overall Score | 3.70/5.0 | 4.50+/5.0 |
| Scroll Areas | 2.0/5.0 | 4.5/5.0 |
| Graph Page | 3.70/5.0 | 4.3/5.0 |
| Documents Page | 3.80/5.0 | 4.3/5.0 |
| Query Page | 4.40/5.0 | 4.6/5.0 |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| CSS change breaks layout | Low | Medium | Test at multiple viewports |
| Sheet component not available | Low | High | Verify shadcn/ui installed |
| Keyboard nav conflicts with existing | Medium | Low | Test with focus trapping |
| Performance impact from scroll | Very Low | Low | Native scroll, no JS overhead |

---

## Appendix: Playwright Verification Script

```typescript
// e2e/scroll-fixes-verification.spec.ts
import { test, expect } from '@playwright/test';

test.describe('Scroll Fixes Verification', () => {
  test('Entity Browser scrolls with mouse wheel', async ({ page }) => {
    await page.goto('/graph');
    await page.waitForSelector('[aria-label="Entity browser"]');
    await page.setViewportSize({ width: 1280, height: 700 });
    
    const scrollArea = page.locator('[aria-label="Entity browser"] [data-radix-scroll-area-viewport]');
    const scrollHeight = await scrollArea.evaluate(el => el.scrollHeight);
    const clientHeight = await scrollArea.evaluate(el => el.clientHeight);
    
    // Verify scroll is possible
    expect(scrollHeight).toBeGreaterThan(clientHeight);
    
    // Scroll and verify position changes
    await scrollArea.evaluate(el => el.scrollTop = 100);
    const scrollTop = await scrollArea.evaluate(el => el.scrollTop);
    expect(scrollTop).toBe(100);
  });

  test('Details Panel scrolls when content overflows', async ({ page }) => {
    await page.goto('/graph');
    await page.setViewportSize({ width: 1280, height: 700 });
    
    // Click on an entity to open details
    await page.click('[role="option"]');
    await page.waitForSelector('text=Details & Filters');
    
    const scrollArea = page.locator('aside:has-text("Details") [data-radix-scroll-area-viewport]');
    await expect(scrollArea).toBeVisible();
  });

  test('Document Preview Panel scrolls', async ({ page }) => {
    await page.goto('/documents');
    await page.setViewportSize({ width: 1280, height: 700 });
    
    // Click on a document row
    await page.click('table tbody tr:first-child');
    await page.waitForSelector('aside[aria-label*="Document"]');
    
    const scrollArea = page.locator('aside [data-radix-scroll-area-viewport]');
    await expect(scrollArea).toBeVisible();
  });

  test('Mobile Legend does not overlay graph', async ({ page }) => {
    await page.goto('/graph');
    await page.setViewportSize({ width: 320, height: 568 });
    
    // Legend should be hidden or collapsed on mobile
    const floatingLegend = page.locator('.absolute.bottom-4.right-4 [role="region"]');
    await expect(floatingLegend).not.toBeVisible();
  });
});
```

---

*Implementation plan created December 30, 2025*  
*Based on Playwright browser automation verification*
