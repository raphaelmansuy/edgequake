# 001 — Performance UX Audit

**First Principle: Feedback** — The app should feel fast even when it isn't.

---

## Core Web Vitals Impact

### CWV-01 · LCP (Largest Contentful Paint)

The dashboard page loads:
1. Next.js shell (fast)
2. Auth check (`useAuthStore`)
3. Tenant hydration (`useTenantStore`)
4. Stats queries (parallel React Query fetch)
5. Stats cards render

The LCP element is likely the stats card area or the page heading. The issue is that the stats queries hit the backend before rendering, so users see a skeleton for 200-500ms+ on every page load.

**Optimization:** Stats cards should use `staleTime` + `gcTime` to serve cached data immediately:

```typescript
// Already done if using React Query defaults, but verify:
useQuery({
  queryKey: ['workspace-stats', workspaceId],
  staleTime: 30_000,      // Use cache for 30s before revalidating
  gcTime: 5 * 60_000,     // Keep in cache for 5 minutes
  // This allows instant render from cache + background update
});
```

### CWV-02 · CLS (Cumulative Layout Shift)

Sources of layout shift identified:

1. **Backend status banner** — appears after load, shifts all content down
2. **Upload progress rows** — appear inline in documents table, shift rows
3. **Breadcrumb** — renders after client-side navigation, may shift on first render
4. **Fonts** — Geist font loaded via `next/font` (should be zero CLS if configured correctly)

**Fix for banner CLS:** Reserve space or use fixed positioning:

```typescript
// backend-status-banner.tsx
// Render as overlay, not inline block
<div className="fixed top-[var(--header-height)] left-0 right-0 z-sticky">
```

### CWV-03 · INP (Interaction to Next Paint)

Heavy interactions identified:
- Graph rendering (Sigma.js) on `/graph` page
- Document table re-render when WebSocket updates arrive
- Large chat messages with complex Markdown rendering

**GraphViewer** already uses `dynamic import` with SSR disabled — this is correct. The `GraphLoadingFallback` prevents blank screens during chunk loading.

**Document table:** The `memo()` wrapping on `DocumentTableSection` is correct. Verify that WebSocket updates don't trigger unnecessary re-renders of all rows.

---

## Perceived Performance

### PP-01 · Optimistic Updates Missing

When a user clicks "Delete document," the current flow is:
1. User clicks Delete
2. Confirmation dialog
3. Confirm
4. API call (200-500ms)
5. Table updates

Users wait for the API response before seeing feedback. **Optimistic updates** would immediately remove the row (visually), then revert if the API call fails.

```typescript
// useDocumentMutations hook
deleteMutation: useMutation({
  mutationFn: (id) => deleteDocument(id),
  // ADD: Optimistic update
  onMutate: async (id) => {
    await queryClient.cancelQueries({ queryKey: ['documents'] });
    const previous = queryClient.getQueryData(['documents']);
    queryClient.setQueryData(['documents'], (old) => 
      old?.filter(doc => doc.id !== id)
    );
    return { previous };
  },
  onError: (err, id, ctx) => {
    queryClient.setQueryData(['documents'], ctx?.previous);
    toast.error('Failed to delete document');
  },
  onSettled: () => {
    queryClient.invalidateQueries({ queryKey: ['documents'] });
  },
});
```

### PP-02 · Backend Health Check Frequency

```typescript
// header.tsx
const interval = setInterval(checkConnection, 30000); // Check every 30s
```

Additionally, `BackendStatusBanner` polls every 10s. That's **two polling loops** running simultaneously checking backend health. Consolidate into one shared query:

```typescript
// Use React Query's shared cache — both components use the same key
useQuery({
  queryKey: ['backend-health'],
  queryFn: checkHealth,
  refetchInterval: 30_000,
  staleTime: 10_000,
});
```

Both the header indicator and the banner should read from this single cache entry.

### PP-03 · Graph Page: Initial Load Experience

```typescript
// graph/page.tsx — dynamic import with loading fallback
const GraphViewer = dynamic(
  () => import('@/components/graph/graph-viewer'),
  { ssr: false, loading: GraphLoadingFallback }
);
```

This is correct. However, the `GraphLoadingOverlay` text is:
```typescript
<GraphLoadingOverlay visible={true} phase="Loading graph viewer..." />
```

The loading phase text is hardcoded and doesn't update as loading progresses. Users see "Loading graph viewer..." for the entire 2-5 second load.

**Better:** Use phase-aware text:
```typescript
const loadingPhases = [
  { ms: 0,    text: "Loading graph viewer..." },
  { ms: 1000, text: "Fetching graph data..." },
  { ms: 3000, text: "Rendering nodes..." },
  { ms: 5000, text: "Almost ready..." },
];
```

### PP-04 · Pagination vs. Virtual Scrolling

The documents table uses client-side pagination (10/20/50/100 rows). For workspaces with thousands of documents, even the 100-row option may be slow to render.

Consider implementing **virtualized rows** using `@tanstack/react-virtual` for tables with >50 rows:

```typescript
// For large document lists:
import { useVirtualizer } from '@tanstack/react-virtual';

const rowVirtualizer = useVirtualizer({
  count: documents.length,
  getScrollElement: () => parentRef.current,
  estimateSize: () => 56, // Approximate row height
  overscan: 5,
});
```

However, pagination at 20 items is already quite good for most use cases. Virtual scroll only matters if users regularly work with 500+ documents.

---

## Bundle Performance

### BP-01 · Dynamic Imports Coverage

| Component                 | Dynamic Import | Justification               |
| ------------------------- | -------------- | --------------------------- |
| GraphViewer (Sigma.js)    | ✅ Yes          | Heavy canvas library        |
| GraphTourWrapper          | ✅ Yes          | Client-only                 |
| ChatMessage markdown      | ❓ Unknown      | Should be dynamic if >100KB |
| CodeBlock (highlight.js?) | ❓ Unknown      | Should be dynamic           |
| ConversationHistoryPanel  | ❓ Unknown      | Non-critical path           |

Heavy Markdown rendering libraries (rehype-highlight, katex) should be dynamically imported if they aren't already.

---

## Positive Performance Patterns Found

```
✅ Dynamic import for GraphViewer (avoids SSR + reduces initial bundle)
✅ React.memo on DocumentTableSection, DocumentTableRow, ChatMessage
✅ React.memo on QueryEmptyState
✅ Suspense boundaries for URL sync and workspace hooks
✅ staleTime in React Query for background data freshness
✅ getAutomationAwareRefetchInterval (prevents Playwright interference)
✅ requestAnimationFrame for theme switching
✅ WebSocket for real-time document status (replaces polling)
```

---

## Lighthouse Target Scores

```
Performance:    ≥ 90
Accessibility:  ≥ 95  (currently likely 70-80 based on issues found)
Best Practices: ≥ 95
SEO:            ≥ 80 (limited for authenticated app)
```

---

## External References

- [Web Vitals — web.dev](https://web.dev/vitals/)
- [CLS Best Practices — web.dev](https://web.dev/cls/)
- [Optimistic UI — UX Collective](https://uxdesign.cc/optimistic-ui-for-better-ux)
- [React Query Performance Patterns](https://tkdodo.eu/blog/react-query-performance)
- [TanStack Virtual](https://tanstack.com/virtual/latest)
- [Perceived Performance — NNGroup](https://www.nngroup.com/articles/response-times-3-important-limits/)
