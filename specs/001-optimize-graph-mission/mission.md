# Mission: Optimize Knowledge Graph Display & UX

## Task

Your mission is to optimize the Knowledge Graph visualization for SOTA performance and UX:

1. **Optimize Loading Time** - Study and implement strategies to minimize graph loading time
2. **Fix Expand Neighbors** - Fix the "expand neighbors" function that throws "Entity not found" errors
3. **Enforce Node Limit** - Ensure no more than 500 nodes displayed at once (currently showing 1700+)
4. **Fix Node Labels** - Node labels are not being displayed in the graph visualization
5. **Server-Side Search with Graph Refresh** - Search must query the server and refresh graph centered on selected node

Fully Read specs/001-optimize-graph-mission/mission.md before starting OODA iterations. Re-read at the start of every iteration to prevent alignment drift.

## Context

- **Location**: `/Users/raphaelmansuy/Github/03-working/edgequake`
- **Frontend**: `edgequake_webui/` - Next.js + React + TypeScript + Sigma.js
- **Backend**: `edgequake/crates/` - Rust API server
- **Graph Store**: `use-graph-store.ts` - Zustand state management
- **Graph Renderer**: Sigma.js with WebGL rendering

## Current Issues (from screenshots)

1. Graph loads 1700+ nodes (should be max 500)
2. `Entity 'CRÉANCES_CLIENTS' not found` error on expand
3. Node labels not visible in visualization
4. Search doesn't refresh graph view

## Key Files to Investigate

- `edgequake_webui/src/stores/use-graph-store.ts` - maxNodes setting
- `edgequake_webui/src/hooks/use-graph-expansion.ts` - expand neighbors logic
- `edgequake_webui/src/components/graph/graph-renderer.tsx` - label rendering
- `edgequake_webui/src/components/graph/graph-search.tsx` - search functionality
- `edgequake/crates/edgequake-api/src/handlers/graph.rs` - API endpoints

## Success Criteria

- [ ] Graph loads ≤500 nodes initially
- [ ] Expand neighbors works without errors
- [ ] Node labels visible on zoom
- [ ] Search queries server and centers view on result
- [ ] Loading time < 2 seconds for 500 nodes
- [ ] All tests passing

## Process

Execute OODA loops with iterations in `specs/001-optimize-graph-mission/ooda_loop/iteration_XX/`


Ensure DRY and SRP principles in code changes. Add comments explaining WHY for all changes.

---

⚠️ **CRITICAL**: Re-read this mission file at the start of EVERY OODA iteration to prevent alignment drift.
