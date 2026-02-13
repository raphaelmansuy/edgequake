# Decision - Iteration 41

## Changes to Make

1. **`metadata-sidebar.tsx` root div**: Add `overflow-hidden` to establish overflow boundary
   - `className="h-full flex flex-col border-l bg-background"` → `className="h-full flex flex-col border-l bg-background overflow-hidden"`

2. **`metadata-sidebar.tsx` header**: Change `sticky top-0` to `shrink-0`
   - `sticky` has no effect in a non-scrolling flex child; `shrink-0` prevents header from collapsing

3. **`metadata-sidebar.tsx` ScrollArea**: Add `min-h-0` and `showShadows`
   - `className="flex-1"` → `className="flex-1 min-h-0"` + `showShadows` prop

## Priority

1. **High impact, low effort**: Add `min-h-0` to ScrollArea (the core fix)
2. **High impact, low effort**: Add `overflow-hidden` to root (boundary establishment)
3. **Medium impact, low effort**: Change `sticky` → `shrink-0` (correct semantics)
4. **Low impact, low effort**: Add `showShadows` (visual scroll indicator)

## Expected Outcome

- ScrollArea viewport height constrained to available space (~630px)
- ScrollArea becomes scrollable (scrollHeight 1060px > clientHeight 630px)
- All metadata sections visible by scrolling: Document Info, Source Details, Processing Info, Extended Metadata
- Header stays fixed at top without `sticky` hack
- Shadow gradients appear at top/bottom when content is scrollable
