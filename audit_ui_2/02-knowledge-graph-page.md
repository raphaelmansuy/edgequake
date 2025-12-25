# UI Audit: Knowledge Graph Page (Full View)

**Screen:** Knowledge Graph - Desktop View  
**Date:** 2025-12-25  
**Priority:** High - Primary application feature

---

## Screenshot Analysis

Full desktop view showing:

- Left sidebar with navigation
- Entity browser panel (collapsible)
- Main graph canvas with node visualization
- Right details panel for selected node
- Floating legend component
- Toolbar with graph controls

---

## Issues Identified

### Critical Issues

| ID    | Issue                                                                                                                                       | Location    | Severity    |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------- | ----------- | ----------- |
| KG-01 | **Graph canvas underutilized** - Single node "Nemotron 3" displayed with excessive empty space, graph lines extend to edges without context | Main canvas | 🔴 Critical |
| KG-02 | **Entity browser has no visual hierarchy** - All entities look the same regardless of connection count or importance                        | Left panel  | 🔴 Critical |

### High Priority Issues

| ID    | Issue                                                                                                   | Location           | Severity |
| ----- | ------------------------------------------------------------------------------------------------------- | ------------------ | -------- |
| KG-03 | **Duplicate toolbars** - Zoom/refresh controls appear in BOTH top toolbar AND floating left toolbar     | Header + Floating  | 🟠 High  |
| KG-04 | **Legend overlaps potential graph content** - Fixed position legend at bottom could cover nodes         | Legend box         | 🟠 High  |
| KG-05 | **Selected entity highlight too subtle** - "Nemotron 3" in list has light teal background, easy to miss | Entity browser     | 🟠 High  |
| KG-06 | **No visual connection lines visible** - Graph shows node but edge lines are faint/invisible            | Canvas             | 🟠 High  |
| KG-07 | **Search bar placement inconsistent** - Two search bars: one in entity panel, one in top toolbar        | Multiple locations | 🟠 High  |

### Medium Priority Issues

| ID    | Issue                                                                                                              | Location              | Severity  |
| ----- | ------------------------------------------------------------------------------------------------------------------ | --------------------- | --------- |
| KG-08 | **Entity count shows "7" but only visible list shows fewer** - Panel header says "Entities 7" but scrolling needed | Entity browser header | 🟡 Medium |
| KG-09 | **Sort controls small and cramped** - "Sort: Name Degree" with toggle buttons too compact                          | Entity browser        | 🟡 Medium |
| KG-10 | **Footer info cramped** - "3 types 6 connections" at bottom of entity panel has low visibility                     | Entity browser footer | 🟡 Medium |
| KG-11 | **Legend "eye" toggle icons unclear** - Eye icons next to entity types not obviously toggles                       | Legend                | 🟡 Medium |
| KG-12 | **Breadcrumb navigation** - "EdgeQuake > Knowledge Graph" uses text that could be more compact                     | Header breadcrumb     | 🟡 Medium |

### Low Priority Issues

| ID    | Issue                                                                                 | Location         | Severity |
| ----- | ------------------------------------------------------------------------------------- | ---------------- | -------- |
| KG-13 | **"Grouped/List" toggle styling** - Segmented control could have clearer active state | Entity browser   | 🟢 Low   |
| KG-14 | **Version display "EdgeQuake v0.1.0"** - Takes space in sidebar footer                | Sidebar          | 🟢 Low   |
| KG-15 | **Floating toolbar icons small** - Zoom +/- and other controls could be larger        | Floating toolbar | 🟢 Low   |

---

## Improvement Plan

### Phase 1: Graph Canvas Enhancement (Week 1)

#### 1.1 Force-Directed Layout Improvements

```
Current:  Single node centered, lines extend to void
Improved:
┌─────────────────────────────────────────────────┐
│                                                 │
│           ○ NVIDIA                              │
│            \                                    │
│    ○ Mamba-2 ── ● Nemotron 3 ── ○ Qwen3-30B   │
│            /        |                           │
│   ○ MoE          ○ NVFP4                       │
│                                                 │
└─────────────────────────────────────────────────┘
```

**Implementation:**

- Show connected nodes by default when selecting an entity
- Animate edges with gradients indicating direction
- Add edge labels on hover
- Implement "expand neighbors" button on node selection

#### 1.2 Edge Visibility Enhancement

```css
.graph-edge {
  stroke: var(--muted-foreground);
  stroke-width: 2px;
  stroke-opacity: 0.6;
}

.graph-edge:hover,
.graph-edge.connected-to-selected {
  stroke: var(--primary);
  stroke-width: 3px;
  stroke-opacity: 1;
}
```

### Phase 2: Consolidate Controls (Week 1)

#### 2.1 Remove Duplicate Toolbars

```
Current Layout:
┌─ Top Bar ─────────────────────────────────────┐
│ [Refresh][Zoom+][Zoom-][Fullscreen]           │
└───────────────────────────────────────────────┘
     +
┌─ Floating ─┐
│ [Zoom+]    │  ← DUPLICATE
│ [Zoom-]    │  ← DUPLICATE
│ [Rotate]   │
│ [Recenter] │
│ [Expand]   │
│ [Fullscr]  │  ← DUPLICATE
└────────────┘

Proposed Layout:
┌─ Top Bar ─────────────────────────────────────┐
│ Knowledge Graph  │ [Search...]  [Layout][Export]│
└───────────────────────────────────────────────┘
     +
┌─ Floating ─┐
│ [Zoom+]    │
│ [Zoom-]    │
│ [Recenter] │
│ ────────── │
│ [Rotate]   │
│ [Expand]   │
└────────────┘
```

**Keep in floating toolbar:** Zoom, pan, layout controls
**Keep in top bar:** Search, export, layout algorithm selector

### Phase 3: Entity Browser Improvements (Week 2)

#### 3.1 Visual Hierarchy with Connection Strength

```
Current:
● Nemotron 3
  PRODUCT · 6 connections

Improved:
┌────────────────────────────────────────┐
│ ● Nemotron 3                      ★ 6 │
│   PRODUCT                     ████████│
├────────────────────────────────────────┤
│ ● Attention layers                  1 │
│   TECHNOLOGY                  ██      │
└────────────────────────────────────────┘
```

**Features:**

- Connection count as visual bar
- Star/highlight for most connected
- Compact view option

#### 3.2 Selected State Enhancement

```css
.entity-item.selected {
  background: var(--primary);
  color: var(--primary-foreground);
  border-left: 4px solid var(--primary);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}
```

### Phase 4: Legend Improvements (Week 2)

#### 4.1 Repositionable Legend

```
Options:
┌─ Position selector ─┐
│ ◉ Bottom-right      │
│ ○ Bottom-left       │
│ ○ Top-right         │
│ ○ Collapsed         │
└─────────────────────┘
```

**Features:**

- Draggable legend position
- Collapse to icon-only mode
- Toggle visibility per entity type
- Clear visual checkboxes instead of eye icons

#### 4.2 Legend Interaction

```
┌─ Legend ─────────────────────────────────┐
│ [✓] ● Technology (4)                     │
│ [✓] ● Product (2)                        │
│ [ ] ● Organization (1)  ← dimmed         │
└──────────────────────────────────────────┘
```

### Phase 5: Unified Search (Week 3)

#### 5.1 Single Global Search

```
┌─ Search ────────────────────────────────────────┐
│ 🔍 Search nodes...                      ⌘K      │
├─────────────────────────────────────────────────┤
│ ENTITIES                                        │
│ ● Nemotron 3 (PRODUCT)                         │
│ ● NVIDIA (ORGANIZATION)                        │
│                                                 │
│ TYPES                                           │
│ ● Technology (4 entities)                      │
│ ● Product (2 entities)                         │
└─────────────────────────────────────────────────┘
```

**Remove:** Search box in entity panel header
**Enhance:** Top toolbar search with ⌘K shortcut, dropdown results

---

## Layout Optimization

### Current 3-Panel Layout Issues

```
┌────────┬────────────────────────────┬────────┐
│Sidebar │ Entity    │    Graph       │Details │
│  nav   │ Browser   │    Canvas      │ Panel  │
│        │  (280px)  │   (flexible)   │(280px) │
└────────┴───────────┴────────────────┴────────┘

Problem: 4 columns = cramped graph canvas
```

### Proposed Collapsible Panels

```
Default (entity selected):
┌────┬────────────────────────────────┬────────┐
│Nav │        Graph Canvas            │Details │
│    │                                │ Panel  │
└────┴────────────────────────────────┴────────┘
     [◀ Entities] floating button to expand

Entity Browser Expanded:
┌────┬────────┬────────────────────────────────┐
│Nav │Entities│        Graph Canvas            │
│    │ Panel  │                                │
└────┴────────┴────────────────────────────────┘
           Details panel collapsed →
```

---

## Accessibility Improvements

1. **Keyboard Navigation:**

   - Arrow keys to navigate entity list
   - Enter to select and focus graph node
   - Tab to move between panels
   - Escape to deselect

2. **Screen Reader:**

   - "Graph contains 7 entities with 6 connections"
   - "Selected: Nemotron 3, Product type, 6 connections"
   - Live region for selection changes

3. **Reduced Motion:**
   - Disable graph animations if `prefers-reduced-motion`
   - Static layout option

---

## Success Metrics

| Metric               | Current             | Target             |
| -------------------- | ------------------- | ------------------ |
| Graph utilization    | ~10% canvas used    | 60%+ visible nodes |
| Control duplication  | 4 duplicate buttons | 0 duplicates       |
| Selection visibility | Subtle teal         | High contrast      |
| Panel flexibility    | Fixed widths        | Collapsible        |
