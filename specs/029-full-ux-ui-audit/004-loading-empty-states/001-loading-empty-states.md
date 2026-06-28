# 001 — Loading & Empty States Audit

**First Principle: Feedback** — Every action and wait state deserves a response.

---

## Inventory of States

| Page/Component | Loading State | Empty State | Quality |
|----------------|--------------|-------------|---------|
| Documents list | ✅ Skeleton rows | ✅ Filter-aware + CTA | Good |
| Dashboard stats | ✅ Skeleton cards | ⚠️ Shows "0" without CTA | Needs work |
| Query chat | ✅ Animated dots | ✅ Suggestions + icon | Good |
| Graph | ✅ Full overlay with phase text | ⚠️ No empty state for 0 entities | Needs work |
| Settings | ❌ No loading state | N/A | Missing |
| Knowledge page | ❓ Unknown | ❓ Unknown | Needs audit |
| Pipeline | ✅ Described in pipeline cards | ⚠️ No empty state | Partial |
| Login | ✅ Button spinner | N/A | Good |
| Document detail | ⚠️ Unknown | ⚠️ Unknown | Unknown |

---

## Loading State Analysis

### LS-01 · Document List Skeleton (Good Pattern)

```typescript
// document-table-states.tsx — LoadingSkeleton
function LoadingSkeleton({ rowCount = 5 }) {
  return (
    <div className="border rounded-lg overflow-hidden">
      {[...Array(rowCount)].map((_, i) => (
        <div key={i} className="flex items-center gap-4 px-4 py-3 ... animate-pulse">
          <Skeleton className="h-4 w-4" />     // checkbox
          <Skeleton className="h-4 w-48" />    // name
          <Skeleton className="h-5 w-20 rounded-full" /> // status badge
          ...
        </div>
      ))}
    </div>
  );
}
```

**Strengths:**
- Matches the actual table structure (content-aware skeleton)
- `animate-pulse` provides motion feedback
- Column widths approximate real content widths

**Issues:**
- `animate-pulse` opacity flicker can cause visual noise at low brightness — use `shimmer` gradient instead for premium feel
- No `aria-busy` attribute on the containing element
- No `aria-label` for screen readers during load

**Fix:**
```typescript
<div 
  role="status" 
  aria-busy="true" 
  aria-label="Loading documents..."
  className="border rounded-lg overflow-hidden"
>
```

### LS-02 · Graph Loading Overlay (Good Pattern)

The `GraphLoadingOverlay` provides a full-screen overlay with phase text ("Loading graph viewer..."). This is a good pattern for heavy visual components.

**Issues:**
- Phase text is hardcoded string rather than translated
- No progress indicator — users don't know if it's 5% or 95% complete
- The overlay background could be more refined (matches full dark overlay vs. subtle backdrop)

### LS-03 · Dashboard Stats: No Graceful Loading →  "0"

```typescript
// stats-card.tsx
if (isLoading) {
  return <Card>...<Skeleton /></Card>;  // ✅ skeleton
}
// ...
<span>{value}</span>  // Shows "0" immediately after load
```

When stats load as 0 (new workspace), the page shows `0 0 0 0` — these look like loading errors, not actual data. The stats need contextual awareness.

**Fix:**
```typescript
// Distinguish between:
// (a) loading → skeleton
// (b) loaded, value = 0, is new user → show onboarding prompt
// (c) loaded, value = 0, had data before → show "0 (empty)"

{value === 0 && !isLoading && isNewWorkspace ? (
  <div className="flex items-center gap-1.5 text-muted-foreground text-sm">
    <Upload className="h-3.5 w-3.5" />
    <span>Upload documents to see stats</span>
  </div>
) : (
  <span className="text-3xl font-bold">{value}</span>
)}
```

---

## Empty State Analysis

### ES-01 · Document Empty State (Good Pattern)

```typescript
// document-table-states.tsx — EmptyState
<div className="text-center py-16 text-muted-foreground border rounded-lg bg-muted/5">
  <FileText className="h-12 w-12 mx-auto mb-4 opacity-40" />
  <p className="font-medium text-lg text-foreground">No documents yet</p>
  <p className="text-sm mt-2 max-w-sm mx-auto">Upload documents to build your knowledge graph</p>
  <Button className="mt-6">Upload Documents</Button>
</div>
```

**Strengths:** Icon + title + description + CTA. Standard pattern.

**Issues:**
- `opacity-40` on a `FileText` icon with `text-muted-foreground` parent may result in very low contrast icon
- `py-16` (64px) may not center correctly when table section has flex layout
- Button missing variant specification — likely inherits `default` which is fine

### ES-02 · Filter Empty State (Good Pattern)

```typescript
function FilteredEmptyState({ onClearFilter }) {
  return (
    <div className="text-center py-16 ...">
      <Search className="h-12 w-12 ..." />
      <p>No matching documents</p>
      <Button onClick={onClearFilter}>Clear filter</Button>
    </div>
  );
}
```

This correctly distinguishes "no documents" from "filter hides results." The [Clear filter] button is appropriately action-oriented.

**Minor issue:** The `Search` icon in a filter-empty state is semantically correct but visually could be enhanced with a more emotional/illustrative SVG.

### ES-03 · Query Empty State (Good, Could Be Better)

```typescript
// query-empty-state.tsx
<div className="flex flex-col items-center justify-center h-full py-12 px-4">
  <div className="...bg-gradient-to-br from-primary/80 to-primary rounded-2xl p-5">
    <Sparkles className="h-10 w-10 text-primary-foreground" />
  </div>
  <h2>Ask about your knowledge graph</h2>
  <p>Explore entities, find connections...</p>
  {/* Suggestion chips */}
</div>
```

**Strengths:** Icon, heading, description, and suggestion chips create a welcoming, actionable empty state.

**Issues:**
- `h-full` with `flex flex-col items-center justify-center` — if the scroll area isn't taking full viewport height, this may not vertically center
- The gradient icon box (`from-primary/80 to-primary`) is heavy for a minimalist aesthetic — a simple outlined icon would be cleaner
- 4 suggestion chips is the right number (NNGroup recommends 3-5 suggestions)
- Suggestion chips should have `type="button"` to prevent form submission issues

### ES-04 · Graph Empty State (Missing)

When the graph has 0 nodes, the `GraphViewer` renders an empty canvas with no guidance. Users don't know what to do.

**Fix:** Add an empty state overlay:

```typescript
// graph-viewer.tsx — add empty state check
if (nodes.length === 0 && !isLoading) {
  return (
    <div className="flex flex-col items-center justify-center h-full gap-4">
      <Network className="h-12 w-12 text-muted-foreground opacity-40" />
      <h2 className="text-lg font-semibold">Knowledge graph is empty</h2>
      <p className="text-sm text-muted-foreground text-center max-w-sm">
        Upload and process documents to populate your knowledge graph
      </p>
      <Button asChild>
        <Link href="/documents">Upload Documents</Link>
      </Button>
    </div>
  );
}
```

---

## Best Practice Patterns

### The Empty State Formula

```
┌─────────────────────────────────────────────────────┐
│                                                     │
│           [Illustrative Icon / Illustration]        │  60-80px, muted
│                                                     │
│              Primary Empty State Title              │  16-18px, semibold
│                                                     │
│   Brief description of why it's empty and what     │  14px, muted-foreground
│   the user can do about it (max 2 lines)            │  max-width: 320px
│                                                     │
│               [ Primary CTA Button ]                │  mt-6, primary variant
│                                                     │
│      [Secondary action link if applicable]          │  mt-2, text button
│                                                     │
└─────────────────────────────────────────────────────┘
```

### The Loading State Formula

```
┌─────────────────────────────────────────────────────┐
│ [███████████] Title                                 │  Skeleton matching content
│ [████] subtitle text here...                        │
│                                                     │
│ aria-busy="true" aria-label="Loading {context}..."  │  Screen reader
└─────────────────────────────────────────────────────┘
```

### Shimmer vs Pulse

```
Pulse (current):  opacity: 1 → 0.5 → 1  (choppy, attention-grabbing)
Shimmer (better): gradient sweeps left to right  (smooth, premium feel)
```

```css
/* Add to globals.css */
@keyframes shimmer {
  0% { background-position: -200% center; }
  100% { background-position: 200% center; }
}

.skeleton-shimmer {
  background: linear-gradient(
    90deg,
    var(--muted) 25%,
    oklch(0.93 0 0) 50%,
    var(--muted) 75%
  );
  background-size: 200% 100%;
  animation: shimmer 1.5s ease-in-out infinite;
}
```

---

## External References

- [Empty States Design — NNGroup](https://www.nngroup.com/articles/empty-states/)
- [Skeleton Screens — Luke Wroblewski](https://www.lukew.com/ff/entry.asp?1797)
- [Progressive Loading — CSS Tricks](https://css-tricks.com/building-skeleton-screens-css-custom-properties/)
- [Perceived Performance — NNGroup](https://www.nngroup.com/articles/response-times-3-important-limits/)
