# Scroll Behavior Analysis

## Updated: 2025-12-26

This document analyzes the scroll behavior of each page and defines what should be fixed vs scrollable.

---

## Global Layout Structure

```
┌─────────────────────────────────────────────────────────────────┐
│ App Shell (h-screen, overflow-hidden)                          │
│ ┌──────────┬───────────────────────────────────────────────────┤
│ │ Sidebar  │ Content Area (flex-1, overflow-hidden)            │
│ │ (fixed)  │ ┌─────────────────────────────────────────────────┤
│ │          │ │ Header (h-12, shrink-0)              [FIXED]   │
│ │          │ ├─────────────────────────────────────────────────┤
│ │          │ │ Breadcrumb (py-2, shrink-0)          [FIXED]   │
│ │          │ ├─────────────────────────────────────────────────┤
│ │          │ │ Main Content (flex-1, overflow-auto) [SCROLL]  │
│ │          │ │                                                 │
│ └──────────┴─┴─────────────────────────────────────────────────┘
└─────────────────────────────────────────────────────────────────┘
```

**Source**: [layout.tsx](<../edgequake_webui/src/app/(dashboard)/layout.tsx>)

---

## Documents Page - Scroll Analysis

### Current State

```
┌─────────────────────────────────────────────────────────────────┐
│ Main Content Area (flex-1, overflow-auto)                      │
│ ┌─────────────────────────────────────────────────┬────────────┤
│ │ Container (p-6, space-y-6)                      │ RightPanel │
│ │ ┌─────────────────────────────────────────────┐ │            │
│ │ │ Page Header (flex) - h1 + buttons [SCROLLS]│ │            │
│ │ └─────────────────────────────────────────────┘ │            │
│ │ ┌─────────────────────────────────────────────┐ │            │
│ │ │ Filters (Document Filters)         [SCROLLS]│ │            │
│ │ └─────────────────────────────────────────────┘ │            │
│ │ ┌─────────────────────────────────────────────┐ │            │
│ │ │ Upload Dropzone                    [SCROLLS]│ │            │
│ │ └─────────────────────────────────────────────┘ │            │
│ │ ┌─────────────────────────────────────────────┐ │            │
│ │ │ Documents Table                    [SCROLLS]│ │ ScrollArea │
│ │ │ (grows with content)                        │ │            │
│ │ └─────────────────────────────────────────────┘ │            │
│ │ ┌─────────────────────────────────────────────┐ │            │
│ │ │ Pagination Controls                [SCROLLS]│ │            │
│ │ └─────────────────────────────────────────────┘ │            │
│ └─────────────────────────────────────────────────┴────────────┘
└─────────────────────────────────────────────────────────────────┘
```

### Desired State (Issue #4)

**Goal**: Header + Search + Drag Area should be fixed; Document list should scroll independently.

```
┌─────────────────────────────────────────────────────────────────┐
│ Main Content Area (flex, h-full, overflow-hidden)               │
│ ┌─────────────────────────────────────────────────┬────────────┤
│ │ Content Container (flex-1, flex-col, h-full)    │ RightPanel │
│ │ ┌─────────────────────────────────────────────┐ │            │
│ │ │ Page Header (shrink-0)              [FIXED] │ │            │
│ │ └─────────────────────────────────────────────┘ │            │
│ │ ┌─────────────────────────────────────────────┐ │            │
│ │ │ Search/Filters Bar (shrink-0)       [FIXED] │ │            │
│ │ └─────────────────────────────────────────────┘ │            │
│ │ ┌─────────────────────────────────────────────┐ │            │
│ │ │ Upload Dropzone (shrink-0)          [FIXED] │ │            │
│ │ └─────────────────────────────────────────────┘ │            │
│ │ ┌─────────────────────────────────────────────┐ │            │
│ │ │ Documents Table (flex-1)           [SCROLL] │ │ ScrollArea │
│ │ │ (ScrollArea component)                      │ │            │
│ │ └─────────────────────────────────────────────┘ │            │
│ │ ┌─────────────────────────────────────────────┐ │            │
│ │ │ Pagination Controls (shrink-0)      [FIXED] │ │            │
│ │ └─────────────────────────────────────────────┘ │            │
│ └─────────────────────────────────────────────────┴────────────┘
└─────────────────────────────────────────────────────────────────┘
```

**Source**: [document-manager.tsx](../edgequake_webui/src/components/documents/document-manager.tsx)

---

## Query Page - Scroll Analysis

### Current State

```
┌─────────────────────────────────────────────────────────────────┐
│ Main Content (flex, h-full, min-h-0)                            │
│ ┌─────────────────────────────────────────────────┬────────────┤
│ │ Query Area (flex-1, flex-col, overflow-hidden)  │ Conversation│
│ │ ┌─────────────────────────────────────────────┐ │ History    │
│ │ │ Header (shrink-0)                   [FIXED] │ │ Panel      │
│ │ └─────────────────────────────────────────────┘ │            │
│ │ ┌─────────────────────────────────────────────┐ │ [SCROLL]   │
│ │ │ Chat Area (flex-1)                 [SCROLL] │ │            │
│ │ │ (ScrollArea component)                      │ │            │
│ │ └─────────────────────────────────────────────┘ │            │
│ │ ┌─────────────────────────────────────────────┐ │            │
│ │ │ Input Area (shrink-0)               [FIXED] │ │            │
│ │ └─────────────────────────────────────────────┘ │            │
│ └─────────────────────────────────────────────────┴────────────┘
└─────────────────────────────────────────────────────────────────┘
```

### Status: ✅ Correct

**Source**: [query-interface.tsx](../edgequake_webui/src/components/query/query-interface.tsx)

---

## Graph Page - Scroll Analysis

### Current State

```
┌─────────────────────────────────────────────────────────────────┐
│ Main Content (h-full, overflow-hidden)                          │
│ ┌──────────────┬─────────────────────────────────┬─────────────┤
│ │ Entity       │ Graph Area (flex-1)             │ Details     │
│ │ Browser      │ ┌─────────────────────────────┐ │ Panel       │
│ │ [SCROLL]     │ │ Toolbar (shrink-0)  [FIXED] │ │ [SCROLL]    │
│ │              │ └─────────────────────────────┘ │             │
│ │ ScrollArea   │ ┌─────────────────────────────┐ │ ScrollArea  │
│ │              │ │ Canvas (flex-1)     [FIXED] │ │             │
│ │              │ │ (Sigma.js)                  │ │             │
│ │              │ └─────────────────────────────┘ │             │
│ │              │ Controls overlay (absolute)    │             │
│ └──────────────┴─────────────────────────────────┴─────────────┘
└─────────────────────────────────────────────────────────────────┘
```

### Status: ✅ Mostly correct - needs dark mode fix for fullscreen

**Source**: [graph-viewer.tsx](../edgequake_webui/src/components/graph/graph-viewer.tsx)

---

## Settings Panel (Sheet) - Scroll Analysis

### Current State (Query Settings Sheet)

```
┌─────────────────────────────────────────────────────────────────┐
│ Sheet Content (max-w-md)                                        │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ Sheet Header                                        [FIXED] │ │
│ └─────────────────────────────────────────────────────────────┘ │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ Sheet Body                                         [SCROLL] │ │
│ │ - Query Mode Section                                        │ │
│ │ - Parameters Section (sliders)                              │ │
│ │ - Advanced Settings                                         │ │
│ └─────────────────────────────────────────────────────────────┘ │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ Sheet Footer (if any)                               [FIXED] │ │
│ └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### Improvements Needed (Issue #5)

- Analyze padding and margins for consistency
- Ensure smooth scrolling on content
- Check slider interaction spacing

**Source**: [query-interface.tsx](../edgequake_webui/src/components/query/query-interface.tsx) (Sheet component)

---

## Key CSS Classes for Scroll Control

| Class             | Purpose                       | Use Case                          |
| ----------------- | ----------------------------- | --------------------------------- |
| `h-full`          | Full height of parent         | Container that fills space        |
| `flex-1`          | Grow to fill available space  | Main content areas                |
| `shrink-0`        | Don't shrink                  | Fixed headers/footers             |
| `overflow-hidden` | Hide overflow                 | Outer container to contain scroll |
| `overflow-auto`   | Auto scrollbars               | Scrollable content                |
| `min-h-0`         | Allow shrinking below content | Required for nested flex scroll   |
| `ScrollArea`      | Custom scroll component       | Consistent scroll styling         |

---

## Implementation Checklist

- [x] Documents Page: Restructure to flex-col with fixed header/dropzone
- [x] Documents Page: Wrap table in ScrollArea
- [x] Query Settings Sheet: Audit padding consistency - polished with ScrollArea
- [x] Graph Page: Fix dark mode in fullscreen - added dark class sync
- [x] Graph Page: Add refresh on navigation hook - refetchOnMount='always'
- [x] Query Page: Verify scroll structure - ✅ correct
- [x] Settings Page: Verify scroll structure - ✅ uses main content scroll
- [x] Dashboard Page: Verify scroll structure - ✅ uses main content scroll
