# Audit: Knowledge Graph Screen

**Component:** `src/components/graph/graph-viewer.tsx`  
**Route:** `/graph`  
**Screenshot:** `e2e/screenshots/02-graph.png`

---

## Current Layout (ASCII)

```
┌─ Graph Page ─────────────────────────────────────────────────────────────────┐
│  ENTITIES 200  [←]   ‖  Knowledge Graph  [🔍 Search] [⚏][↺][⬇][⚙][⊟][⚆]  ‖  DETAILS & FILTERS  [→]  │
│                       ‖  [+] [−] [↻] [⛶] [⛆] [⛦]                           ‖                          │
│  🔍 Search entities   ‖                                                       ‖  Click on a node         │
│  Sort: Name | Degree  ‖   [minimap: 130×80]                                   ‖  to view details         │
│  ─────────────────────‖                                                       ‖                          │
│  [ Grouped | List ]   ‖      ● TECHNOLOGY  ● ORGANIZATION                    ‖                          │
│  ─────────────────────‖      ● CONCEPT     ● ...                              ‖                          │
│  ● TECHNOLOGY         ‖                                                       ‖                          │
│  ● ORGANIZATION       ‖     [dense node cloud: 200 nodes, all-caps labels]   ‖                          │
│  ● CONCEPT            ‖                                                       ‖                          │
│  A              CONC 46‖                                                       ‖                          │
│  AB_CARVAL_AVI  CONC 43‖                                                       ‖                          │
│  ...                   ‖                                                       ‖                          │
│  16 types · 4305 conn  ‖                                                       ‖                          │
└────────────────────────────────────────────────────────────────────────────────┘
```

---

## Findings

### F-GR-01 · Entity labels are ALL_CAPS_WITH_UNDERSCORES — unreadable · CRITICAL
**Problem:** Node labels render as `AB_CARVAL_AVIATION_LEASING_FU`, `MARKET_SURVEILLANCE_AUTH` etc. These are database identifiers exposed directly to the UI. A knowledge graph for humans should show human-readable labels.  
**Code ref:** Graph data from backend stores entity names in normalized form. The frontend renders them as-is.  
**Fix:** Add a `formatEntityLabel()` utility that:
- Converts `_` to spaces
- Applies Title Case
- Truncates at ~30 chars with ellipsis for canvas labels  
```
AB_CARVAL_AVIATION → "Ab Carval Aviation..."
MARKET_SURVEILLANCE_AUTH → "Market Surveillance Auth"
```
**Reference:** [Graph visualization best practices — IBM Design](https://www.ibm.com/design/language/data-visualization/basics/)

### F-GR-02 · Graph toolbar has 12+ icon buttons with no visual grouping · HIGH
**Problem:** The toolbar across the top has 12 icon buttons in a row with no grouping, no dividers, inconsistent icon sizes. Users cannot scan the control surface.  
**Principle:** Hick's Law — more options = slower decision time.  
**Fix:**  
```
Current:  [⚏][↺][⬇][⚙][⊟][⚆][⊕][⊖][↺][⛶]

Target:
  ┌─ View ──┐  ┌─ Actions ──┐  ┌─ Filter ─┐
  [⚏] [⊟]     [⬇] [↺]        [⚙] [⚆]
  (grouping using subtle separators)
```

### F-GR-03 · Minimap takes prime canvas real estate — always visible · MED
**Problem:** The 130×80px minimap in the upper-left of the canvas is always visible even when not needed (small graphs). It obscures part of the visualization.  
**Fix:** Collapse to a small toggle button by default. Show only when canvas has >100 nodes or user explicitly toggles.

### F-GR-04 · Left panel entity list truncates names mid-word · MED
**Problem:** `AB_CARVAL_AVIATION_LEASING_FU` — name truncated at column boundary. Combined with the underscore-formatted names, users can't identify nodes.  
**Fix:** Apply `formatEntityLabel()` to the list view. Show tooltip on hover with full name.

### F-GR-05 · Right panel "DETAILS & FILTERS" header in small-caps looks inconsistent · LOW
**Problem:** The right panel header uses a different typographic treatment from the rest of the app.  
**Code ref:** `graph-viewer.tsx` right panel header.

### F-GR-06 · "Click on a node to view details" placeholder is low-signal · LOW
**Problem:** The right panel shows a generic icon + text in empty state. No guidance on what details are shown, no suggestion for first action.  
**Fix:** Replace with: "Select a node to explore its connections, relationships, and source documents."

### F-GR-07 · No way to reset the graph view quickly · MED
**Problem:** If the user zooms deep into the graph, there's no obvious "reset view" / "fit to screen" button. The zoom controls exist but fit-to-canvas is buried.

### F-GR-08 · Color contrast of node labels on light grey canvas · HIGH
**Problem:** Dark text labels on the pale grey node canvas background. TECHNOLOGY nodes (green) carry labels in dark grey — contrast ratio for small text may fall below 4.5:1.  
**Reference:** WCAG 1.4.3 Contrast (minimum).

### F-GR-09 · "16 types · 4305 connections" footer info buried · LOW
**Problem:** The summary is shown in tiny text at the bottom of the left panel. This is valuable context that should be in a more prominent position.

### F-GR-10 · No progressive loading feedback for large graphs · MED
**Problem:** When the graph has 42k+ entities but only 200 are loaded at once, there's no indication of the total scope / loading progress per batch.

---

## Proposed Layout (ASCII)

```
┌─ Graph Page ──────────────────────────────────────────────────────────────────────────┐
│  ┌─ Entities (200/42k) ─┐  ┌─ Canvas ──────────────────────────────┐  ┌─ Details ──┐ │
│  │  🔍 filter entities  │  │  ← K. Graph  [↺ Reset] ⚙ Settings ▾  │  │ Node name  │ │
│  │  Sort: Name ▾        │  │  ┌─ Toolbar ─────────────────────────┐ │  │ Type badge │ │
│  │  ─────────────────── │  │  │ [zoom−][fit][zoom+] │ [export] [↺]│ │  │            │ │
│  │  Type filter chips   │  │  └────────────────────────────────────┘ │  │ Properties │ │
│  │                      │  │                                          │  │            │ │
│  │  Technology (green)  │  │    [ readable labels, cleaner graph ]    │  │ Relations  │ │
│  │  Organization (blue) │  │                                          │  │            │ │
│  │  Concept (pink)      │  │                          [minimap ▾]     │  │ Documents  │ │
│  │                      │  │                                          │  │            │ │
│  │  Entity list rows    │  │                                          │  │            │ │
│  │  (human labels)      │  │                                          │  │            │ │
│  └──────────────────────┘  └──────────────────────────────────────────┘  └────────────┘ │
└───────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## Summary Score

| Dimension             | Score | Notes                      |
| --------------------- | ----- | -------------------------- |
| Readability           | 3/10  | ALL_CAPS labels unreadable |
| Navigation            | 5/10  | Toolbar bloat, no grouping |
| Information Hierarchy | 5/10  | Dense, no hierarchy        |
| Empty State           | 6/10  | Has illustration           |
| Contrast              | 6/10  | Some labels may fail WCAG  |
| Elegance              | 4/10  | Bloated, noisy             |
