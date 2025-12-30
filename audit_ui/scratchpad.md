# UX/UI Audit Scratchpad - December 30, 2025

This is an append-only log of observations during the audit process.

---

## Session Start: 2025-12-30

### Initial State

- Existing audit from December 25, 2025 found
- 75 screenshots already captured
- Focus areas: Graph, Document, Query pages
- Special attention to scroll areas and panel behaviors

---

## Implementation Session: 2025-12-30

### Key Discovery: Flexbox Scroll Pattern

During Playwright testing, discovered the initial fix approach was incorrect:

**Wrong approach:** Removing `overflow-hidden` from parent
- Result: ScrollArea viewport expanded freely (1945px) beyond container bounds (619px)
- ScrollArea viewport was not constrained, preventing scroll behavior

**Correct approach:** Keep `overflow-hidden` on parent AND add `min-h-0` to ScrollArea
- Parent needs `overflow-hidden` to clip children
- Child needs `min-h-0` to reset flexbox minimum height from content

### Files Modified

1. **entity-browser-panel.tsx** (line 514)
   - Added `min-h-0` to ScrollArea className
   - Kept `overflow-hidden` on aside (line 399)

2. **graph-viewer.tsx**
   - Line 349: Added `overflow-hidden` to Details Panel container
   - Line 365: Added `min-h-0` to ScrollArea
   - Lines 313-315: Added `hidden md:block` to Legend container for mobile hide

3. **right-panel.tsx** (line 152)
   - Added `min-h-0` to ScrollArea

4. **conversation-history-panel.tsx** (line 103)
   - Changed padding from `px-2.5 py-2` to `px-3 py-2.5`
   - Changed role from "button" to "option"
   - Added `aria-selected={isActive}`

### E2E Verification Results

**Entity Browser (1280x700 viewport):**
```json
{
  "asideHeight": 619,
  "asideOverflow": "hidden",
  "scrollViewportHeight": 417,
  "scrollHeight": 1945,
  "canScroll": true,
  "scrollDiff": 1528
}
```

**Details Panel (with entity selected):**
```json
{
  "panelHeight": 619,
  "panelOverflow": "hidden",
  "scrollViewportHeight": 578,
  "scrollHeight": 739,
  "canScroll": true,
  "scrollDiff": 161
}
```

**Mobile Legend (375x800 viewport):**
```json
{
  "parentDisplay": "none",
  "isHidden": true
}
```

**Desktop Legend (1280x900 viewport):**
```json
{
  "parentDisplay": "block",
  "isHidden": false
}
```

### Lessons Learned

1. **Testing is essential** - Initial fix appeared correct in code but failed in practice
2. **Flexbox scroll is tricky** - Both parent constraint AND child min-height reset needed
3. **Playwright evaluation** - Excellent for measuring actual DOM dimensions
4. **Radix ScrollArea** - Uses `data-radix-scroll-area-viewport` for scroll container

---
