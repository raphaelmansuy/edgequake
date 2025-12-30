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

---

## December 30, 2025 - Live Testing Update

### New Observations with Real Data

Testing performed with actual document data (794 entities, 576 relations from an AI safety research paper).

#### ✅ Positive Findings

1. **Entity Browser Panel Collapse Works Well**

   - Smooth collapse animation observed
   - Collapsed state shows rotated "Entities (N)" label
   - Panel persists state correctly
   - Width transitions smoothly from 256px to minimal collapsed state (~40px)

2. **Graph Canvas Renders Large Datasets**

   - 794 entities rendered without visible lag
   - Force-directed layout positions nodes correctly
   - Color-coded entity types clearly visible
   - Node labels readable at default zoom

3. **Both Panels Collapsible**

   - Left (Entity Browser) and Right (Details) panels both collapse
   - When both collapsed, graph canvas maximizes correctly
   - Professional vertical text labels on collapsed panels

4. **Search Functionality**
   - Entity search works and filters list correctly
   - Scroll within entity list works smoothly
   - Clear visual hierarchy for entity types

#### 🔴 Critical Issues Found

1. **Mobile Legend Overlay (CRITICAL)**

   - **Viewport:** 320px (Mobile S)
   - **Issue:** The Legend component overlays and partially blocks the graph canvas
   - **Impact:** Users cannot see/interact with graph content behind the legend
   - **Recommendation:**
     - On mobile, move legend to bottom sheet or collapsible drawer
     - Or reduce legend to minimal floating chip that expands on tap
     - Consider hiding legend by default on mobile, show via toggle

2. **Entity Browser Panel NOT SCROLLABLE (P0 - CRITICAL)**

   - **Priority:** P0 - CRITICAL - Discovered December 30, 2025 (second pass)
   - **Location:** Entity Browser (left panel)
   - **Root Cause:** Parent container has `overflow: hidden`
   - **Evidence:**
     - Content height: 2147px
     - Visible height: 619px
     - **71% of entities are hidden and inaccessible**
   - **Impact:**
     - Mouse wheel scroll does NOT work
     - Keyboard navigation (arrow keys, Page Up/Down) does NOT work
     - Users cannot access entities beyond visible area (~30 of 100)
   - **Fix Required:**
     ```tsx
     // CURRENT (BUG)
     <aside className="overflow-hidden ...">
     
     // SHOULD BE
     <aside className="overflow-y-auto overflow-x-hidden ...">
     ```

3. **Details & Filters Panel NOT SCROLLABLE (P0 - CRITICAL)**

   - **Priority:** P0 - CRITICAL - Discovered December 30, 2025 (second pass)
   - **Location:** Details & Filters (right panel)
   - **Root Cause:** Container has `overflow: hidden`
   - **Evidence:**
     - Content height: 780px
     - Visible height: 619px
     - **161px hidden (Edit/Merge/Delete buttons may be cut off)**
   - **Impact:**
     - Cannot scroll to see all entity properties
     - Action buttons may be hidden for entities with many relationships
   - **Fix Required:**
     ```tsx
     // CURRENT (BUG)
     <div className="flex flex-col h-full overflow-hidden">
     
     // SHOULD BE
     <div className="flex flex-col h-full">
       <div className="flex-1 overflow-y-auto">
     ```

4. **Fixed Zones Verification**
   - Header: ✅ Fixed at top (64px)
   - Sidebar: ✅ Fixed on left (256px expanded, 64px collapsed)
   - Breadcrumb: ✅ Fixed below header
   - Graph Canvas: ✅ Fills remaining space, has own scroll/pan
   - Entity Browser: ✅ Has internal scroll for entity list

#### Responsive Behavior - Verified

| Breakpoint | Layout                           | Status | Notes                |
| ---------- | -------------------------------- | ------ | -------------------- |
| 320px      | Full-width, hamburger menu       | ⚠️     | Legend overlay issue |
| 768px      | Sidebar visible, panels collapse | ✅     | Works well           |
| 1280px     | Full three-panel layout          | ✅     | Optimal experience   |

### Updated Screenshots

| State                  | Breakpoint     | File (Dec 30)                              |
| ---------------------- | -------------- | ------------------------------------------ |
| With Data              | Desktop 1280px | `graph-desktop-1280-initial.png`           |
| Entity Panel Collapsed | Desktop        | `graph-desktop-entity-panel-collapsed.png` |
| Both Panels Collapsed  | Desktop        | `graph-desktop-both-panels-collapsed.png`  |
| With Data              | Tablet 768px   | `graph-tablet-768.png`                     |
| Legend Overlay Issue   | Mobile 320px   | `graph-mobile-320.png`                     |

---

### Updated Score (Revised after scroll bug discovery)

| Criterion           | Previous | Current  | Change     |
| ------------------- | -------- | -------- | ---------- |
| Visual refinement   | 3.8      | 4.2      | +0.4       |
| Modern styling      | 4.0      | 4.2      | +0.2       |
| Smooth interactions | 3.5      | 4.0      | +0.5       |
| Professional polish | 3.8      | 4.2      | +0.4       |
| **Scroll/Overflow** | **N/A**  | **2.0**  | **NEW**    |
| **Overall**         | **3.8**  | **3.7**  | **-0.1**   |

**Score Revision Note:** Initial testing showed 4.15, but deeper scroll testing revealed 2 P0-critical bugs:
- Entity Browser panel: `overflow: hidden` blocks all scrolling (71% content hidden)
- Details panel: `overflow: hidden` blocks scrolling (action buttons may be cut off)

These bugs significantly impact usability and must be fixed before the page is production-ready.

---

_Last updated: December 30, 2025 (Revised with scroll bug findings)_
