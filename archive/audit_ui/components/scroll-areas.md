# Scroll Areas & Fixed Zones Audit

**Date:** December 30, 2025  
**Priority:** CRITICAL (per spec: "VERY IMPORTANT")  
**Status:** 🚨 4 P0 CRITICAL BUGS FOUND

---

## 🚨 CRITICAL BUGS SUMMARY

| Bug                           | Location       | Root Cause          | Hidden Content |
| ----------------------------- | -------------- | ------------------- | -------------- |
| Entity Browser NOT scrollable | Graph page     | `overflow: hidden`  | 71% hidden     |
| Details Panel NOT scrollable  | Graph page     | `overflow: hidden`  | 161px hidden   |
| Preview Panel NOT scrollable  | Documents page | `overflow: visible` | 46% hidden     |
| Mobile Legend overlay         | Graph page     | Not responsive      | Blocks canvas  |

**Scroll Score: 2.0/5.0** (was 4.5/5.0)

---

## Overview

This audit documents the scroll behavior and fixed zone architecture across EdgeQuake WebUI. Proper scroll handling is essential for professional UX, especially in data-heavy applications like knowledge graph management.

---

## Global Fixed Zones

### Application Shell

```
┌─────────────────────────────────────────────────────────┐
│ FIXED: Header (64px)                                    │
├────────────┬────────────────────────────────────────────┤
│ FIXED:     │ FIXED: Breadcrumb (varies)                 │
│ Sidebar    ├────────────────────────────────────────────┤
│ (256px     │                                            │
│ expanded,  │ SCROLLABLE: Main Content Area              │
│ 64px       │                                            │
│ collapsed) │                                            │
│            │                                            │
│            │                                            │
└────────────┴────────────────────────────────────────────┘
```

### Fixed Element Specifications

| Element    | Position       | Height/Width            | CSS Classes                   | Status |
| ---------- | -------------- | ----------------------- | ----------------------------- | ------ |
| Header     | `fixed top-0`  | h: 64px, w: 100%        | `sticky top-0 z-50`           | ✅     |
| Sidebar    | `fixed left-0` | w: 256px/64px, h: 100vh | `fixed left-0 top-0 h-screen` | ✅     |
| Breadcrumb | `sticky`       | h: ~40px                | `sticky top-[64px]`           | ✅     |

---

## Page-Specific Scroll Behavior

### Graph Page (`/graph`)

```
┌────────────────────────────────────────────────────────┐
│ FIXED: Entity Browser Panel (256px)                    │
│ ┌────────────────────────────────────────┐ FIXED:      │
│ │ SCROLLABLE: Entity List                │ Controls    │
│ │ (internal scroll)                      │ (floating)  │
│ └────────────────────────────────────────┘             │
│                                                        │
│ PANNABLE: Graph Canvas (zoom/pan, not scroll)          │
│                                                        │
│ FIXED: Legend                                          │
└────────────────────────────────────────────────────────┘
```

| Scroll Container | Behavior                                | Status    |
| ---------------- | --------------------------------------- | --------- |
| Entity List      | Internal scroll with `overflow-y: auto` | ❌ BROKEN |
| Graph Canvas     | Pan/zoom (not traditional scroll)       | ✅        |
| Details Panel    | Internal scroll when content overflows  | ❌ BROKEN |

**🔴 Issue 1:** Entity Browser has `overflow: hidden` - mouse wheel and keyboard navigation blocked.
**🔴 Issue 2:** Details Panel has `overflow: hidden` - action buttons cut off.
**🔴 Issue 3:** On mobile (320px), Legend component overlays graph canvas. Needs fixing.

---

### Query Page (`/query`)

```
┌────────────────────────────────────────────────────────┐
│ FIXED: Mode Selector & New Chat Button                 │
├────────────────────────────────────────────────────────┤
│ SCROLLABLE: Conversation Messages                      │
│ (auto-scrolls to bottom on new message)                │
├────────────────────────────────────────────────────────┤
│ FIXED: Input Area                                      │
└────────────────────────────────────────────────────────┘
```

| Scroll Container | Behavior                                            | Status     |
| ---------------- | --------------------------------------------------- | ---------- |
| Message List     | Scrolls with auto-scroll-to-bottom during streaming | ✅         |
| History Panel    | Internal scroll for conversation list               | ⚠️ UX POOR |
| Input Area       | Fixed at bottom, expands with textarea              | ✅         |

**✅ Excellent:** Input area remains fixed during conversation scroll. Auto-scroll during streaming works flawlessly.
**🟡 Issue:** History Panel has 0px padding, cramped items, missing ARIA roles, no keyboard navigation.

---

### Documents Page (`/documents`)

```
┌────────────────────────────────────────────────────────┐
│ FIXED: Filters & Upload Area (stacked)                 │
├────────────────────────────────────────────────────────┤
│ SCROLLABLE: Document List                              │
│ (table with pagination, internal scroll if needed)     │
├────────────────────────────────────────────────────────┤
│ FIXED: Pagination Controls                             │
└────────────────────────────────────────────────────────┘

Preview Panel (Right):
┌────────────────────────────────────────────────────────┐
│ FIXED: Header (Document Title)                         │
├────────────────────────────────────────────────────────┤
│ SCROLLABLE: Metadata & Actions                         │
└────────────────────────────────────────────────────────┘
```

| Scroll Container | Behavior                           | Status    |
| ---------------- | ---------------------------------- | --------- |
| Document List    | Scrolls when list exceeds viewport | ✅        |
| Preview Panel    | Internal scroll for metadata       | ❌ BROKEN |
| Filters          | Fixed, always visible              | ✅        |

**🔴 Issue:** Preview Panel has `overflow: visible` - content overflows, 46% hidden.

---

### Document Detail Page (`/documents/[id]`)

```
┌────────────────────────────────────────────────────────┐
│ FIXED: Document Header (Title, Status, Actions)        │
├───────────────────────────┬────────────────────────────┤
│ SCROLLABLE:               │ FIXED:                     │
│ Full Document Content     │ Metadata Sidebar           │
│ (66,290px for large doc)  │ (internal scroll)          │
│                           │                            │
│ - Rendered Markdown       │ - Extraction Lineage       │
│ - References              │ - Knowledge Graph Stats    │
│ - Tables                  │ - Source Details           │
│                           │ - Processing Info          │
└───────────────────────────┴────────────────────────────┘
```

**Verified Measurements:**

- Scrollable container: `div.flex-1.overflow-auto`
- Scroll height: 66,290px (for 134K character academic paper)
- Client height: 731px
- Scroll works smoothly with no jank

| Scroll Container   | Behavior                               | Status |
| ------------------ | -------------------------------------- | ------ |
| Main Content       | `overflow-auto` on flex container      | ✅     |
| Metadata Sidebar   | Internal scroll for accordion sections | ✅     |
| Extraction Lineage | Collapsible, no scroll needed          | ✅     |

---

## Scroll Technical Implementation

### CSS Pattern Used

```css
/* Main scrollable container */
.flex-1.overflow-auto {
  flex: 1 1 0%;
  overflow-y: auto;
  overflow-x: hidden;
}

/* Fixed sidebar */
.fixed.left-0.top-0.h-screen {
  position: fixed;
  left: 0;
  top: 0;
  height: 100vh;
}

/* Sticky header */
.sticky.top-0.z-50 {
  position: sticky;
  top: 0;
  z-index: 50;
}
```

### Scroll Indicators

| Feature              | Implementation                     | Status      |
| -------------------- | ---------------------------------- | ----------- |
| Scrollbar visibility | Native scrollbar, styled           | ✅          |
| Scroll shadows       | Not implemented                    | 🟡 Consider |
| Infinite scroll      | Not needed (pagination used)       | N/A         |
| Pull-to-refresh      | Not implemented (not mobile-first) | N/A         |

---

## Responsive Scroll Behavior

### Mobile (320px)

| Page      | Scroll Behavior                   | Status          |
| --------- | --------------------------------- | --------------- |
| Graph     | Full page scroll, panels hidden   | ⚠️ Legend issue |
| Query     | Message scroll works, input fixed | ✅              |
| Documents | Full page scroll                  | ✅              |

### Tablet (768px)

| Page      | Scroll Behavior                            | Status |
| --------- | ------------------------------------------ | ------ |
| Graph     | Entity panel collapsible, graph scrollable | ✅     |
| Query     | Side panels collapsible                    | ✅     |
| Documents | Preview panel collapsible                  | ✅     |

### Desktop (1280px+)

| Page      | Scroll Behavior                     | Status |
| --------- | ----------------------------------- | ------ |
| Graph     | Multi-panel with internal scrolls   | ✅     |
| Query     | Three-column with internal scrolls  | ✅     |
| Documents | Master-detail with internal scrolls | ✅     |

---

## Issues Found

### 🔴 Critical (P0) - December 30, 2025 Update

#### 1. Entity Browser Panel NOT SCROLLABLE (Graph Page)

**Priority:** P0 - CRITICAL  
**Severity:** Blocks usability completely

| Attribute          | Value                                    |
| ------------------ | ---------------------------------------- |
| Location           | Graph page → Entity Browser (left panel) |
| Container          | `aside[aria-label="Entity browser"]`     |
| Root Cause         | Parent container has `overflow: hidden`  |
| Content Height     | 2147px                                   |
| Visible Height     | 619px                                    |
| **Hidden Content** | **1528px (71% of content inaccessible)** |

**Technical Analysis:**

```css
/* CURRENT (BUG) */
.entity-browser-container {
  overflow: hidden; /* ← BLOCKS ALL SCROLL */
}

/* SHOULD BE */
.entity-browser-container {
  overflow-y: auto;
  overflow-x: hidden;
}
```

**Impact:**

- Mouse wheel scroll does NOT work
- Keyboard navigation (arrow keys, Page Up/Down) does NOT work
- Users cannot see 51 UNKNOWN entities, many PRODUCT entities, etc.
- With 100 entities, only ~30% are visible

**Fix Required:**
Change the Entity Browser container from `overflow-hidden` to `overflow-y-auto`.

---

#### 2. Details & Filters Panel NOT SCROLLABLE (Graph Page)

**Priority:** P0 - CRITICAL  
**Severity:** Blocks usability when entity has many relationships

| Attribute          | Value                                        |
| ------------------ | -------------------------------------------- |
| Location           | Graph page → Details & Filters (right panel) |
| Container          | `div.flex.flex-col.h-full.overflow-hidden`   |
| Root Cause         | Container has `overflow: hidden`             |
| Content Height     | 780px                                        |
| Visible Height     | 619px                                        |
| **Hidden Content** | **161px (Edit/Merge/Delete buttons hidden)** |

**Technical Analysis:**

```css
/* CURRENT (BUG) */
.details-panel {
  overflow: hidden; /* ← BLOCKS SCROLL */
}

/* SHOULD BE */
.details-panel {
  overflow-y: auto;
}
```

**Impact:**

- Cannot scroll to see all entity properties
- Action buttons (Edit, Merge, Delete) may be cut off
- Long relationship lists truncated

---

#### 3. Document Preview Panel NOT SCROLLABLE (Documents Page)

**Priority:** P0 - CRITICAL  
**Severity:** Blocks access to document actions

| Attribute          | Value                                              |
| ------------------ | -------------------------------------------------- |
| Location           | Documents page → Preview Panel (right panel)       |
| Container          | `aside[aria-label="document-name"]`                |
| Root Cause         | Container has `overflow: visible` (not scrollable) |
| Content Height     | 1149px                                             |
| Visible Height     | 619px                                              |
| **Hidden Content** | **530px (Action buttons potentially hidden)**      |

**Technical Analysis:**

```css
/* CURRENT (BUG) */
.preview-panel {
  overflow: visible; /* ← NO SCROLL CAPABILITY */
}

/* SHOULD BE */
.preview-panel-content {
  flex: 1;
  overflow-y: auto;
}
```

**Impact:**

- Cannot scroll to see full content preview
- Action buttons (View Details, Graph, Reprocess, Delete) may be hidden
- Processing cost details may be cut off

---

#### 4. Graph Mobile Legend Overlay

**Priority:** P0 - CRITICAL  
**Severity:** Graph unusable on mobile

- Legend blocks graph canvas on 320px viewport
- Recommendation: Move to bottom sheet or collapse to chip

---

### 🟠 Major (P1)

#### 5. Conversation History Panel UX Issues (Query Page)

**Priority:** P1 - Major  
**Location:** Query page → History Panel (right side)

| Issue                     | Current                    | Recommended                        |
| ------------------------- | -------------------------- | ---------------------------------- |
| Scroll container          | `overflow-auto` (✅ works) | No change needed                   |
| Container padding         | `0px`                      | `8px` or `12px` for breathing room |
| Conversation item padding | `6px 12px`                 | `8px 16px` for better touch target |
| List role                 | Missing                    | Add `role="listbox"`               |
| Item role                 | Missing                    | Add `role="option"`                |
| Keyboard navigation       | Tab only                   | Add arrow key navigation           |

**Recommended Fix:**

```tsx
// Add proper ARIA roles and keyboard handling
<div
  role="listbox"
  aria-label="Conversation history"
  onKeyDown={handleArrowNavigation}
>
  {conversations.map((conv) => (
    <button
      role="option"
      aria-selected={isSelected}
      className="px-4 py-2" // Increased padding
    >
      {conv.name}
    </button>
  ))}
</div>
```

---

### 🟡 Minor

1. **No Scroll Shadows**

   - Consider adding shadow indicators when content is scrollable above/below
   - Example: `shadow-[inset_0_10px_10px_-10px_rgba(0,0,0,0.1)]`

2. **Keyboard Scroll**
   - Space/PageUp/PageDown work in scrollable areas
   - Arrow key navigation in lists could be improved

---

## Recommendations

### 1. Add Scroll Shadows

```tsx
// Scroll container with shadows
<div
  className={cn(
    "overflow-auto",
    "scroll-shadows" // Custom class
  )}
  onScroll={handleScrollShadows}
>
```

### 2. Fix Mobile Legend

```tsx
// On mobile, render legend differently
{
  isMobile ? (
    <Sheet>
      <SheetTrigger>
        <Button variant="ghost" size="sm">
          <Info className="h-4 w-4" />
          Legend
        </Button>
      </SheetTrigger>
      <SheetContent side="bottom">
        <Legend />
      </SheetContent>
    </Sheet>
  ) : (
    <Legend />
  );
}
```

### 3. Scroll Position Persistence

Consider persisting scroll position when navigating away and returning to a page, especially for:

- Document detail page (long content)
- Entity browser (large lists)
- Conversation history

---

## Verification Checklist

- [x] Header remains fixed during page scroll
- [x] Sidebar remains fixed during content scroll
- [x] Query input area stays fixed at bottom
- [x] Document list scrolls independently
- [ ] **Entity browser panel has internal scroll** ❌ BROKEN - `overflow: hidden`
- [x] Graph canvas pan/zoom works correctly
- [ ] **Preview panels have internal scroll** ❌ BROKEN - `overflow: visible`
- [x] Conversation messages scroll with auto-bottom
- [x] Large document content (66K+ lines) scrolls smoothly
- [ ] **Details panel scrolls when content overflows** ❌ BROKEN - `overflow: hidden`
- [ ] Mobile legend overlay (NEEDS FIX)
- [ ] Scroll shadows for visual feedback (ENHANCEMENT)
- [ ] Conversation history keyboard navigation (NEEDS IMPROVEMENT)

---

## Score (Updated December 30, 2025 - Post Scroll Bug Discovery)

| Criterion                | Score (1-5) | Notes                                          |
| ------------------------ | ----------- | ---------------------------------------------- |
| Fixed Zone Consistency   | 4.5         | All major zones fixed correctly                |
| Internal Scroll Handling | **2.5**     | ❌ 3 critical panels have broken scroll        |
| Responsive Scroll        | 3.5         | Mobile legend issue + panel scroll bugs        |
| Performance              | 5.0         | Large document scrolls smoothly (when working) |
| Keyboard Navigation      | 3.0         | Missing in entity list and history panel       |
| **Overall**              | **3.7**     | ⚠️ Critical scroll bugs must be fixed          |

**Score Change:** 4.5 → 3.7 (-0.8) due to discovery of 3 critical scroll bugs

---

## Priority Fix Order

1. **P0**: Entity Browser scroll (Graph page) - 71% of content hidden
2. **P0**: Details Panel scroll (Graph page) - Action buttons hidden
3. **P0**: Preview Panel scroll (Documents page) - Action buttons hidden
4. **P0**: Mobile Legend overlay (Graph page) - Blocks interaction
5. **P1**: History Panel padding/keyboard (Query page) - UX improvement

---

_Created: December 30, 2025_
_Updated: December 30, 2025 (Critical scroll bugs discovered)_
