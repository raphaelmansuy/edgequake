# Page: Graph

## Overview

- **Route**: `/graph`
- **Title**: "Knowledge Graph"
- **Layout**: Three-panel layout: Entity Browser (left), Graph Canvas (center), Details/Filters (right)
- **Source File**: [src/app/(dashboard)/graph/page.tsx](../../edgequake_webui/src/app/(dashboard)/graph/page.tsx)

## Layout Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│ ┌───────────────┬─────────────────────────────────────────────────┐ │
│ │               │ Header (64px)                                   │ │
│ │               ├─────────────────────────────────────────────────┤ │
│ │   Sidebar     │ Breadcrumb: EdgeQuake > Knowledge Graph         │ │
│ │   (64px)      ├───────────────────────────────────┬─────────────┤ │
│ │               │ Entity    │ Graph Canvas          │ Details &   │ │
│ │               │ Browser   │                       │ Filters     │ │
│ │               │ (280px)   │ ┌───────────────────┐ │ (280px)     │ │
│ │               │           │ │ Search + Toolbar  │ │             │ │
│ │               │ - Search  │ └───────────────────┘ │ - Selected  │ │
│ │               │ - Sort    │                       │   node info │ │
│ │               │ - View    │ ┌───────────────────┐ │ - Filters   │ │
│ │               │   mode    │ │                   │ │ - Legend    │ │
│ │               │           │ │ Sigma.js Canvas   │ │             │ │
│ │               │ - Entity  │ │ (Graph Render)    │ │             │ │
│ │               │   list    │ │                   │ │             │ │
│ │               │           │ └───────────────────┘ │             │ │
│ │               │           │                       │             │ │
│ │               │ - Stats   │ ┌─────┐ ┌──────────┐ │             │ │
│ │               │   footer  │ │Zoom │ │ Controls │ │             │ │
│ │               │           │ └─────┘ └──────────┘ │             │ │
│ └───────────────┴───────────┴───────────────────────┴─────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

## Screenshots

| Viewport | Screenshot |
|----------|------------|
| Desktop (1440px) | [graph-desktop.png](../screenshots/graph/graph-desktop.png) |
| Tablet (768px) | [graph-tablet.png](../screenshots/graph/graph-tablet.png) |
| Mobile (375px) | [graph-mobile.png](../screenshots/graph/graph-mobile.png) |

---

## Region: Entity Browser Panel (Left)

- **Position**: Left side
- **Dimensions**: 280px width, collapsible
- **Background**: `var(--card)`
- **Border**: 1px solid border on right
- **Source File**: [src/components/graph/entity-browser-panel.tsx](../../edgequake_webui/src/components/graph/entity-browser-panel.tsx)

### Container: Panel Header

- **Layout**: Flex row with icon, title, count, collapse button
- **Content**:
  - Network icon (16px)
  - H2: "Entities" (16px, semibold)
  - Count badge
  - Collapse button (ChevronLeft icon)

### Component: Entity Search

- **Type**: Input with search icon
- **Placeholder**: "Search entities..."
- **Height**: 36px

### Container: Sort Controls

- **Layout**: Flex row
- **Label**: "Sort:"
- **Options**: Name, Degree
- **Sort Direction Toggle**: Arrow icon button

### Container: View Mode Tabs

- **Type**: Segmented button group
- **Options**: "Grouped" (by type), "List" (flat list)

### Container: Entity List

- **Type**: ScrollArea with entity items
- **Empty State**: Network icon + "No entities yet" text

#### Component: Entity Item

- **Type**: Clickable list item
- **Content**: Type badge + entity label
- **States**: Default, hover (bg-muted), selected (bg-accent)

### Container: Stats Footer

- **Position**: Bottom of panel
- **Content**: 
  - "N types" label
  - Link icon + "N connections" count

---

## Region: Graph Canvas (Center)

- **Position**: Center, flexible width
- **Type**: Sigma.js WebGL canvas
- **Background**: `var(--background)`
- **Source File**: [src/components/graph/graph-renderer.tsx](../../edgequake_webui/src/components/graph/graph-renderer.tsx)

### Container: Canvas Toolbar

- **Position**: Top of canvas
- **Layout**: Flex row, space-between
- **Background**: `var(--card)` with blur

#### Component: Graph Title

- **Type**: H2 heading
- **Content**: "Knowledge Graph"

#### Component: Search Command

- **Type**: Button with keyboard shortcut
- **Icon**: Search icon (left)
- **Text**: "Rechercher des nœuds..."
- **Badge**: "⌘K" keyboard shortcut
- **Source File**: [src/components/graph/graph-search.tsx](../../edgequake_webui/src/components/graph/graph-search.tsx)

#### Component: Layout Control

- **Type**: Dropdown button
- **Icon**: Grid icon
- **Function**: Change graph layout algorithm
- **Source File**: [src/components/graph/layout-control.tsx](../../edgequake_webui/src/components/graph/layout-control.tsx)

#### Component: Export Button

- **Type**: Button with dropdown
- **Icon**: Download icon
- **Text**: "Exporter le graphe"
- **Options**: PNG, SVG, JSON
- **Source File**: [src/components/graph/graph-export.tsx](../../edgequake_webui/src/components/graph/graph-export.tsx)

#### Component: Toolbar Actions

- **Type**: Icon button group
- **Buttons**: Refresh, Zoom In, Zoom Out, Reset View, Fullscreen

### Container: Empty State

- **Type**: Centered content block
- **Visibility**: Shown when no graph data
- **Content**:
  - Large Network icon (48px)
  - H3: "No knowledge graph yet"
  - Description text
  - "Upload Documents" primary button

### Container: Zoom Controls (Floating)

- **Position**: Absolute, right side of canvas
- **Type**: Vertical button stack
- **Source File**: [src/components/graph/zoom-controls.tsx](../../edgequake_webui/src/components/graph/zoom-controls.tsx)
- **Buttons**:
  - Zoom In (ZoomIn icon)
  - Zoom Out (ZoomOut icon)
  - Rotate CW
  - Rotate CCW
  - Reset View
  - Fullscreen

### Container: Graph Controls (Floating)

- **Position**: Absolute, bottom-left of canvas
- **Type**: Toolbar with icon buttons
- **Source File**: [src/components/graph/graph-controls.tsx](../../edgequake_webui/src/components/graph/graph-controls.tsx)

### Container: Minimap

- **Position**: Absolute, bottom-left corner
- **Type**: Small canvas showing full graph overview
- **Border**: 1px solid border
- **Background**: Semi-transparent

---

## Region: Details & Filters Panel (Right)

- **Position**: Right side
- **Dimensions**: 280px width, collapsible
- **Background**: `var(--card)`
- **Border**: 1px solid border on left

### Container: Panel Header

- **Layout**: Flex row with title and collapse button
- **Content**:
  - H3: "Details & Filters" (14px, semibold)
  - Collapse button (ChevronRight icon)

### Container: Empty State

- **Visibility**: When no node selected
- **Content**: Network icon + "Click on a node to view details"

### Container: Node Details

- **Visibility**: When node selected
- **Source File**: [src/components/graph/node-details.tsx](../../edgequake_webui/src/components/graph/node-details.tsx)
- **Sections**:
  - Node type badge
  - Node label (heading)
  - Description
  - Metadata properties
  - Connected nodes list
  - Action buttons

### Container: Graph Filters

- **Position**: Below node details or as primary content
- **Source File**: [src/components/graph/graph-filters.tsx](../../edgequake_webui/src/components/graph/graph-filters.tsx)
- **Content**:
  - Entity type checkboxes
  - Relationship type checkboxes
  - Clear filters button

### Container: Graph Legend

- **Position**: Bottom of panel
- **Source File**: [src/components/graph/graph-legend.tsx](../../edgequake_webui/src/components/graph/graph-legend.tsx)
- **Content**: Color-coded entity type legend

---

## Component: Node Context Menu

- **Type**: Right-click context menu
- **Source File**: [src/components/graph/node-context-menu.tsx](../../edgequake_webui/src/components/graph/node-context-menu.tsx)
- **Options**:
  - View Details
  - Expand Neighborhood
  - Find Related
  - Query about this entity
  - Copy Label

---

## Responsive Behavior

| Breakpoint | Entity Browser | Graph Canvas | Details Panel |
|------------|----------------|--------------|---------------|
| Mobile (<768px) | Collapsed | Full width | Collapsed |
| Tablet (768-1024px) | Visible (narrower) | Flexible | Collapsed |
| Desktop (>1024px) | Full 280px | Flexible | Full 280px |

---

## Graph Rendering Technology

- **Library**: Sigma.js v3 with React wrapper (@react-sigma)
- **Rendering**: WebGL for performance
- **Layout Algorithms**: ForceAtlas2, Circular, Random
- **Graphology**: Graph data structure library
- **Features**:
  - Node hover highlighting
  - Edge curve rendering
  - Community detection (Louvain)
  - Zoom/pan controls
  - Fullscreen mode

---

## Component Cross-References

- [Button](../components/buttons.md) — Toolbar buttons, zoom controls
- [Input](../components/inputs.md) — Search inputs
- [Card](../components/cards.md) — Panel containers
- [ScrollArea](../components/navigation.md) — Entity list scrolling
- [Badge](../components/buttons.md) — Entity type badges
- [Tooltip](../components/dialogs.md) — Button tooltips
- [Context Menu](../components/dialogs.md) — Node right-click menu
