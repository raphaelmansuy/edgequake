# Mission: Optimize Knowledge Graph Display & UX


Fully Read the mission before starting. Re-read it at the start of every OODA iteration to prevent alignment drift.

## Task

Your mission is to optimize the Knowledge Graph visualization for SOTA performance and UX:

1. **Optimize Loading Time** - Study and implement strategies to minimize graph loading time
2. **Fix Expand Neighbors** - Fix the "expand neighbors" function that throws "Entity not found" errors
3. **Enforce Node Limit** - Ensure no more than 500 nodes displayed at once (currently showing 1700+)
4. **Fix Node Labels** - Node labels are not being displayed in the graph visualization
5. **Server-Side Search with Graph Refresh** - Search must query the server and refresh graph centered on selected node


Inspect all the screen and panels of graph visualization to identify potential bottlenecks or misconfigurations. Use profiling tools if necessary to analyze rendering performance.

Ensure the UX / Layout / Scrolling / Zooming interactions are smooth and intuitive. Ensure Features are discoverable and work as expected. Provide evidence of testing and validation for each fix implemented. 

Ensure all features of Graph are user-friendly and accessible. Ensure graph interactions are intuitive and responsive.

Ensure WCAG accessibility standards are met for graph interactions (keyboard navigation, screen reader support, color contrast).

Use mcp playwright interractive tests to validate graph functionality and performance improvements. Don't take screenshots as it bloat the session, use the interactive mode to verify functionality.

As part of the mission for other screens query, documents, etc:

Verify screen layout of each screen and panel to ensure no visual bugs or misalignments. Ensure all UI elements are visible and properly styled. Check for responsiveness on different screen sizes. Ensure scrollable areas are properly contained and don't cause layout issues. Verify zooming interactions work smoothly without visual glitches. Ensure Fixed Area and Scrollable Area are properly defined and don't overlap or cause usability issues.



## Context

- **Location**: `/Users/raphaelmansuy/Github/03-working/edgequake`
- **Frontend**: `edgequake_webui/` - Next.js + React + TypeScript + Sigma.js
- **Backend**: `edgequake/crates/` - Rust API server
- **Graph Store**: `use-graph-store.ts` - Zustand state management
- **Graph Renderer**: Sigma.js with WebGL rendering

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

You Must absolutely read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.

Mission file: `specs/001-optimize-graph-mission/mission.md`

You Must always produce the 4 files per iteration:

1. `observe.md` -> Map the territory. Never make assumptions about code structure or function. Always verify against the actual codebase.
2. `orient.md` -> Analyze findings and define possible solutions using First Principles as north star. Assess risks and benefits.
3. `decide.md` -> Prioritize specific changes to be made based on signal value and impact.
4. `act.md` -> Implement decided changes with precision, reference specific file:line numbers and commit SHAs.

```
001-optimize-graph-mission/ooda_loop/
├── iteration_01/
│   ├── observe.md
│   ├── orient.md
│   ├── decide.md
│   └── act.md
├── iteration_02/
│   └── ...
└── summary.md
```

### Per-Iteration Requirements

| Step        | Output                                                     |
| ----------- | ---------------------------------------------------------- |
| **Observe** | Code analysis, feature inventory, dependency mapping       |
| **Orient**  | Gap analysis, documentation quality assessment             |
| **Decide**  | Specific changes prioritized by signal value               |
| **Act**     | Implementation with commit (`OODA-XX: <decision summary>`) |

### Constraints

1. **Re-read mission** every iteration
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Apply SRP** - Split large files for maintainability
6. **Apply DRY** - Don't repeat yourself
7. **Document WHY** in code comments
8. **Test everything** - deliver evidence all tests pass

---

## Current Issues (from screenshots)

1. Graph loads 1700+ nodes (should be max 500)
2. `Entity 'CRÉANCES_CLIENTS' not found` error on expand
3. Node labels not visible in visualization
4. Search doesn't refresh graph view

## Key Files to Investigate

- `edgequake_webui/src/stores/use-graph-store.ts` - maxNodes setting, MAX_DISPLAY_NODES
- `edgequake_webui/src/components/graph/graph-settings-panel.tsx` - localStorage persistence
- `edgequake_webui/src/lib/graph/auto-optimize.ts` - auto-optimization logic
- `edgequake_webui/src/hooks/use-graph-expansion.ts` - expand neighbors logic
- `edgequake_webui/src/components/graph/graph-renderer.tsx` - label rendering
- `edgequake_webui/src/components/graph/graph-search.tsx` - search functionality
- `edgequake/crates/edgequake-api/src/handlers/graph.rs` - API endpoints
- `edgequake/crates/edgequake-api/src/handlers/entities.rs` - Entity lookup

## Success Criteria

- [ ] Graph loads ≤500 nodes initially
- [ ] Expand neighbors works without errors
- [ ] Node labels visible on zoom
- [ ] Search queries server and centers view on result
- [ ] Loading time < 2 seconds for 500 nodes
- [ ] All tests passing
- [ ] All features meet WCAG accessibility standards

---

⚠️ **CRITICAL**: Re-read this mission file at the start of EVERY OODA iteration to prevent alignment drift.
