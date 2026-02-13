# Decision - Iteration 51

## Changes to Make

1. **graph-viewer.tsx:754** — Override Radix table wrapper on right panel ScrollArea
   - Add `[&_[data-slot=scroll-area-viewport]>div]:!block` to ScrollArea className
   - This forces `display: block !important` on the anonymous div

2. **graph-viewer.tsx:755** — Add `overflow-hidden` to content div inside ScrollArea
   - Change `<div className="px-4 py-4 space-y-5">` → `<div className="px-4 py-4 space-y-5 overflow-hidden">`

3. **node-details.tsx:95** — Fix PropertyValue outer flex div
   - Add `min-w-0` to allow flex shrinking
   - Reduce `gap-3` → `gap-2` to save 4px per row

4. **node-details.tsx:96** — Fix PropertyValue label
   - Remove `min-w-[70px]` (was forcing 70px minimum on labels)

5. **node-details.tsx:104** — Fix PropertyValue value span
   - Add `min-w-0` to truncate class when not expanded

6. **node-details.tsx:228** — Fix description paragraph
   - Add `break-words` to prevent long words from overflowing

## Priority

1. **P0** — Radix wrapper override (eliminates root cause)
2. **P0** — PropertyValue min-w-0 + gap reduction (fixes content-level overflow)
3. **P1** — Content div overflow-hidden (safety net)
4. **P1** — Description break-words (prevents edge case)

## Expected Outcome

- Right panel scrollWidth === clientWidth (zero horizontal overflow)
- All property values, buttons, and controls fully visible
- Content properly constrained within panel boundaries
- No visual clipping of Edit/Merge/Delete buttons
