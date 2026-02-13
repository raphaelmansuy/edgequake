# Observation - Iteration 51

## Focus: Graph Panel Right Panel Horizontal Content Overflow

## Files Examined

- `edgequake_webui/src/components/graph/graph-viewer.tsx` (lines 729-810) — Right panel structure with ResizablePanel + ScrollArea + NodeDetails
- `edgequake_webui/src/components/graph/node-details.tsx` (lines 60-130) — PropertyValue component with truncate behavior
- `edgequake_webui/src/components/ui/scroll-area.tsx` (lines 1-120) — Radix ScrollArea wrapper with shadow indicators
- `edgequake_webui/src/components/ui/resizable-panel.tsx` (lines 1-222) — ResizablePanel with drag handle

## DOM Analysis (Playwright evaluate)

Walked 10 levels from H3 "Details & Filters" to body:

```
Level 0: H3 — 118×16px, overflow:visible
Level 1: Header div — 279×45px, flex items-center justify-between px-4 py-2.5
Level 2: Panel content — 279×819px, flex flex-col h-full overflow-hidden
Level 3: ResizablePanel inner — 279×819px, flex-1 overflow-hidden
Level 4: ResizablePanel outer — 280×819px, relative flex shrink-0
Level 5: Main layout — 1184×819px, flex h-full overflow-hidden
Level 6: Content wrapper — 1184×819px
Level 7: MAIN — 1184×819px, flex-1 min-h-0 overflow-hidden
Level 8: Flex column — 1184×900px, flex flex-1 flex-col overflow-hidden
Level 9: Root — 1440×900px, flex h-screen overflow-hidden
```

## ScrollArea Viewport Analysis

**Right panel viewport (index 1)**:
- clientWidth: 279px
- scrollWidth: 332px  ← **53px HORIZONTAL OVERFLOW**
- scrollHeight: 774px = clientHeight (no vertical overflow)

**Root cause identified**: Radix ScrollArea injects a `<div style="display: table; min-width: 100%">` wrapper around content. This `display: table` element shrink-wraps to its content's intrinsic width (328px > viewport 279px).

## PropertyValue Component Layout

```
flex justify-between gap-3 py-1
├── label: shrink-0 min-w-[70px] text-[11px]  → forces 70px minimum
└── value div: flex items-center gap-1 min-w-0 flex-1 justify-end
    ├── span: truncate font-mono text-[10px]   → 20 chars ≈ 138px
    ├── expand button: shrink-0 h-4 w-4        → 16px fixed
    └── copy button: shrink-0 h-5 w-5          → 20px fixed
```

Minimum width calculation: 70 + 12 + 138 + 16 + 20 + 8 = 264px
Available width: 279 - 32 (px-4) - 16 (p-2 wrapper) = 231px
**Overflow: 264 - 231 = 33px**

## Current State

- Content overflows horizontally by 53px
- No horizontal ScrollBar rendered (only vertical)
- Property values visually truncated/clipped at right edge
- Edit/Merge/Delete action buttons partially cut off
- Panel attached to right border (verified)
