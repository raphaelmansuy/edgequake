# Observation - Iteration 41

## Focus: Detail Page Right Panel Scrollability Audit

## Files Examined

- `edgequake_webui/src/components/document/metadata-sidebar.tsx` (115 lines) — Right panel content with fixed header + sections
- `edgequake_webui/src/app/(dashboard)/documents/[id]/page.tsx` (370 lines) — Document detail page layout
- `edgequake_webui/src/components/ui/resizable-panel.tsx` (222 lines) — Reusable resizable panel wrapper
- `edgequake_webui/src/components/ui/scroll-area.tsx` (138 lines) — Radix UI ScrollArea with `showShadows` prop
- `edgequake_webui/src/app/(dashboard)/layout.tsx` — Dashboard layout with `flex h-screen overflow-hidden`

## Browser Evaluation (Pre-Fix)

CSS evaluation at `http://localhost:3000/documents/{id}`:

| Property | Container | ScrollArea Viewport |
|----------|-----------|---------------------|
| scrollHeight | 1281px | 1060px |
| clientHeight | 851px | 1060px |
| overflow | visible | — |
| scrollable? | — | NO (scrollHeight === clientHeight) |

## Root Cause Identified

Classic CSS flexbox `min-height: auto` bug:

1. Dashboard layout: `flex h-screen overflow-hidden` → main `flex-1 min-h-0 overflow-hidden`
2. Detail page: `flex-1 flex overflow-hidden` wraps content + `ResizablePanel`
3. ResizablePanel renders children inside `div.flex-1.overflow-hidden`
4. MetadataSidebar: `div.h-full.flex.flex-col` → `ScrollArea.flex-1`

**Bug**: In step 4, the `ScrollArea` with `className="flex-1"` in a `flex-col` container doesn't shrink below its content's intrinsic minimum height (`min-height: auto` is the CSS default). The ScrollArea viewport expands to exactly match content height (1060px), making scrolling impossible.

**Solution**: Add `min-h-0` to the ScrollArea to override the `min-height: auto` default, allowing it to shrink to the available space.

## Current State

- The MetadataSidebar header used `sticky top-0 z-10` — but sticky positioning doesn't work correctly in a non-scrolling flex child context
- No `overflow-hidden` on the root container to establish a proper overflow boundary
- `showShadows` prop (visual scroll indicators) not used despite being available
