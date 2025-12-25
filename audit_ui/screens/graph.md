# Graph Screen Audit

**Route:** `/graph`  
**Viewport(s) Tested:** 320px, 428px, 768px, 1280px, 1536px  
**UI Regions:** Header, Sidebar, Breadcrumb, Entity Browser, Graph Canvas, Controls, Legend  
**States Captured:** Empty State, Default  
**Screenshots:** `screenshots/screens/graph/`  
**Relevant Files:** `src/app/(dashboard)/graph/page.tsx`, `src/components/graph/`

---

## What I Reviewed

### Layout Structure

```
┌─────────────────────────────────────────────────────────┐
│ Header (fixed, h: 64px)                    │ API │ 🌐 ☀️ 👤│
├────────────┬────────────────────────────────────────────┤
│ Sidebar    │ Breadcrumb: EdgeQuake > Knowledge Graph    │
│ w: 256px   ├──────────────┬─────────────────────────────┤
│            │ 🔗 Entities 0│ Knowledge Graph    🔍 ⊞ ↓ ⟳ │
│            │ ◁            │                    ⊕ ⊖ ↻    │
│            ├──────────────┤ ┌─────────────────────┐     │
│            │ 🔍 Search... │ │                     │ [⊕] │
│            │ Sort: Name   │ │                     │ [⊖] │
│            │ [Grouped]List│ │      🔗             │ [↻] │
│            ├──────────────┤ │  No knowledge       │ [↷] │
│            │              │ │  graph yet          │ [↶] │
│            │ 🔗 No        │ │                     │ [↻] │
│            │ entities yet │ │  [Upload Documents] │ [⛶] │
│            │              │ │                     │     │
│            │              │ └─────────────────────┘     │
│            ├──────────────┤                             │
│            │ 0 types      │                        [⚙]  │
│            │ 🔗 0 connect │                             │
└────────────┴──────────────┴─────────────────────────────┘
```

---

## Slickness Score

| Criterion           | Score (1–5) | Notes                                         |
| ------------------- | ----------- | --------------------------------------------- |
| Visual refinement   | 3.8         | Good layout, empty state could be better      |
| Modern styling      | 4.0         | Control toolbar is clean                      |
| Smooth interactions | 3.5         | Need to test with actual graph data           |
| Professional polish | 3.8         | Empty state is clear but plain                |
| **Overall**         | **3.8**     | Good foundation, needs data to fully evaluate |

---

## Issues

### 🟠 Major

#### Entity Browser Panel Collapse Animation

- **Severity:** 🟠 Major
- **Location:** Left entity browser panel
- **Viewport(s) affected:** Desktop
- **Current behavior:** Has collapse button but animation not tested
- **Expected behavior:** Smooth 250ms collapse with content fade

#### Graph Controls Not Visible on Empty State

- **Severity:** 🟠 Major
- **Location:** Graph toolbar
- **Viewport(s) affected:** All
- **Current behavior:** All controls shown but most disabled
- **Expected behavior:** Consider hiding non-functional controls or showing enabled state

#### Duplicate Control Sets

- **Severity:** 🟠 Major
- **Location:** Top toolbar + floating controls on right
- **Viewport(s) affected:** Desktop
- **Current behavior:** Zoom controls appear in both locations
- **Expected behavior:** Single location for controls, or clearly differentiate purposes

---

### 🟡 Minor

#### Empty State Could Be More Engaging

- **Severity:** 🟡 Minor
- **Location:** Graph canvas area
- **Viewport(s) affected:** All
- **Current behavior:** Plain icon + text + button
- **Expected behavior:** Add illustration or animation suggesting graph visualization

#### Entity Search Has No Results Indicator

- **Severity:** 🟡 Minor
- **Location:** Entity browser search
- **Viewport(s) affected:** Desktop
- **Current behavior:** Shows "No entities yet" - same as empty state
- **Expected behavior:** Differentiate between "no entities" and "no search results"

#### Grouped/List Toggle Could Be More Obvious

- **Severity:** 🟡 Minor
- **Location:** Entity browser view toggle
- **Viewport(s) affected:** Desktop
- **Current behavior:** Two text buttons
- **Expected behavior:** Icon buttons with active state highlight

#### Footer Stats Could Be More Prominent

- **Severity:** 🟡 Minor
- **Location:** Entity browser footer
- **Viewport(s) affected:** Desktop
- **Current behavior:** "0 types" and "0 connections" small text
- **Expected behavior:** More visible stats with icons

---

## Recommendations

### 1. Enhance Empty State with Illustration

**Change:** Add engaging empty state illustration

**Specifications:**

```tsx
<div className="flex flex-col items-center justify-center h-full text-center p-8">
  {/* Animated placeholder showing graph concept */}
  <div className="relative w-48 h-48 mb-6">
    <div className="absolute inset-0 flex items-center justify-center">
      {/* Animated dotted lines connecting placeholder nodes */}
      <svg className="w-full h-full opacity-30">
        <circle cx="50%" cy="30%" r="20" className="fill-primary/20" />
        <circle cx="25%" cy="70%" r="15" className="fill-muted" />
        <circle cx="75%" cy="70%" r="15" className="fill-muted" />
        <line
          x1="50%"
          y1="30%"
          x2="25%"
          y2="70%"
          className="stroke-muted-foreground/30 stroke-2 stroke-dasharray-4"
        />
        <line
          x1="50%"
          y1="30%"
          x2="75%"
          y2="70%"
          className="stroke-muted-foreground/30 stroke-2 stroke-dasharray-4"
        />
      </svg>
    </div>
    <Network className="h-12 w-12 text-muted-foreground mx-auto mt-16" />
  </div>
  <h3>No knowledge graph yet</h3>
  <p>Upload documents to automatically extract entities and relationships.</p>
  <Button>Upload Documents</Button>
</div>
```

**Acceptance Criteria:**

- [ ] Illustration suggests graph structure
- [ ] Subtle animation (optional)
- [ ] Clear CTA

---

### 2. Consolidate Control Locations

**Change:** Remove duplicate controls, use single toolbar

**Specifications:**

- **Option A:** Keep only top toolbar, remove floating controls
- **Option B:** Keep floating controls on right, remove from top toolbar
- **Recommended:** Option B - floating controls are more accessible during graph interaction

**Top toolbar should contain:**

- Search
- Layout selector
- Export button
- Refresh button

**Floating controls should contain:**

- Zoom in/out
- Rotate
- Reset view
- Fullscreen

**Acceptance Criteria:**

- [ ] No duplicate controls
- [ ] Each control in single location
- [ ] Clear visual grouping

---

### 3. Improve View Toggle

**Change:** Use icon-based toggle with clear active state

**Specifications:**

```tsx
<div className="inline-flex rounded-md border overflow-hidden">
  <button
    className={cn(
      "p-2",
      isGrouped ? "bg-primary text-primary-foreground" : "bg-background"
    )}
    aria-pressed={isGrouped}
  >
    <LayoutGrid className="h-4 w-4" />
  </button>
  <button
    className={cn(
      "p-2",
      !isGrouped ? "bg-primary text-primary-foreground" : "bg-background"
    )}
    aria-pressed={!isGrouped}
  >
    <List className="h-4 w-4" />
  </button>
</div>
```

**Acceptance Criteria:**

- [ ] Icons clearly indicate view type
- [ ] Active state is obvious
- [ ] Meets touch target size

---

### 4. Add Loading State for Graph

**Change:** Show skeleton/placeholder while graph loads

**Specifications:**

```tsx
{isLoading ? (
  <div className="flex items-center justify-center h-full">
    <div className="text-center">
      <Loader2 className="h-8 w-8 animate-spin text-primary mx-auto mb-4" />
      <p className="text-sm text-muted-foreground">Loading graph...</p>
    </div>
  </div>
) : (
  // ... graph content
)}
```

**Acceptance Criteria:**

- [ ] Loading spinner while fetching
- [ ] Message indicates loading
- [ ] Smooth transition to graph

---

## Measurements

| Element                | Current          | Recommended                           |
| ---------------------- | ---------------- | ------------------------------------- |
| Entity browser width   | ~240px           | Consider 280px for better readability |
| Graph canvas           | Flex (remaining) | ✅ Good                               |
| Control button size    | ~32px            | Increase to 40px for touch            |
| Floating toolbar width | ~40px            | ✅ Good                               |
| Search input height    | 40px             | ✅ Good                               |

---

## Responsive Behavior

### Mobile (320-428px)

- ⚠️ Entity browser should be hidden/sheet-based
- ✅ Graph should fill screen
- ⚠️ Controls should be minimal or in bottom sheet
- ⚠️ Search should be modal

### Tablet (768px)

- ⚠️ Entity browser could be collapsible
- ✅ Graph has enough space
- ⚠️ Controls could be consolidated

### Desktop (1280px+)

- ✅ Three-panel layout works
- ⚠️ Consider right panel for entity details on selection
- ✅ All controls visible

---

## Accessibility

| Check               | Status        | Notes                             |
| ------------------- | ------------- | --------------------------------- |
| Canvas keyboard nav | ⚠️ Needs work | Add keyboard controls for graph   |
| Control buttons     | ✅ Good       | Have aria-labels                  |
| Entity list         | ✅ Good       | Standard list navigation          |
| Zoom controls       | ⚠️ Consider   | Add keyboard shortcuts            |
| Graph ARIA          | ⚠️ Needs work | Consider aria-live for node focus |

### Recommended Keyboard Shortcuts

| Shortcut     | Action         |
| ------------ | -------------- |
| `+` / `=`    | Zoom in        |
| `-`          | Zoom out       |
| `0`          | Reset view     |
| `Arrow keys` | Pan graph      |
| `Tab`        | Navigate nodes |
| `Enter`      | Select node    |
| `Escape`     | Deselect       |

---

## Graph Visualization Requirements

When data is present, verify:

- [ ] Node colors differentiate entity types
- [ ] Edge labels are readable
- [ ] Hover shows node details
- [ ] Click selects and highlights connections
- [ ] Zoom maintains label readability
- [ ] Large graphs don't lag
- [ ] Node clustering for dense areas

---

## Screenshots Reference

| State  | Breakpoint       | File                     |
| ------ | ---------------- | ------------------------ |
| Empty  | Desktop 1280px   | `04-graph-desktop.png`   |
| Empty  | Desktop L 1536px | `04-graph-desktop-l.png` |
| Empty  | Tablet 768px     | `04-graph-tablet.png`    |
| Empty  | Mobile L 428px   | `04-graph-mobile-l.png`  |
| Canvas | Desktop          | `04-graph-canvas.png`    |
| Search | Desktop          | `04-graph-search.png`    |

---

_Last updated: December 25, 2025_
