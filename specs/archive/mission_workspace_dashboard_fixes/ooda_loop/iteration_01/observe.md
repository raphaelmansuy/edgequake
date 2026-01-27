# OODA Loop - Iteration 01: Observe

## Mission Reminder

**Re-read Mission**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/mission_workspace_dashboard_fixes/MISSION.md`

Fix four critical issues:
1. Tenant/Workspace name visibility (truncated)
2. Dashboard statistics accuracy per workspace
3. KG rebuild resilience with model changes
4. Document reprocessing functionality

---

## Observations

### Issue 1: Workspace Name Truncation

**Location**: [header-tenant-selector.tsx](edgequake_webui/src/components/layout/header-tenant-selector.tsx#L244-L247)

```tsx
const displayName = selectedWorkspace?.name || selectedTenant?.name || t('tenant.selectContext', 'Select workspace');
const truncatedName = displayName.length > 16 ? displayName.slice(0, 16) + '...' : displayName;
```

**Problem**: The workspace name is hard-coded to truncate at 16 characters and shown with `max-w-[120px] truncate`.

**Evidence**: 
- Line 262: `<span className="max-w-[120px] truncate hidden sm:inline">`
- Shows `truncatedName` which cuts at 16 chars + "..."

**Impact**: Users cannot see full workspace names like "Workspace A / Test" visible in screenshot.

---

### Issue 2: Dashboard Missing Stats Cards

**Location**: [page.tsx (dashboard)](edgequake_webui/src/app/(dashboard)/page.tsx)

**Problem**: The dashboard page does NOT display the StatsCard components with Documents, Entities, Relationships, Entity Types counts.

**Current Structure**:
```
Dashboard Page:
├── Header (title + welcome message)
├── QuickActions (upload, query, graph links)
└── RecentActivity + SystemStatus grid
```

**Missing**: There is NO integration with `getWorkspaceStats()` API or StatsCard components.

**Evidence**: Screenshot shows stat cards on dashboard, but current code only shows QuickActions and RecentActivity.

**Required API**: `getWorkspaceStats(workspaceId)` returns:
```typescript
interface WorkspaceStats {
  workspace_id: string;
  document_count: number;
  entity_count: number;
  relationship_count: number;
  chunk_count: number;
  storage_bytes: number;
}
```

---

### Issue 3: KG Rebuild with Model Changes

**Location**: [rebuild-knowledge-graph-button.tsx](edgequake_webui/src/components/workspace/rebuild-knowledge-graph-button.tsx)

**Observed Flow**:
1. `rebuildKnowledgeGraph()` clears graph data (nodes, edges, vectors)
2. Then `reprocessAllDocuments()` queues documents for reprocessing

**Backend Handler**: [workspaces.rs](edgequake/crates/edgequake-api/src/handlers/workspaces.rs#L1710)

**Potential Issue**: 
- When LLM model is changed via `updateWorkspace()`, the rebuild uses the NEW model
- The workspace config is already updated before rebuild is triggered
- Need to verify the pipeline uses the updated workspace config for extraction

**Verification Needed**:
- Check if pipeline reads workspace.llm_model at processing time
- Verify that reprocess tasks include the new model configuration

---

### Issue 4: Document Reprocessing

**Location**: 
- Frontend: [rebuild-knowledge-graph-button.tsx](edgequake_webui/src/components/workspace/rebuild-knowledge-graph-button.tsx#L86-L104)
- Backend: [workspaces.rs](edgequake/crates/edgequake-api/src/handlers/workspaces.rs#L2026)

**Observed Flow**:
```
reprocessAllDocuments(workspaceId, {include_completed: true, max_documents: 10000})
    -> POST /workspaces/{workspace_id}/reprocess-documents
    -> Queue documents as Task::Insert with is_reprocess: true
```

**Potential Issues**:
1. The backend filters by `workspace_id` but uses string comparison: `doc_workspace != workspace_id.to_string()`
2. For "default" workspace documents, they may not be reprocessed correctly
3. Documents may need content from KV storage which could be missing

**Backend Logic** (lines 2076-2095):
- Scans all keys ending with "-metadata"
- Checks `workspace_id` field in metadata
- Skips if status is "processing"
- Creates new Task with `is_reprocess: true` metadata

---

## Component Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Dashboard Layout                         │
├─────────────────────────────────────────────────────────────┤
│ ┌─────────────────┐  ┌──────────────────────────────────┐  │
│ │ Sidebar         │  │ Main Content                     │  │
│ │                 │  │                                   │  │
│ │ - Dashboard     │  │ Header                           │  │
│ │ - Knowledge     │  │   └── HeaderTenantSelector       │  │
│ │ - Documents     │  │       └── Dropdown (truncated!)  │  │
│ │ - Query         │  │                                   │  │
│ │ - Workspace     │  │ Dashboard Page                   │  │
│ │ - Costs         │  │   ├── [MISSING: Stats Cards]     │  │
│ │ - API Explorer  │  │   ├── QuickActions               │  │
│ │ - Settings      │  │   └── RecentActivity+SystemStatus│  │
│ └─────────────────┘  └──────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## Key Files Identified

| Issue | Primary Files | Status |
|-------|---------------|--------|
| 1. Name truncation | `header-tenant-selector.tsx` L244-262 | Bug confirmed |
| 2. Dashboard stats | `(dashboard)/page.tsx` | Missing feature |
| 3. KG rebuild | `rebuild-knowledge-graph-button.tsx`, `workspaces.rs` | Needs verification |
| 4. Reprocessing | `workspaces.rs` L2026-2200 | Needs verification |

---

## Dependencies

- `@tanstack/react-query` for data fetching
- `useTenantStore` Zustand store for workspace selection
- Backend `/workspaces/{id}/stats` endpoint
- Backend `/workspaces/{id}/rebuild-knowledge-graph` endpoint
- Backend `/workspaces/{id}/reprocess-documents` endpoint
