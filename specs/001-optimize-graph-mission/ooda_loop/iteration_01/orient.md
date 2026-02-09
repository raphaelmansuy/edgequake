# OODA Iteration 01 - Orient

**Mission**: Optimize Knowledge Graph Display & UX
**Mission File**: `specs/001-optimize-graph-mission/mission.md`
**Date**: 2026-02-09

---

## Analysis & Solutions

### Issue 1: Node Limit (500 max)

**First Principles**:

- Large graphs degrade UX (slow rendering, cluttered view)
- Users need to see manageable subset of graph
- "Load More" should be controlled, not exponential growth

**Options**:
| Option | Pros | Cons | Risk |
|--------|------|------|------|
| A. Hard cap at 500 | Simple, predictable | Users can't load more | Low |
| B. Cap at 500 with smarter filtering | Controlled, flexible | More complex | Med |
| C. Keep 10000 limit | Maximum data | Poor UX at scale | High |

**Recommended**: **Option B** - Cap at 500 but make Load More smarter (load different nodes, not more of same)

**Changes Required**:

```
truncation-banner.tsx:38     → cap at 500
graph-viewer.tsx:550         → cap at 500
use-graph-store.ts           → add MAX_DISPLAY_NODES constant
```

---

### Issue 2: Entity Expand (ID Mismatch)

**First Principles**:

- Entity IDs must be consistent between frontend and backend
- Accented characters need proper handling
- API should be lenient and do its own lookup

**Options**:
| Option | Pros | Cons | Risk |
|--------|------|------|------|
| A. Fix frontend to encode properly | Simple | Doesn't fix root cause | Med |
| B. Backend fuzzy lookup by label | Robust | More DB queries | Low |
| C. Store both ID and label, use ID | Clean | Migration needed | High |

**Recommended**: **Option B** - Backend should try label lookup if ID fails

**Changes Required**:

```
entities.rs:799  → Try get_node, if fails, search_nodes by label
```

---

### Issue 3: Labels Not Visible

**First Principles**:

- Labels are critical for graph comprehension
- LOD (Level of Detail) should balance performance and usability
- At least high-degree nodes should always show labels

**Options**:
| Option | Pros | Cons | Risk |
|--------|------|------|------|
| A. Increase labelDensity for large graphs | More labels visible | May overlap | Low |
| B. Show labels only for selected/hovered | Clean, performant | Less context | Low |
| C. Always show labels for top N by degree | Best of both worlds | Slightly complex | Low |

**Recommended**: **Option C** - Always show labels for nodes with degree > 5

**Changes Required**:

```
graph-renderer.tsx:475  → Increase base labelDensity
graph-renderer.tsx       → Add reducer to always render high-degree labels
```

---

### Issue 4: Search Graph Refresh

**First Principles**:

- Search should help users find AND navigate to entities
- Server search returns nodes not in current view
- Camera should focus on result for best UX

**Options**:
| Option | Pros | Cons | Risk |
|--------|------|------|------|
| A. Focus camera after adding nodes | Best UX | Layout may shift | Low |
| B. Replace entire graph with search results | Clear context | Loses existing context | Med |
| C. Highlight-only, no camera move | Non-disruptive | User may not see result | Med |

**Recommended**: **Option A** - Add nodes, focus camera on selected result

**Changes Required**:

```
graph-search.tsx:handleSelect  → Add camera focus after server search
```

---

## Priority Matrix

| Issue             | Impact | Effort | Priority |
| ----------------- | ------ | ------ | -------- |
| 1. Node limit     | High   | Low    | 1        |
| 3. Labels         | High   | Med    | 2        |
| 4. Search refresh | Med    | Low    | 3        |
| 2. Entity expand  | Med    | Med    | 4        |

---

## Risk Assessment

1. **Performance**: Increasing label density may slow rendering → mitigate with smart LOD
2. **Data Loss**: Lowering maxNodes may frustrate users → add messaging explaining limit
3. **API Changes**: Backend changes need testing → add unit tests

---

## Next Step

Decide phase: Commit to specific implementation plan.
