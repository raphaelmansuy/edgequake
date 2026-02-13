# Analysis - Iteration 54

## Should We Fix the Entity Browser Too?

### Pros of Applying Same Fix
- Consistent behavior across all ScrollAreas in graph page
- Reduces unnecessary layout computation (browser doesn't compute 512px table)
- Defensive against future CSS changes that might remove overflow-hidden

### Cons
- Entity browser has no visible bug
- Adding the selector adds complexity to the className
- Risk of breaking entity browser group accordion layout

### Decision: Apply Defensively

Apply the same `[&_[data-slot=scroll-area-viewport]>div]:!block` override to the entity browser ScrollArea, but verify it doesn't break the accordion/group layout.

### Layout Chain Comparison

| Aspect | Right Panel | Left Panel |
|--------|-------------|------------|
| Has overflow-hidden parent? | No (was missing) | Yes |
| User-visible overflow? | Yes (buttons cut off) | No |
| Radix wrapper width | 328px | 512px |
| Viewport width | 279px | 255px |
| Excess | 49px | 257px |
| Fix priority | P0 (critical) | P2 (defensive) |
