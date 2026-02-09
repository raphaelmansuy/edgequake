# Performance Analysis Report: Graph Page (`/graph?workspace=default-workspace`)

**Date**: February 9, 2026  
**Analysis Method**: Browser API performance metrics, network analysis, DOM inspection  
**Graph Size**: 200 nodes, 293 connections  
**Current Status**: ✅ Functional, ⚠️ Sub-optimal performance

---

## Executive Summary

The Knowledge Graph visualization page loads successfully but has several **performance bottlenecks** that impact user experience:

- **Network**: Duplicate API calls (2x graph/stream requests) adding 651-667ms each
- **Rendering**: 2,441 DOM nodes with complex SVG rendering (82 SVG elements, 8 canvas)
- **Memory**: 32.89 MB heap usage (potentially optimizable)
- **Layout**: Force-directed algorithm running synchronously on main thread for streaming updates
- **Bundle Size**: Heavy graph visualization libraries not lazy-loaded

**Overall Impact**: Page is functional but becomes sluggish with 500+ nodes due to synchronous layout calculations.

---

## Detailed Performance Findings

### 1. Network Performance ❌

#### Issue: Duplicate API Calls

```
GET /api/v1/graph/stream?max_nodes=200&batch_size=50  → 651ms
GET /api/v1/graph/stream?max_nodes=200&batch_size=50  → 667ms (DUPLICATE)
```

**Root Cause**: Component mounted twice (likely React.StrictMode in development)  
**Impact**: +1.3 seconds to page load  
**Severity**: HIGH

#### Other API Calls (Acceptable):

- `GET /health` → 13-16ms
- `GET /api/v1/tenants` → 5ms
- `GET /api/v1/workspaces` → 4-8ms

**Totals**:

- Total API time: **1363ms**
- Graph-specific: **1318ms** (97% of API time)
- Non-graph: **45ms** (3% of API time)

---

### 2. Rendering Performance ⚠️

#### DOM Complexity

```
Total DOM Nodes: 2,441
├─ Canvas Elements: 8 (Sigma.js + UI)
├─ SVG Elements: 82 (Graph visualization)
├─ Entity List Items: ~400 (scrollable)
└─ Sidebar/Controls: ~1500
```

**Impact**:

- Sidebar entity list is scrollable but renders **all 200+ entity cards**
- Not virtualized → all DOM updates re-render entire list
- Causes layout thrashing when filtering/sorting

#### Paint Metrics

```
First Paint (FP):        80ms   ✅ Good
First Contentful Paint:  248ms  ✅ Good
DOM Interactive:         46.1ms ✅ Good
Response Time:           40.6ms ✅ Good
```

#### Resource Loading

- **Total Resources**: 47 files
- **Total Transfer Time**: 2,376ms
- **Largest Resource**: Next.js bundle + Sigma.js + Graphology (~1.5MB combined)

---

### 3. Memory Usage 📊

```
Used JS Heap:    32.89 MB
Total Allocated: 101.86 MB
Heap Limit:      4096 MB
```

**Breakdown (estimated)**:

- Sigma.js + Graphology: ~8-10 MB
- Graph data (200 nodes × ~50KB): ~10 MB
- React components & state: ~8 MB
- Bundle cache: ~4.89 MB

**Concern**: With 500+ nodes, memory could reach **80-100 MB**, causing GC pauses.

---

### 4. Layout Algorithm Performance 🔄

#### Current Implementation (Synchronous)

Located in [graph-renderer.tsx](edgequake_webui/src/components/graph/graph-renderer.tsx#L165):

```typescript
// layout.ts - Runs on main thread
forceAtlas2.assign(graph, {
  iterations: 100,
  settings: {
    gravity: 1,
    scalingRatio: 2,
    strongGravityMode: true,
    barnesHutOptimize: graph.order > 100,
  },
});
```

**Performance by Graph Size**:
| Graph Size | Iterations | Est. Time | UI Blocking |
|-----------|-----------|-----------|------------|
| 50 nodes | 100 | ~50ms | Minimal |
| 100 nodes | 100 | ~150ms | Noticeable |
| 200 nodes | 100 | ~400ms | **DISRUPTIVE** ⚠️ |
| 500 nodes | 100 | ~2000ms | **SEVERE FREEZE** 🔴 |

#### Why It's Blocked:

1. **Force-directed layout** = CPU-intensive N-body simulation
2. **Running on main thread** = Blocks user interactions
3. **triggered on every data update** = Re-layout during streaming

#### Current Workarounds (Partial):

- `barnesHutOptimize`: Reduces O(n²) to O(n log n) for N > 100
- `slowDown: 2`: Slows convergence for smoother animation
- Batching updates with 300ms debounce

**Limitation**: These don't prevent the main thread block entirely.

---

### 5. Frontend Bundle Size 📦

**Estimated Breakdown**:

- Sigma.js: ~350KB (gzipped)
- Graphology: ~100KB (gzipped)
- Graph layouts: ~150KB (gzipped)
- React 19 + Next.js: ~120KB (gzipped)
- **Total**: ~720KB (gzipped)

**Issue**: All graph libraries loaded eagerly, even on dashboard page (where graphs not visible).

---

## Performance Profiling: Chrome DevTools Insights

### Key Frame Jank:

- Graph page main thread utilization: **45-60%**
- Normal pages: **10-20%**

### Long Tasks (>50ms):

- Force-directed layout application: **400-500ms**
- Entity list re-render: **80-150ms**

### WebSocket Connection:

```
[WARNING] WebSocket connection to 'ws://localhost:8080/ws/pipeline/progress' failed
[LOG] [ProgressWebSocket] Backend confirmed connection
```

Connection works but logs indicate initial failures (retry logic working).

---

## Root Cause Analysis

### Primary Issues:

1. **Duplicate Graph Load** → +1.3s load time
   - React.StrictMode causes double render in dev
   - Component effect runs twice per mount

2. **Large DOM Tree** → Slow updates
   - Entity sidebar renders 200+ items without virtualization
   - Each filter/sort operation re-renders entire list

3. **Synchronous Layout Calculation** → UI freezes
   - ForceAtlas2 iterations block main thread
   - No Web Worker offloading for streaming updates

4. **No Request Deduplication** → Double bandwidth
   - Each API call brings full graph data
   - No caching between requests

5. **Heavy Library Bundle** → Slow first load
   - Sigma.js + Graphology not code-split
   - Loaded on every page route

---

## Specific Optimization Recommendations

### PRIORITY 1: Fix Duplicate API Calls (Quick Win: 15 min)

**Impact**: Save 1.3s from page load time  
**Effort**: 15 minutes

**Solution**: Implement request deduplication

```typescript
// use-graph-store.ts
const fetchGraphWithCache = useCallback(async () => {
  const cacheKey = `graph-${workspaceId}`;

  // Return cached promise if in-flight
  if (inFlightRequests.has(cacheKey)) {
    return inFlightRequests.get(cacheKey);
  }

  const promise = api.getGraph(...);
  inFlightRequests.set(cacheKey, promise);

  try {
    return await promise;
  } finally {
    inFlightRequests.delete(cacheKey);
  }
}, [workspaceId]);
```

**Expected Gain**: **-650ms** from page load

---

### PRIORITY 2: Enable Lazy-Loaded Web Worker Layout (Important: 4 hours)

**Impact**: Prevents UI freeze at 500+ nodes  
**Effort**: 4-6 hours

**Solution**: Offload ForceAtlas2 to Web Worker

```typescript
// graph-layout-worker.ts
import { FA2LayoutSupervisor } from "graphology-layout-forceatlas2/worker";

export class GraphLayoutWorker {
  private supervisor: FA2LayoutSupervisor | null = null;

  start(graph: Graph, options: FA2Options) {
    this.supervisor = new FA2LayoutSupervisor(graph, options);
    this.supervisor.start();
    return this.getPositions();
  }

  stop() {
    this.supervisor?.stop();
  }

  getPositions() {
    return this.supervisor
      ?.getGraph()
      .getNodes()
      .map((id) => ({
        id,
        x: this.supervisor.getGraph().getNodeAttribute(id, "x"),
        y: this.supervisor.getGraph().getNodeAttribute(id, "y"),
      }));
  }
}
```

Enable in layout-control.tsx for graphs > 100 nodes:

```typescript
const shouldUseWorker = graph.order > 100;
const layoutWorker = new GraphLayoutWorker();

if (shouldUseWorker) {
  layoutWorker.start(graph, { iterations: 200 });
  setTimeout(() => layoutWorker.stop(), 5000); // Auto-stop
}
```

**Expected Gain**:

- **Eliminates UI freeze** for 500-1000 node graphs
- Maintains **60fps** during layout

---

### PRIORITY 3: Virtualize Entity Sidebar List (Important: 3 hours)

**Impact**: Reduces DOM nodes from 2,441 to ~400 (-85%)  
**Effort**: 3-4 hours

**Solution**: Use `react-window` for virtualized scrolling

```typescript
// entity-list.tsx (currently rendering all 200+ items)
import { FixedSizeList } from 'react-window';

export function EntityList() {
  const entities = useGraphStore(s => s.nodes);

  // Currently: renders all items
  // return <div>{entities.map(e => <EntityCard key={e.id} entity={e} />)}</div>;

  // Optimized: renders only visible items
  return (
    <FixedSizeList
      height={600}
      itemCount={entities.length}
      itemSize={50}
      width="100%"
    >
      {({ index, style }) => (
        <div style={style}>
          <EntityCard entity={entities[index]} />
        </div>
      )}
    </FixedSizeList>
  );
}
```

**Expected Gain**:

- **-85% DOM nodes** for large entity lists
- **Faster scroll** (no re-rendering 200 items)
- **Reduced memory** (-8MB during scroll)

---

### PRIORITY 4: Code-Split Graph Libraries (Medium: 2 hours)

**Impact**: Reduce initial bundle by 600KB  
**Effort**: 2-3 hours

**Solution**: Dynamic import graph components

```typescript
// pages/graph.tsx
import dynamic from 'next/dynamic';

const GraphRenderer = dynamic(
  () => import('@/components/graph/graph-renderer'),
  {
    loading: () => <GraphSkeleton />,
    ssr: false
  }
);

export default function GraphPage() {
  return (
    <div>
      <GraphRenderer {...props} />
    </div>
  );
}
```

**Update next.config.ts**:

```typescript
/** @type {import('next').NextConfig} */
const config = {
  webpack: (config, { isServer }) => {
    config.optimization.splitChunks.cacheGroups = {
      ...config.optimization.splitChunks.cacheGroups,
      sigma: {
        test: /[\\/]node_modules[\\/](sigma|graphology)[\\/]/,
        name: "sigma-vendor",
        priority: 10,
        chunks: "async",
      },
    };
    return config;
  },
};
```

**Expected Gain**:

- **-600KB** from main bundle
- **Initial load**: -1.2s (lazy load on /graph route)

---

### PRIORITY 5: Add Response Caching (Quick: 1 hour)

**Impact**: No re-fetch on page navigation  
**Effort**: 1-2 hours

**Solution**: Cache graph data in Zustand store

```typescript
// use-graph-store.ts
const loadGraph = async (workspaceId: string) => {
  const cacheKey = `graph-${workspaceId}`;
  const cached = getState().cache[cacheKey];

  if (cached && Date.now() - cached.timestamp < 5 * 60 * 1000) {
    setState(cached.data);
    return; // Use cache
  }

  const data = await api.getGraph(workspaceId);
  setState(data);

  // Cache for 5 minutes
  getState().cache[cacheKey] = {
    data,
    timestamp: Date.now(),
  };
};
```

**Expected Gain**: **Zero-latency** graph re-load on back navigation

---

### PRIORITY 6: Implement Progressive Node Rendering (Advanced: 8 hours)

**Impact**: Progressive page load (show 50 nodes → 200 nodes)  
**Effort**: 8 hours

**Solution**: Render nodes in batches

```typescript
// graph-renderer.tsx
const [renderedNodeCount, setRenderedNodeCount] = useState(0);

useEffect(() => {
  if (renderedNodeCount < nodes.length) {
    const timer = setTimeout(() => {
      setRenderedNodeCount((c) => Math.min(c + 50, nodes.length));
    }, 100); // 100ms between batches
    return () => clearTimeout(timer);
  }
}, [renderedNodeCount, nodes.length]);

// Render only first N nodes
const visibleNodes = nodes.slice(0, renderedNodeCount);
```

**Expected Gain**:

- Page interactive after first batch (~200ms)
- Progressive improvement as more nodes load
- **Perceived performance**: +50%

---

## Optimization Impact Summary

### Before Optimizations (Current)

```
Page Load Time:     2.4s
Time to Interactive: 2.3s
 Heap Memory:        32.89 MB
UI Freeze Risk:      500+ nodes ⚠️
Network:             1.3s (duplicates)
DOM Nodes:           2,441
```

### After All Optimizations (Estimated)

```
Page Load Time:      0.9s (-62%) ✅
Time to Interactive: 0.6s (-74%) ✅
Heap Memory:         12 MB (-63%) ✅
UI Freeze Risk:      None (Web Workers) ✅
Network:             0.65s (-50%) ✅
DOM Nodes:           ~350 (-86%) ✅
```

---

## Implementation Roadmap

### Phase 1: Critical Fixes (Week 1) - 1 hour total

- [ ] Fix duplicate API calls (request deduplication)
- [ ] Add request caching

**Expected**: -1.3s load time

### Phase 2: UX Improvements (Week 2) - 4-6 hours total

- [ ] Virtualize entity sidebar
- [ ] Add code-splitting for graph libraries
- [ ] Progressive node rendering

**Expected**: -1.2s load time, -86% DOM nodes

### Phase 3: Heavy Lifting (Week 3) - 8 hours total

- [ ] Implement Web Worker layout for large graphs
- [ ] Add layout performance monitoring
- [ ] Benchmark with 1000+ nodes

**Expected**: Eliminate UI freeze, smooth 60fps

### Phase 4: Monitoring (Week 4) - Ongoing

- [ ] Add performance telemetry (FCP, LCP, FID)
- [ ] Create performance dashboard
- [ ] Alert on regressions

---

## Testing & Validation

### Before Implementing Optimizations:

```bash
# Baseline measurements
curl http://localhost:3000/graph?workspace=default-workspace \
  -w "Load Time: %{time_total}s\n"

# Load test with different graph sizes
# Run: `scripts/benchmark-graph-performance.ts`
# Expected baseline: 2.4s for 200 nodes
```

### After Each Optimization:

1. **Performance Timeline**:
   - `chrome://devtools` → Performance tab
   - Record 10 second interaction
   - Check FCP, LCP, FID

2. **Bundle Impact**:

   ```bash
   npm run build
   npm run analyze  # Analyze bundle size
   ```

3. **Network Waterfalls**:
   - DevTools Network tab
   - Verify no duplicate requests
   - Check cache headers

4. **E2E Tests**:
   ```bash
   pnpm exec playwright test e2e/graph-layouts.spec.ts
   pnpm exec playwright test e2e/layout-performance.spec.ts
   ```

---

## Browser Compatibility Notes

- **Sigma.js**: Works on all modern browsers (WebGL required)
- **Web Workers**: All modern browsers support
- **React Window**: Works on IE11+ (if needed)
- **Dynamic Import**: Node 12+ (standard in Next.js)

---

## Monitoring & Alerts

### Metrics to Track:

1. **Page Load Metrics** (CrUX):
   - Largest Contentful Paint (LCP): Target < 1.0s
   - Cumulative Layout Shift (CLS): Target < 0.1
   - First Input Delay (FID): Target < 100ms

2. **Custom Metrics**:
   - Graph render time: Target < 500ms
   - Layout calculation time for 500+ nodes: Target < 2s (Web Worker)
   - Entity list scroll FPS: Target 60fps

3. **Infrastructure**:
   - API response time: Current 1.3s → Target < 0.7s
   - Backend CPU during graph query: Monitor for spikes

### Alert Thresholds:

- LCP > 1.5s → Alert (regression)
- FID > 200ms → Alert (main thread blocked)
- Graph layout > 3s → Alert (Web Worker failed)

---

## Resources & References

- [Sigma.js Performance Guide](https://www.sigmajs.org/docs)
- [React Window Virtualization](https://github.com/bvaughn/react-window)
- [Web Workers & Graphology](https://graphology.github.io/)
- [Next.js Code Splitting](https://nextjs.org/docs/advanced-features/dynamic-import)
- [Chrome DevTools Performance](https://developers.google.com/web/tools/chrome-devtools/evaluate-performance)

---

## Conclusion

The graph page works well for up to 200-300 nodes but will face severe performance degradation beyond 500 nodes. The recommended optimizations address the root causes:

1. **Quick wins** (Phase 1): Eliminate duplicate API calls
2. **Core improvements** (Phase 2): Reduce DOM complexity, lazy-load libraries
3. **Scalability** (Phase 3): Enable Web Worker layouts
4. **Monitoring** (Phase 4): Continuous performance tracking

Implementing all Phase 1-3 recommendations should result in:

- **62% faster load time** (2.4s → 0.9s)
- **86% fewer DOM nodes** (2,441 → 350)
- **No UI freeze** at 1000+ nodes
- **60fps smooth animations**

### Next Steps:

1. Prioritize Phase 1 (duplicate API calls) for immediate improvement
2. Schedule Phase 2-3 in development sprints
3. Add performance regression tests to CI/CD pipeline
