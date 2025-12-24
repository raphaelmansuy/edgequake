# UX/UI Improvement: Knowledge Graph Page

## Current State Analysis

### Page Structure

- **Header**: Title with search, layout, zoom, and reset controls
- **Graph Canvas**: Force-directed graph visualization
- **Legend Panel**: Collapsible panel showing node types with counts
- **Control Toolbar**: Floating toolbar with zoom, rotate, reset, fullscreen

### Positive Observations

- Clear node coloring by type (Person=blue, Organization=green, Project=gray)
- Node labels visible and readable
- Interactive zoom and pan
- Legend shows entity counts
- Fullscreen mode available

---

## UX Issues Identified

### Critical

1. **"No knowledge graph yet" State (Observed in User Screenshot)**

   - **Issue**: Empty state appears when no entities exist
   - **Status**: Working when documents are processed
   - **Recommendation**:
     - Clear call-to-action: "Upload documents to build your graph"
     - Link directly to Documents page

2. **Graph Not Updating After Upload**
   - **Issue**: User may upload documents but graph doesn't refresh
   - **Impact**: Appears broken when it's just stale
   - **Recommendation**:
     - Auto-refresh graph when navigating to page
     - Add visual indicator when data is stale
     - Show "New data available - Refresh" toast

### High Priority

3. **Node Interaction**

   - **Issue**: Clicking on nodes doesn't show details
   - **Impact**: Users can't explore entity information
   - **Recommendation**:
     - Click to show entity detail panel
     - Show: name, type, connected entities, source documents
     - Option to filter graph by selected node

4. **Edge/Relationship Visibility**

   - **Issue**: Relationship lines are thin and unlabeled by default
   - **Impact**: Users can't understand relationships
   - **Recommendation**:
     - Show edge labels on hover
     - Increase line thickness
     - Use arrows to show direction

5. **Search Functionality**

   - **Issue**: "Search nodes..." placeholder but unclear behavior
   - **Impact**: Users don't know what they're searching
   - **Recommendation**:
     - Add typeahead/autocomplete
     - Highlight matching nodes
     - Show "No results" message

6. **Legend Panel Position**
   - **Issue**: Legend overlaps graph area, may obscure nodes
   - **Impact**: Reduces visible graph area
   - **Recommendation**:
     - Make legend draggable
     - Auto-position to avoid node overlap
     - Remember user's preferred position

### Medium Priority

7. **Layout Options**

   - **Issue**: Only force-directed layout visible
   - **Impact**: Large graphs may be hard to read
   - **Recommendation**:
     - Add hierarchical layout
     - Add radial layout
     - Add circular layout
     - Remember last used layout

8. **Graph Performance**

   - **Issue**: May slow down with many nodes
   - **Impact**: Poor experience with large knowledge bases
   - **Recommendation**:
     - Implement level-of-detail rendering
     - Add clustering for large datasets
     - Show node count warning

9. **Export Graph**

   - **Issue**: No way to export graph visualization
   - **Impact**: Can't share or present findings
   - **Recommendation**:
     - Export as PNG/SVG
     - Export as JSON (nodes + edges)
     - Share link with current view state

10. **Filter by Node Type**
    - **Issue**: Legend shows types but doesn't filter
    - **Impact**: Can't focus on specific entity types
    - **Recommendation**:
      - Click legend item to toggle visibility
      - Add filter sidebar with more options
      - Filter by source document

### Low Priority

11. **Mini-map**

    - **Issue**: No overview map for large graphs
    - **Impact**: Easy to get lost in large graphs
    - **Recommendation**: Add mini-map in corner

12. **Graph Stats**

    - **Issue**: Only node counts in legend
    - **Impact**: Missing graph metrics
    - **Recommendation**:
      - Show total relationships
      - Show graph density
      - Show most connected nodes

13. **Keyboard Navigation**
    - **Issue**: No keyboard controls for graph
    - **Impact**: Accessibility limitation
    - **Recommendation**:
      - Tab through nodes
      - Arrow keys to navigate
      - Space to select

---

## Recommendations

### Short Term (Sprint 1)

- [ ] Add node click to show details
- [ ] Improve empty state with CTA
- [ ] Show edge labels on hover
- [ ] Add auto-refresh on navigation

### Medium Term (Sprint 2)

- [ ] Implement search with autocomplete
- [ ] Add multiple layout algorithms
- [ ] Make legend draggable
- [ ] Add export options

### Long Term

- [ ] Implement clustering for large graphs
- [ ] Add mini-map
- [ ] Add advanced filtering
- [ ] Add graph animation (timeline view)

---

## Wireframe: Node Detail Panel

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  [Graph Canvas with nodes and edges]                        │
│                                                             │
│                    ┌──────────────────────────────────────┐ │
│                    │ 👤 Dr. Sarah Chen                    │ │
│                    │ ──────────────────────               │ │
│                    │ Type: Person                         │ │
│                    │                                      │ │
│                    │ Connections (4):                     │ │
│                    │ • 🏢 Stanford University             │ │
│                    │ • 👤 Dr. Michael Wong                │ │
│                    │ • 📁 GraphRAG                        │ │
│                    │ • 🏢 Microsoft Research              │ │
│                    │                                      │ │
│                    │ Source Documents:                    │ │
│                    │ • knowledge_test.md                  │ │
│                    │                                      │ │
│                    │ [Focus] [Filter] [View Document]     │ │
│                    └──────────────────────────────────────┘ │
│                                                             │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ 🎨 Legend                                    [−]        ││
│  │  ● Person 5  ● Organization 5  ● Project 1             ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

---

## Wireframe: Improved Empty State

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│                                                             │
│                       🕸️                                    │
│                                                             │
│              No Knowledge Graph Yet                         │
│                                                             │
│     Your knowledge graph is empty. Upload documents         │
│     to automatically extract entities and relationships.    │
│                                                             │
│              ┌─────────────────────────┐                   │
│              │  📄 Upload Documents    │                   │
│              └─────────────────────────┘                   │
│                                                             │
│     After uploading, we'll identify:                        │
│     👤 People  🏢 Organizations  📍 Locations  📁 Projects  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Acceptance Criteria

- [ ] Clicking node shows detail panel
- [ ] Edge labels visible on hover
- [ ] Search finds and highlights nodes
- [ ] Empty state has clear CTA
- [ ] Graph auto-refreshes when data changes
- [ ] Multiple layout options available
- [ ] Export graph as image/JSON
- [ ] Legend items toggle node type visibility
