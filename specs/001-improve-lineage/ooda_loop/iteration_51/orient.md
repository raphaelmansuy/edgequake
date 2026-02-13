# Analysis - Iteration 51

## Root Cause

The horizontal content overflow in the graph right panel has **two independent causes**:

### Cause 1: Radix ScrollArea `display: table` Wrapper

Radix UI's ScrollArea Viewport injects an anonymous `<div style="display: table; min-width: 100%">` around children. Tables shrink-wrap to content intrinsic width. Since content has minimum intrinsic width > viewport width, the wrapper expands to 328px while viewport is 279px.

### Cause 2: PropertyValue Flex Layout Minimum Width

The PropertyValue component uses `min-w-[70px]` on labels and `shrink-0` on buttons, creating a minimum intrinsic width of ~264px. Combined with px-4 outer padding (32px) and p-2 wrapper padding (16px), the total exceeds available space.

## Possible Solutions

### Solution A: Override Radix Table Wrapper (CSS)
- Override `display: table` → `display: block` on the Radix wrapper
- Use Tailwind arbitrary selector: `[&_[data-slot=scroll-area-viewport]>div]:!block`
- **Pros**: Fixes root cause at the layout level, simple one-line fix
- **Cons**: Uses `!important` to override inline styles
- **Risk**: Low — only affects the specific ScrollArea, not global

### Solution B: Constrain Content Width
- Add explicit width/max-width on content div
- Use `width: 100%; overflow: hidden` 
- **Pros**: Doesn't modify Radix behavior
- **Cons**: `width: 100%` resolves against table wrapper (not viewport), so it's circular
- **Risk**: Medium — may not work due to table layout resolution order

### Solution C: Fix PropertyValue Component
- Remove `min-w-[70px]` from labels
- Add `min-w-0` to outer flex div and value span
- Reduce `gap-3` to `gap-2`
- **Pros**: Fixes content-level overflow
- **Cons**: Alone won't fix `display: table` expansion
- **Risk**: Low — improves flex shrinking behavior

## Recommendation

**Combine Solution A + Solution C**: Override the Radix wrapper AND fix the PropertyValue layout. This provides defense in depth — even if one fix is insufficient alone, the combination eliminates horizontal overflow completely.
