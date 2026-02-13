# Analysis - Iteration 41

## Gaps Identified

1. **ScrollArea not shrinkable** — `flex-1` without `min-h-0` means the flex item can't shrink below content height
2. **Header sticky without scroll parent** — `sticky top-0` has no effect when the parent is a non-scrolling flex container
3. **No overflow boundary** — Root container `div.h-full.flex.flex-col` lacks `overflow-hidden`, so content spills without constraint
4. **No visual scroll hint** — Users can't tell content is scrollable without `showShadows`

## Possible Solutions

### Solution A: Add `min-h-0` + `overflow-hidden` (CSS-only fix)

- Pros: Minimal change, addresses root cause, no component refactoring
- Cons: None — this is the canonical CSS flexbox fix
- Risk: Low

### Solution B: Replace ScrollArea with native `overflow-y-auto`

- Pros: Simpler, fewer dependencies
- Cons: Loses Radix UI's custom scrollbar styling and `showShadows` feature
- Risk: Medium (visual regression)

### Solution C: Restructure layout to avoid flex nesting

- Pros: Eliminates root cause entirely
- Cons: Major refactor, risk of breaking other layouts
- Risk: High

## Recommendation

**Solution A** — The canonical CSS flexbox `min-h-0` pattern. This is well-documented (CSS spec: flex items have `min-height: auto` by default in column direction). Adding `overflow-hidden` to the root container establishes a proper boundary, and `shrink-0` on the header prevents it from collapsing.

## First Principles Analysis

The CSS flexbox specification (Section 4.5) states that flex items have `min-width: auto` and `min-height: auto` by default. In a `flex-direction: column` container, a child with `flex: 1` will grow to fill available space but **will not shrink below its content's intrinsic minimum height**. The fix is to explicitly set `min-height: 0` (Tailwind: `min-h-0`).
