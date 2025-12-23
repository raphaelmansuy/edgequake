# Performance Optimization Strategy

> **Document Version:** 1.0  
> **Date:** 2024-12-23  
> **Purpose:** Frontend and backend performance optimization approaches

---

## Table of Contents

1. [Performance Goals](#performance-goals)
2. [Current Performance Baseline](#current-performance-baseline)
3. [Frontend Optimizations](#frontend-optimizations)
4. [Data Loading Strategies](#data-loading-strategies)
5. [Rendering Optimizations](#rendering-optimizations)
6. [Bundle Size Optimization](#bundle-size-optimization)
7. [Monitoring & Metrics](#monitoring--metrics)

---

## Performance Goals

### Core Web Vitals Targets

| Metric   | Target  | Description              |
| -------- | ------- | ------------------------ |
| **LCP**  | < 2.5s  | Largest Contentful Paint |
| **FID**  | < 100ms | First Input Delay        |
| **CLS**  | < 0.1   | Cumulative Layout Shift  |
| **TTFB** | < 600ms | Time to First Byte       |
| **TTI**  | < 3.8s  | Time to Interactive      |

### Application-Specific Targets

| Operation                        | Target  | Current (Est.) |
| -------------------------------- | ------- | -------------- |
| Initial page load                | < 2s    | ~3s            |
| Graph render (1000 nodes)        | < 1s    | ~2s            |
| Document table render (100 rows) | < 200ms | ~500ms         |
| Query response display           | < 100ms | ~200ms         |
| Theme switch                     | < 50ms  | ~150ms         |

---

## Current Performance Baseline

### Bundle Analysis

**Estimated Current Bundle Sizes:**

| Bundle      | Estimated Size | Notes                |
| ----------- | -------------- | -------------------- |
| Main bundle | ~300KB gzip    | Next.js + React + UI |
| Sigma.js    | ~150KB gzip    | Graph visualization  |
| Radix UI    | ~50KB gzip     | UI primitives        |
| Other deps  | ~100KB gzip    | Various utilities    |
| **Total**   | **~600KB**     | Initial load         |

### Identified Bottlenecks

1. **Graph Loading:** Full graph loaded on mount
2. **No Code Splitting:** Heavy components not lazy loaded
3. **No Image Optimization:** No Next.js Image optimization used
4. **No Memoization:** Components re-render unnecessarily
5. **No Virtualization:** Long lists rendered fully

---

## Frontend Optimizations

### 1. Code Splitting & Lazy Loading

#### Heavy Components to Lazy Load

```tsx
// Instead of direct imports
import { GraphViewer } from "@/components/graph/graph-viewer";

// Use dynamic imports
import dynamic from "next/dynamic";

const GraphViewer = dynamic(
  () =>
    import("@/components/graph/graph-viewer").then((mod) => mod.GraphViewer),
  {
    loading: () => <GraphSkeleton />,
    ssr: false, // Sigma.js doesn't support SSR
  }
);
```

**Components to Lazy Load:**

| Component            | Reason                 | Impact         |
| -------------------- | ---------------------- | -------------- |
| GraphViewer          | Sigma.js is heavy      | -150KB initial |
| MermaidDiagram       | Mermaid.js is heavy    | -100KB initial |
| QuerySettings        | Not immediately needed | -20KB initial  |
| PipelineStatusDialog | Used occasionally      | -10KB initial  |
| PropertyEditDialog   | Used occasionally      | -5KB initial   |

---

#### Route-Based Code Splitting

Next.js App Router automatically code-splits by route. Ensure:

```
app/
├── (dashboard)/
│   ├── documents/    # Separate chunk
│   ├── graph/        # Separate chunk
│   └── query/        # Separate chunk
```

---

### 2. Component Memoization

#### React.memo for List Items

```tsx
// Document row - renders many times
const DocumentRow = memo(function DocumentRow({ doc }: { doc: Document }) {
  return (
    <TableRow>
      <TableCell>{doc.title}</TableCell>
      <TableCell>
        <StatusBadge status={doc.status} />
      </TableCell>
      {/* ... */}
    </TableRow>
  );
});

// Custom comparison for complex objects
const DocumentRow = memo(
  function DocumentRow({ doc }: { doc: Document }) {
    // ...
  },
  (prev, next) =>
    prev.doc.id === next.doc.id && prev.doc.status === next.doc.status
);
```

---

#### useMemo for Expensive Computations

```tsx
// Graph data transformation
const graphData = useMemo(() => {
  return transformRawGraph(rawData, {
    filter: filters,
    sort: sortConfig,
  });
}, [rawData, filters, sortConfig]);

// Search engine initialization
const searchEngine = useMemo(() => {
  if (!nodes.length) return null;

  const miniSearch = new MiniSearch({
    idField: "id",
    fields: ["label"],
  });

  miniSearch.addAll(nodes);
  return miniSearch;
}, [nodes]);
```

---

#### useCallback for Event Handlers

```tsx
// Stable callback references
const handleNodeClick = useCallback(
  (nodeId: string) => {
    selectNode(nodeId);
  },
  [selectNode]
);

const handleSearch = useCallback(
  debounce((query: string) => {
    setSearchResults(searchEngine?.search(query) ?? []);
  }, 300),
  [searchEngine]
);
```

---

### 3. Virtual Scrolling

#### For Document Tables

```tsx
import { useVirtualizer } from "@tanstack/react-virtual";

function DocumentTable({ documents }: { documents: Document[] }) {
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: documents.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 48, // Row height
    overscan: 10,
  });

  return (
    <div ref={parentRef} className="h-[600px] overflow-auto">
      <div style={{ height: virtualizer.getTotalSize() }}>
        {virtualizer.getVirtualItems().map((virtualRow) => (
          <DocumentRow
            key={documents[virtualRow.index].id}
            doc={documents[virtualRow.index]}
            style={{
              transform: `translateY(${virtualRow.start}px)`,
            }}
          />
        ))}
      </div>
    </div>
  );
}
```

**When to Use Virtualization:**

- Document list > 50 items
- Query history > 100 items
- Any scrollable list with > 100 items

---

### 4. Debouncing & Throttling

#### Input Debouncing

```tsx
// Search input - debounce 300ms
const debouncedSearch = useMemo(
  () =>
    debounce((value: string) => {
      setSearchQuery(value);
    }, 300),
  []
);

// Cleanup on unmount
useEffect(() => {
  return () => debouncedSearch.cancel();
}, [debouncedSearch]);
```

#### Scroll Throttling

```tsx
// Scroll handlers - throttle to 16ms (60fps)
const throttledScroll = useMemo(
  () =>
    throttle((e: Event) => {
      updateScrollPosition(e);
    }, 16),
  []
);
```

---

## Data Loading Strategies

### 1. Pagination Implementation

```tsx
// Server-side pagination
const { data, isLoading } = useQuery({
  queryKey: ["documents", page, pageSize, filters],
  queryFn: () =>
    getDocuments({
      page,
      page_size: pageSize,
      status: filters.status,
      sort_by: sortField,
      sort_order: sortDirection,
    }),
  placeholderData: keepPreviousData, // Avoid loading flash
});
```

---

### 2. Infinite Scrolling (Alternative)

```tsx
import { useInfiniteQuery } from "@tanstack/react-query";

const { data, fetchNextPage, hasNextPage, isFetchingNextPage } =
  useInfiniteQuery({
    queryKey: ["documents", filters],
    queryFn: ({ pageParam = 1 }) =>
      getDocuments({
        page: pageParam,
        page_size: 20,
      }),
    getNextPageParam: (lastPage) =>
      lastPage.has_more ? lastPage.page + 1 : undefined,
  });
```

---

### 3. Prefetching

```tsx
// Prefetch next page
const queryClient = useQueryClient();

useEffect(() => {
  if (data?.has_more) {
    queryClient.prefetchQuery({
      queryKey: ["documents", page + 1, pageSize, filters],
      queryFn: () =>
        getDocuments({
          page: page + 1,
          page_size: pageSize,
        }),
    });
  }
}, [data, page, pageSize, filters, queryClient]);
```

---

### 4. Stale-While-Revalidate

```tsx
const { data } = useQuery({
  queryKey: ["graph"],
  queryFn: getGraph,
  staleTime: 5 * 60 * 1000, // 5 minutes
  gcTime: 30 * 60 * 1000, // 30 minutes cache
});
```

---

### 5. Optimistic Updates

```tsx
const deleteMutation = useMutation({
  mutationFn: deleteDocument,
  onMutate: async (docId) => {
    // Cancel outgoing refetches
    await queryClient.cancelQueries({ queryKey: ["documents"] });

    // Snapshot previous value
    const previousDocs = queryClient.getQueryData(["documents"]);

    // Optimistically update
    queryClient.setQueryData(["documents"], (old: any) => ({
      ...old,
      items: old.items.filter((d: Document) => d.id !== docId),
    }));

    return { previousDocs };
  },
  onError: (err, docId, context) => {
    // Rollback on error
    queryClient.setQueryData(["documents"], context?.previousDocs);
  },
  onSettled: () => {
    // Refetch to ensure sync
    queryClient.invalidateQueries({ queryKey: ["documents"] });
  },
});
```

---

## Rendering Optimizations

### 1. Graph Rendering

#### Progressive Loading

```tsx
// Load graph in stages
async function loadGraphProgressive() {
  // Stage 1: Load top 100 nodes (fast)
  const initial = await getGraph({ limit: 100 });
  setGraph(initial);

  // Stage 2: Load remaining nodes (background)
  const full = await getGraph({ limit: 1000 });
  setGraph(full);
}
```

#### Level of Detail (LOD)

```tsx
// Reduce detail at low zoom levels
const sigmaSettings = useMemo(
  () => ({
    labelRenderedSizeThreshold: zoomLevel < 0.5 ? 999 : 12,
    renderEdgeLabels: zoomLevel > 0.7,
  }),
  [zoomLevel]
);
```

---

### 2. Markdown Rendering

#### Stream-Safe Rendering

```tsx
// Only render when content is complete
const [isComplete, setIsComplete] = useState(false);

const safeContent = useMemo(() => {
  if (!isComplete) {
    // During streaming, use simple text
    return <pre className="whitespace-pre-wrap">{content}</pre>;
  }
  // After complete, render full markdown
  return <MarkdownRenderer content={content} />;
}, [content, isComplete]);
```

#### Lazy Plugin Loading

```tsx
// Load KaTeX only when needed
const [plugins, setPlugins] = useState<any[]>([]);

useEffect(() => {
  const hasLatex = content.includes("$");
  if (hasLatex) {
    Promise.all([import("remark-math"), import("rehype-katex")]).then(
      ([remarkMath, rehypeKatex]) => {
        setPlugins([remarkMath.default, rehypeKatex.default]);
      }
    );
  }
}, [content]);
```

---

### 3. Tab Visibility Optimization

```tsx
// Pause expensive operations when tab is hidden
function useTabVisibility() {
  const [isVisible, setIsVisible] = useState(true);

  useEffect(() => {
    const handler = () => setIsVisible(!document.hidden);
    document.addEventListener("visibilitychange", handler);
    return () => document.removeEventListener("visibilitychange", handler);
  }, []);

  return isVisible;
}

// Usage
const isVisible = useTabVisibility();

// Pause graph animation when hidden
useEffect(() => {
  if (!isVisible) {
    sigma?.getLayoutRunner()?.stop();
  } else {
    sigma?.getLayoutRunner()?.start();
  }
}, [isVisible, sigma]);
```

---

## Bundle Size Optimization

### 1. Tree Shaking

Ensure proper imports:

```tsx
// ❌ Bad - imports entire library
import * as LucideIcons from "lucide-react";

// ✅ Good - tree shakeable
import { Search, Upload, Trash2 } from "lucide-react";
```

---

### 2. Dependency Analysis

Run bundle analysis:

```bash
bun add -D @next/bundle-analyzer

# next.config.ts
const withBundleAnalyzer = require('@next/bundle-analyzer')({
  enabled: process.env.ANALYZE === 'true',
});

module.exports = withBundleAnalyzer({ /* config */ });

# Run analysis
ANALYZE=true bun run build
```

---

### 3. Heavy Dependencies Alternatives

| Current  | Alternative        | Savings |
| -------- | ------------------ | ------- |
| `moment` | `date-fns`         | -50KB   |
| `lodash` | Individual imports | -30KB   |
| `axios`  | Native fetch       | -15KB   |

---

### 4. Image Optimization

```tsx
import Image from "next/image";

// Use Next.js Image component
<Image
  src="/logo.png"
  alt="EdgeQuake"
  width={120}
  height={40}
  priority // For above-fold images
/>;
```

---

## Monitoring & Metrics

### 1. Performance Metrics Collection

```tsx
// Collect Core Web Vitals
import { onCLS, onFID, onLCP, onTTFB } from "web-vitals";

export function reportWebVitals(metric: Metric) {
  console.log(metric);

  // Send to analytics
  analytics.track("web_vital", {
    name: metric.name,
    value: metric.value,
    rating: metric.rating,
  });
}

// In layout
useEffect(() => {
  onCLS(reportWebVitals);
  onFID(reportWebVitals);
  onLCP(reportWebVitals);
  onTTFB(reportWebVitals);
}, []);
```

---

### 2. Performance Budgets

| Resource          | Budget       |
| ----------------- | ------------ |
| JavaScript (main) | < 250KB gzip |
| CSS               | < 50KB gzip  |
| Total transfer    | < 500KB      |
| Image per page    | < 200KB      |

---

### 3. Real User Monitoring

```tsx
// Track page load times
const navigationEntry = performance.getEntriesByType("navigation")[0];

// Track component render times
const startTime = performance.now();
// ... render
const renderTime = performance.now() - startTime;

// Track API response times
const apiStart = performance.now();
const response = await fetch("/api/documents");
const apiTime = performance.now() - apiStart;
```

---

### 4. Performance Dashboard Metrics

| Metric       | Good    | Needs Work | Poor    |
| ------------ | ------- | ---------- | ------- |
| Page Load    | < 2s    | 2-4s       | > 4s    |
| API Response | < 200ms | 200-500ms  | > 500ms |
| Graph Render | < 1s    | 1-3s       | > 3s    |
| Theme Switch | < 50ms  | 50-200ms   | > 200ms |

---

## Implementation Checklist

### Phase 1 (Week 1-2)

- [ ] Implement pagination for documents
- [ ] Add lazy loading for GraphViewer
- [ ] Add React.memo to list items
- [ ] Implement debounced search

### Phase 2 (Week 3-4)

- [ ] Add virtual scrolling for long lists
- [ ] Implement progressive graph loading
- [ ] Add tab visibility optimization
- [ ] Lazy load KaTeX and Mermaid

### Phase 3 (Week 5-6)

- [ ] Bundle analysis and optimization
- [ ] Image optimization
- [ ] Performance monitoring setup
- [ ] Core Web Vitals tracking

---

## Cross-References

| Document                                          | Relationship             |
| ------------------------------------------------- | ------------------------ |
| [Gap Analysis](./002-gap-analysis.md)             | Performance-related gaps |
| [Proposed Solutions](./003-proposed-solutions.md) | Implementation details   |
| [QA Plan](./007-qa-plan.md)                       | Performance testing      |
| [Success Criteria](./008-success-criteria.md)     | Performance benchmarks   |

---

_Document defines performance optimization approaches and metrics_
