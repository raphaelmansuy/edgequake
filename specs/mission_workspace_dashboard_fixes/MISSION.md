# Mission: Workspace Dashboard and Processing Fixes

## Task

Your mission is to fix five critical issues in the EdgeQuake WebUI:

1. **Tenant/Workspace Visibility**: Ensure the selected Tenant/Workspace name is always fully visible in the selection dropdown (not truncated)
2. **Dashboard Statistics Accuracy**: Dashboard metrics (Documents, Entities, Relationships, Entity Types) must accurately reflect the selected workspace's data
3. **KG Rebuild Resilience**: Knowledge Graph rebuild must work correctly even when embeddings/LLM models are changed
4. **Document Reprocessing**: The "Reprocess Documents" feature must actually reprocess documents when triggered from the UI
5. **Build CPU Crash Prevention**: Frontend builds must not cause 100% CPU usage or VS Code crashes

## Amendment: CPU Crash Issue (2026-01-26 22:30)

During iteration 01, a critical build issue was identified:

- **Problem**: `next build` or `npm run build` can cause 100% CPU usage, freezing VS Code
- **Solution**: Use the safe build script: `npm run build:safe` or `./scripts/safe-build.sh`
- **Details**: The safe-build script includes:
  - Cache cleanup before build (`rm -rf .next node_modules/.cache`)
  - Memory limits for Node.js (`NODE_OPTIONS="--max-old-space-size=4096"`)
  - CPU priority throttling (`nice -n 10`)
  - Timeout protection (300s default)
  - TypeScript check before build

## Amendment: Statistics Inaccuracy Bug (2026-01-26 22:35)

**CRITICAL BUG DISCOVERED**: Dashboard and Workspace pages show incorrect statistics:

- **Observed**: Dashboard shows 0 Entities, 0 Relationships, 0 Chunks (despite document having 13 entities)
- **Expected**: Dashboard should show accurate counts matching the knowledge graph data
- **Root Cause #1** (FIXED Iteration 03): Backend was querying KV storage metadata instead of Apache AGE graph
  - KV storage metadata doesn't have entity_count/relationship_count fields
  - The actual entity/relationship data is stored in the graph storage
  - **Fix**: Modified `try_kv_storage_stats()` to call `graph_storage.node_count_by_workspace()` and `edge_count_by_workspace()`
- **Root Cause #2** (FIXED Iteration 03-CACHE-FIX): Workspace stats cache not invalidated after document processing
  - Backend has 60-second TTL cache on workspace stats (WORKSPACE_STATS_CACHE)
  - Dashboard loaded first with 0 entities (before fix), cache populated with stale data
  - Workspace page used the same stale cached data, causing discrepancy
  - **Fix**: Added `invalidate_workspace_stats_cache()` helper function
  - Called after document upload (sync processing) in [documents.rs](../../edgequake/crates/edgequake-api/src/handlers/documents.rs)
  - Called after task completion (async processing) in [processor.rs](../../edgequake/crates/edgequake-api/src/processor.rs)
- **Impact**: Users cannot see their knowledge graph extraction results
- **Status**: RESOLVED - Both backend query logic and cache invalidation implemented

## Amendment: Statistics Inaccuracy Bug (2026-01-26 22:35) - DEPRECATED

**NOTE**: This section is kept for historical context. See above for the complete fix.

~~**CRITICAL BUG DISCOVERED**: Dashboard and Workspace pages show incorrect statistics:~~

~~- **Observed**: Dashboard shows 0 Entities, 0 Relationships, 0 Chunks (despite document having 13 entities)~~
~~- **Expected**: Dashboard should show accurate counts matching the knowledge graph data~~
~~- **Root Cause**: To be investigated - likely backend stats API or database query issue~~
~~- **Impact**: Users cannot see their knowledge graph extraction results~~
~~- **Status**: Issue 2 (Dashboard Statistics Accuracy) was INCORRECTLY marked as resolved~~
~~- **Action**: Reopen Issue 2, investigate backend stats endpoint, fix query logic~~

## Context

- **Location**: `edgequake_webui/` - React 19 + TypeScript frontend
- **Backend**: `edgequake/crates/edgequake-api/` - Rust Axum API
- **Storage**: PostgreSQL with Apache AGE graph extension

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

You Must absolutely read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.

Mission file: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/mission_workspace_dashboard_fixes/MISSION.md`

You Must always produce the 4 files per iteration, as shown below:

```
mission_workspace_dashboard_fixes/ooda_loop/
├── iteration_01/
│   ├── observe.md   # Data gathered: code, business rules, workflows
│   ├── orient.md    # Analysis of findings vs. current docs
│   ├── decide.md    # Prioritized action plan
│   └── act.md       # Changes made, with file:line references + commit hashes
├── iteration_02/
│   └── observe.md
│   └── orient.md
│   └── decide.md
│   └── act.md
├── iteration_03/
│   └── ...
└── summary.md       # Cross-iteration insights
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
5. **Split large files** for maintainability, Use Single Responsibility Principle (SRP)
6. **Optimize** Rust build speed (latest toolchain)
7. **Document amendments** in WHY, high signal value, and precise terms in comments
8. **You must perform tests** and deliver evidence that all tests are passing

## Success Criteria

- [x] Workspace name fully visible in dropdown (no truncation) ✅ 30-char limit + 200px max-width
- [x] Dashboard shows correct counts per workspace ✅ AGE graph queries + cache invalidation after processing
- [x] KG rebuild works with model changes ✅ Cache eviction + config update before reprocess
- [x] Document reprocessing actually triggers backend processing ✅ With feedback UI
- [x] All tests pass ✅ TypeScript + 423 Rust tests + 13 unit tests
- [x] No regressions introduced ✅ All existing tests passing
- [x] Cache consistency maintained ✅ Stats cache invalidated after sync/async document processing
