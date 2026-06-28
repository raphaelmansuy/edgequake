# Plan: Knowledge Graph Improvements

**Addresses:** F-GR-01, F-GR-02, F-GR-04, F-GR-06  
**Files to change:** `src/lib/graph/`, `src/components/graph/graph-viewer.tsx`, `src/components/graph/entity-browser-panel.tsx`

---

## Changes

### 1. `formatEntityLabel()` utility

Add to `src/lib/graph/label-utils.ts`:

```ts
/**
 * Convert normalized entity names to human-readable labels.
 * MARKET_SURVEILLANCE_AUTH → "Market Surveillance Auth"
 * AB_CARVAL_AVIATION_LEASING_FU → "Ab Carval Aviation Leasing Fu"
 */
export function formatEntityLabel(raw: string, maxLen = 30): string {
  const formatted = raw
    .replace(/_/g, ' ')
    .replace(/\b\w/g, c => c.toUpperCase());
  if (formatted.length <= maxLen) return formatted;
  return formatted.slice(0, maxLen - 1) + '…';
}
```

Apply in:
- Graph renderer node label rendering
- Entity browser panel list items
- Node details panel title

### 2. Graph toolbar grouping

Add visual separators between toolbar action groups:

```
[zoom−][fit][zoom+]  |  [export][share]  |  [settings][filter]
```

Use `<Separator orientation="vertical" />` between groups.

### 3. Right panel empty state improvement

```tsx
// Before: "Click on a node to view details"
// After:
<div className="text-center text-sm text-muted-foreground p-4">
  <Network className="mx-auto h-8 w-8 mb-2 opacity-30" />
  <p>Select a node to explore its connections, relationships, and source documents.</p>
</div>
```

### 4. Entity type labels in left panel

Convert entity type group headers from ALL_CAPS to Title Case:
```
TECHNOLOGY → Technology
ORGANIZATION → Organization
CONCEPT → Concept
```

---

## Acceptance Criteria

- [ ] Node labels show "Market Surveillance Auth" not "MARKET_SURVEILLANCE_AUTH"
- [ ] Entity browser panel shows formatted names
- [ ] Toolbar has visible separators between action groups
- [ ] Right panel empty state is descriptive
- [ ] Entity type group labels are in Title Case
