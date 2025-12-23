# Performance Optimization Strategy

> **Document Version:** 1.0  
> **Date:** 2024-12-23  
> **Purpose:** Frontend and backend optimization strategies

---

## Table of Contents

1. [Performance Baseline](#performance-baseline)
2. [Core Web Vitals Targets](#core-web-vitals-targets)
3. [Frontend Optimizations](#frontend-optimizations)
4. [Backend Optimizations](#backend-optimizations)
5. [Caching Strategy](#caching-strategy)
6. [Monitoring & Metrics](#monitoring--metrics)
7. [Implementation Priorities](#implementation-priorities)

---

## Performance Baseline

### Current Observations

| Metric                    | Current (Est.) | Target       |
| ------------------------- | -------------- | ------------ |
| First Contentful Paint    | ~1.5s          | < 1.0s       |
| Largest Contentful Paint  | ~2.5s          | < 2.0s       |
| Time to Interactive       | ~3.0s          | < 2.5s       |
| Bundle Size (JS)          | ~450KB gzip    | < 350KB gzip |
| Graph Render (100 nodes)  | ~200ms         | < 150ms      |
| Graph Render (1000 nodes) | ~1.5s          | < 800ms      |
| API Response (query)      | ~500ms         | < 400ms      |

### Measurement Tools

- **Lighthouse:** Core Web Vitals audit
- **Chrome DevTools:** Performance profiling
- **Bundle Analyzer:** Webpack/Next.js bundle analysis
- **React DevTools:** Component render profiling
- **Custom Metrics:** Application-specific timers

---

## Core Web Vitals Targets

| Metric | Description               | Target  | Priority |
| ------ | ------------------------- | ------- | -------- |
| LCP    | Largest Contentful Paint  | < 2.0s  | High     |
| INP    | Interaction to Next Paint | < 200ms | High     |
| CLS    | Cumulative Layout Shift   | < 0.1   | Medium   |
| FCP    | First Contentful Paint    | < 1.0s  | High     |
| TTFB   | Time to First Byte        | < 200ms | Medium   |

---

## Frontend Optimizations

### 1. Bundle Size Reduction

**Current State:**

- Next.js with full bundle
- All components in main bundle
- No code splitting beyond pages

**Optimizations:**

**a) Dynamic Imports for Heavy Components**

```tsx
// Lazy load graph viewer (Sigma.js is ~100KB)
const GraphViewer = dynamic(() => import("@/components/graph/GraphViewer"), {
  loading: () => <GraphSkeleton />,
  ssr: false, // Sigma.js doesn't support SSR
});

// Lazy load markdown components
const MarkdownRenderer = dynamic(
  () => import("@/components/chat/MarkdownRenderer"),
  { loading: () => <Skeleton className="h-4 w-full" /> }
);

// Lazy load heavy dialogs
const SettingsDialog = dynamic(
  () => import("@/components/dialogs/SettingsDialog")
);
```

**b) Tree Shaking Improvements**

```tsx
// Bad: imports entire library
import { Button, Card, Dialog, ... } from '@radix-ui/themes'

// Good: imports only what's needed
import { Button } from '@radix-ui/react-button'
import { Card } from '@radix-ui/react-card'
```

**c) Bundle Analysis**

```bash
# Add to package.json
"analyze": "ANALYZE=true next build"

# In next.config.ts
const withBundleAnalyzer = require('@next/bundle-analyzer')({
  enabled: process.env.ANALYZE === 'true',
})
```

---

### 2. Rendering Optimizations

**a) Memoization Strategy**

```tsx
// Memoize expensive computations
const filteredNodes = useMemo(
  () => nodes.filter((n) => n.label.includes(searchTerm)),
  [nodes, searchTerm]
);

// Memoize callbacks to prevent child re-renders
const handleNodeClick = useCallback((nodeId: string) => {
  setSelectedNode(nodeId);
}, []);

// Memoize component when props rarely change
const NodeDetails = memo(function NodeDetails({ node }: Props) {
  return <div>{/* expensive render */}</div>;
});
```

**b) Virtualization for Large Lists**

```tsx
// Use @tanstack/react-virtual for long lists
import { useVirtualizer } from "@tanstack/react-virtual";

function DocumentList({ documents }: Props) {
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: documents.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 64, // row height
    overscan: 5, // render extra items
  });

  return (
    <div ref={parentRef} className="h-[400px] overflow-auto">
      <div style={{ height: virtualizer.getTotalSize() }}>
        {virtualizer.getVirtualItems().map((virtual) => (
          <div
            key={virtual.key}
            style={{
              height: virtual.size,
              transform: `translateY(${virtual.start}px)`,
            }}
          >
            <DocumentRow document={documents[virtual.index]} />
          </div>
        ))}
      </div>
    </div>
  );
}
```

**c) React Query Optimizations**

```tsx
// Already using React Query - optimize configuration
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5, // 5 minutes
      gcTime: 1000 * 60 * 30, // 30 minutes garbage collection
      refetchOnWindowFocus: false, // Disable for stable data
      retry: 1, // Single retry
    },
  },
});
```

---

### 3. Tab Visibility Optimization

**Problem:** Polling continues when tab is hidden, wasting resources.

**Solution:**

```tsx
// Global hook for visibility-aware polling
function useVisibilityPolling<T>(
  queryKey: QueryKey,
  queryFn: () => Promise<T>,
  options?: { pollingInterval?: number }
) {
  const isVisible = usePageVisibility();

  return useQuery({
    queryKey,
    queryFn,
    refetchInterval: isVisible ? options?.pollingInterval : false,
    refetchIntervalInBackground: false,
  });
}

// Hook to detect page visibility
function usePageVisibility() {
  const [isVisible, setIsVisible] = useState(true);

  useEffect(() => {
    const handleVisibility = () => {
      setIsVisible(document.visibilityState === "visible");
    };
    document.addEventListener("visibilitychange", handleVisibility);
    return () =>
      document.removeEventListener("visibilitychange", handleVisibility);
  }, []);

  return isVisible;
}
```

---

### 4. Graph Performance

**a) WebGL Renderer**

```tsx
// Use WebGL for large graphs (>500 nodes)
import { WebGLSettings } from "@sigma/node-image";

const sigma = new Sigma(graph, container, {
  renderer: {
    type: "webgl",
  },
  renderEdgeLabels: nodeCount < 200, // Disable for large graphs
  labelRenderedSizeThreshold: 6, // Only render labels at zoom level
});
```

**b) Progressive Loading**

```tsx
// Load graph in chunks for very large datasets
async function loadGraphProgressively(graphData: GraphData) {
  const CHUNK_SIZE = 100;

  // Load nodes first
  for (let i = 0; i < graphData.nodes.length; i += CHUNK_SIZE) {
    const chunk = graphData.nodes.slice(i, i + CHUNK_SIZE);
    await addNodesToGraph(chunk);
    await new Promise((r) => requestAnimationFrame(r)); // Allow render
  }

  // Then load edges
  for (let i = 0; i < graphData.edges.length; i += CHUNK_SIZE) {
    const chunk = graphData.edges.slice(i, i + CHUNK_SIZE);
    await addEdgesToGraph(chunk);
    await new Promise((r) => requestAnimationFrame(r));
  }
}
```

**c) Level of Detail**

```tsx
// Simplify rendering at low zoom levels
sigma.on("afterRender", () => {
  const ratio = sigma.getCamera().ratio;
  if (ratio > 0.5) {
    // Zoomed out: hide labels, simplify edges
    sigma.setSetting("renderLabels", false);
    sigma.setSetting("renderEdgeLabels", false);
  } else {
    // Zoomed in: show details
    sigma.setSetting("renderLabels", true);
  }
});
```

---

### 5. Image & Asset Optimization

**a) Next.js Image Optimization**

```tsx
// Use next/image for all images
import Image from "next/image";

<Image
  src="/logo.png"
  width={100}
  height={50}
  alt="Logo"
  priority // For above-the-fold images
/>;
```

**b) Font Optimization**

```tsx
// In layout.tsx - use next/font
import { Inter } from "next/font/google";

const inter = Inter({
  subsets: ["latin"],
  display: "swap", // Prevent FOUT
  preload: true,
});
```

---

## Backend Optimizations

### 1. API Response Optimization

**a) Pagination**

```rust
// Server-side pagination for large datasets
pub async fn list_entities(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<Entity>>, ApiError> {
    let (entities, total) = state.storage
        .list_entities(params.page, params.per_page)
        .await?;

    Ok(Json(PaginatedResponse {
        data: entities,
        page: params.page,
        per_page: params.per_page,
        total,
    }))
}
```

**b) Compression**

```rust
// Enable gzip compression in Axum
use tower_http::compression::CompressionLayer;

Router::new()
    .route("/api/graph", get(get_graph))
    .layer(CompressionLayer::new())
```

**c) Field Selection**

```rust
// Allow clients to select fields
// GET /api/entities?fields=id,label,type

pub async fn list_entities(
    Query(params): Query<EntityListParams>,
) -> Json<Vec<PartialEntity>> {
    let entities = fetch_entities().await;

    // Return only requested fields
    entities.into_iter()
        .map(|e| e.select_fields(&params.fields))
        .collect()
}
```

---

### 2. Streaming Responses

**Current Advantage:** EdgeQuake already implements streaming.

**Optimizations:**

```rust
// Flush chunks immediately
pub async fn stream_query_response(
    Query(params): Query<QueryParams>,
) -> impl IntoResponse {
    let stream = async_stream::stream! {
        let mut response = llm.query(&params.query).await;

        while let Some(chunk) = response.next().await {
            // Yield immediately without buffering
            yield Ok::<_, std::io::Error>(
                format!("data: {}\n\n", serde_json::to_string(&chunk)?)
            );
        }
    };

    Sse::new(stream)
        .keep_alive(KeepAlive::default())
}
```

---

## Caching Strategy

### Layer 1: Browser Cache

```tsx
// Static assets: aggressive caching
// next.config.ts
headers: async () => [
  {
    source: "/:all*(svg|jpg|png|woff2)",
    headers: [
      { key: "Cache-Control", value: "public, max-age=31536000, immutable" },
    ],
  },
];
```

### Layer 2: React Query Cache

```tsx
// Data that rarely changes
const { data: graph } = useQuery({
  queryKey: ["graph", workspaceId],
  queryFn: fetchGraph,
  staleTime: 1000 * 60 * 5, // Fresh for 5 min
  gcTime: 1000 * 60 * 30, // Keep in memory 30 min
});

// Data that changes frequently
const { data: pipelineStatus } = useQuery({
  queryKey: ["pipeline", "status"],
  queryFn: fetchPipelineStatus,
  staleTime: 0, // Always refetch
  refetchInterval: 3000, // Poll every 3s
});
```

### Layer 3: Server-Side Cache

```rust
// Redis caching for expensive queries
use redis::AsyncCommands;

pub async fn get_graph_cached(
    workspace_id: &str,
    redis: &mut redis::aio::Connection,
    storage: &Storage,
) -> Result<GraphData, Error> {
    let cache_key = format!("graph:{}", workspace_id);

    // Try cache first
    if let Ok(cached) = redis.get::<_, String>(&cache_key).await {
        return Ok(serde_json::from_str(&cached)?);
    }

    // Cache miss: fetch from storage
    let graph = storage.get_graph(workspace_id).await?;

    // Store in cache for 5 minutes
    redis.set_ex(&cache_key, serde_json::to_string(&graph)?, 300).await?;

    Ok(graph)
}
```

---

## Monitoring & Metrics

### Frontend Metrics

```tsx
// Report Web Vitals to analytics
export function reportWebVitals(metric: Metric) {
  console.log(metric);

  // Send to analytics
  if (window.gtag) {
    gtag("event", metric.name, {
      value: Math.round(metric.value),
      event_label: metric.id,
      non_interaction: true,
    });
  }
}

// Custom timing metrics
function measureGraphRender() {
  const start = performance.now();

  // After render
  const duration = performance.now() - start;
  console.log(`Graph render: ${duration}ms`);

  // Report if slow
  if (duration > 500) {
    reportPerformanceIssue("graph_render_slow", { duration });
  }
}
```

### Backend Metrics

```rust
// Prometheus metrics
use prometheus::{Counter, Histogram};

lazy_static! {
    static ref REQUEST_DURATION: Histogram = register_histogram!(
        "request_duration_seconds",
        "Request latency"
    ).unwrap();

    static ref QUERY_COUNT: Counter = register_counter!(
        "query_total",
        "Total queries processed"
    ).unwrap();
}

// Middleware to track duration
async fn metrics_middleware<B>(
    req: Request<B>,
    next: Next<B>,
) -> Response {
    let timer = REQUEST_DURATION.start_timer();
    let response = next.run(req).await;
    timer.observe_duration();
    response
}
```

---

## Implementation Priorities

### Phase 1: Quick Wins (Week 1)

| Optimization                   | Impact | Effort | Priority |
| ------------------------------ | ------ | ------ | -------- |
| Tab visibility polling         | High   | Low    | P0       |
| React.memo on heavy components | Medium | Low    | P0       |
| Bundle analysis                | Medium | Low    | P0       |
| React Query staleTime tuning   | Medium | Low    | P0       |

### Phase 2: Major Gains (Week 2-3)

| Optimization           | Impact | Effort | Priority |
| ---------------------- | ------ | ------ | -------- |
| Dynamic imports        | High   | Medium | P1       |
| List virtualization    | High   | Medium | P1       |
| Graph WebGL renderer   | High   | Medium | P1       |
| Compression middleware | Medium | Low    | P1       |

### Phase 3: Fine Tuning (Week 4+)

| Optimization                 | Impact | Effort | Priority |
| ---------------------------- | ------ | ------ | -------- |
| Progressive graph loading    | Medium | High   | P2       |
| Server-side caching          | Medium | High   | P2       |
| Custom performance dashboard | Low    | High   | P3       |

---

## Cross-References

- **Gap Analysis:** [001-gap-analysis.md](./001-gap-analysis.md) - GAP-011, GAP-012
- **Proposed Solutions:** [002-proposed-solutions.md](./002-proposed-solutions.md)
- **Success Criteria:** [008-success-criteria.md](./008-success-criteria.md) - Performance benchmarks
- **QA Plan:** [007-qa-plan.md](./007-qa-plan.md) - Performance testing
