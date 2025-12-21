# Cytoscape.js: Interactive Knowledge Graph Visualization

**Version**: 3.30+  
**GitHub**: https://github.com/cytoscape/cytoscape.js (10k+ stars)  
**Purpose**: Interactive graph visualization in web browsers

---

## Overview

Cytoscape.js is the **industry-standard** JavaScript library for graph visualization, with 35+ years of heritage from the Cytoscape desktop application used in bioinformatics.

### Why Cytoscape.js?

- **Performance**: 1000+ nodes smoothly
- **Layouts**: 10+ algorithms (force, hierarchical, etc.)
- **Interactive**: Pan, zoom, drag, select
- **Extensions**: 50+ official plugins
- **Battle-Tested**: Used by NIST, NIH, pharma companies

---

## Installation

```bash
npm install cytoscape cytoscape-cose-bilkent
```

---

## Quick Start

```html
<div id="cy" style="width: 100%; height: 600px;"></div>
```

```typescript
import cytoscape from 'cytoscape';
import coseBilkent from 'cytoscape-cose-bilkent';

cytoscape.use(coseBilkent);

async function renderGraph(trackId: string) {
  // Fetch graph from LightRAG
  const res = await fetch(`/api/graph/${trackId}`);
  const { nodes, edges } = await res.json();

  const cy = cytoscape({
    container: document.getElementById('cy'),
    
    elements: {
      nodes: nodes.map(n => ({
        data: { id: n.id, label: n.name }
      })),
      edges: edges.map(e => ({
        data: { source: e.source, target: e.target, label: e.type }
      }))
    },

    style: [
      {
        selector: 'node',
        style: {
          'background-color': '#3498db',
          'label': 'data(label)',
          'font-size': '12px'
        }
      },
      {
        selector: 'edge',
        style: {
          'width': 2,
          'line-color': '#95a5a6',
          'target-arrow-shape': 'triangle',
          'label': 'data(label)'
        }
      }
    ],

    layout: {
      name: 'cose-bilkent',
      idealEdgeLength: 100,
      animate: true
    }
  });

  // Add click handler
  cy.on('tap', 'node', (evt) => {
    console.log('Clicked:', evt.target.data());
  });
}
```

---

## Backend API

```rust
// LightRAG Rust backend endpoint
#[derive(Serialize)]
struct GraphData {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

async fn get_graph(
    Path(track_id): Path<String>,
) -> Result<Json<GraphData>> {
    let entities = storage.get_entities(&track_id).await?;
    let relationships = storage.get_relationships(&track_id).await?;
    
    Ok(Json(GraphData {
        nodes: entities.into_iter()
            .map(|e| GraphNode { id: e.id, name: e.name })
            .collect(),
        edges: relationships.into_iter()
            .map(|r| GraphEdge {
                source: r.source_id,
                target: r.target_id,
                type: r.relationship_type
            })
            .collect(),
    }))
}
```

---

## Resources

- [Cytoscape.js Docs](https://js.cytoscape.org/)
- [GitHub](https://github.com/cytoscape/cytoscape.js)

---

**Status**: ✅ Production Ready
