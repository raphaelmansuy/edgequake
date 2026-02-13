# Observation - Iteration 45

## Focus: Cross-Page Scrollable Panel Pattern Consistency

## Pattern Audit

Compared the scrollable panel pattern across all pages with right panels:

| Page                              | Container overflow-hidden | Header shrink-0    | ScrollArea min-h-0 | showShadows        | Status |
| --------------------------------- | ------------------------- | ------------------ | ------------------ | ------------------ | ------ |
| Document Detail (MetadataSidebar) | ✅ (fixed iter 41)        | ✅ (fixed iter 41) | ✅ (fixed iter 41) | ✅ (fixed iter 41) | Fixed  |
| Graph (graph-viewer)              | ✅ (pre-existing)         | ✅ (pre-existing)  | ✅ (pre-existing)  | ✅ (pre-existing)  | OK     |

## Canonical Pattern

```tsx
// WHY: CSS flexbox min-height:auto default prevents flex items from shrinking
// below their content's intrinsic height. This pattern overrides that default
// to create a properly constrained scrollable area within a flex column.
<div className="h-full flex flex-col overflow-hidden">
  <header className="shrink-0">Fixed header</header>
  <ScrollArea className="flex-1 min-h-0" showShadows>
    Scrollable content
  </ScrollArea>
</div>
```

## Current State

Both pages with right panels now use the consistent scrollable panel pattern. No other pages currently have right panels that need this fix.
