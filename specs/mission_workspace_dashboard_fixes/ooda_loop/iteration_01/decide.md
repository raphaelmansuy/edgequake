# OODA Loop - Iteration 01: Decide

## Mission Reminder

**Re-read Mission**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/mission_workspace_dashboard_fixes/MISSION.md`

---

## Prioritized Action Plan

### Phase 1: Dashboard Stats (High Impact, Immediate Visibility)

**Task 1.1: Add Stats Cards to Dashboard**

**File**: `edgequake_webui/src/app/(dashboard)/page.tsx`

**Changes**:
1. Import `StatsCard` from `@/components/dashboard`
2. Import `getWorkspaceStats` from `@/lib/api/edgequake`
3. Import additional icons: `FileText, Users, GitBranch, Tags` from `lucide-react`
4. Add `useQuery` for workspace stats with dependency on `selectedWorkspaceId`
5. Add stats section with 4 `StatsCard` components

**Implementation**:
```tsx
// Add to imports
import { StatsCard } from '@/components/dashboard';
import { getWorkspaceStats } from '@/lib/api/edgequake';
import { FileText, Users, GitBranch, Tags } from 'lucide-react';

// Add query for stats
const { data: stats, isLoading: isLoadingStats } = useQuery({
  queryKey: ['workspaceStats', selectedWorkspaceId],
  queryFn: () => selectedWorkspaceId 
    ? getWorkspaceStats(selectedWorkspaceId) 
    : Promise.reject('No workspace'),
  enabled: !!selectedWorkspaceId,
  staleTime: 30000,
});

// Add stats section before QuickActions
<section aria-label="Statistics" className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
  <StatsCard
    title="Documents"
    value={stats?.document_count ?? 0}
    description="Uploaded documents"
    icon={FileText}
    variant="documents"
    isLoading={isLoadingStats}
  />
  {/* ... 3 more cards */}
</section>
```

---

### Phase 2: Workspace Name Visibility

**Task 2.1: Fix Name Truncation**

**File**: `edgequake_webui/src/components/layout/header-tenant-selector.tsx`

**Changes** (Lines 244-262):
1. Increase truncation limit from 16 to 25 characters
2. Increase `max-w-[120px]` to `max-w-[200px]`
3. Ensure tooltip shows full name (already implemented)

**Specific Edits**:
```tsx
// Line 244: Change truncation limit
const truncatedName = displayName.length > 25 ? displayName.slice(0, 25) + '...' : displayName;

// Line 262: Change max-width
<span className="max-w-[200px] truncate hidden sm:inline">
```

---

### Phase 3: KG Rebuild Verification

**Task 3.1: Verify Model Resolution in Pipeline**

**Files to Check**:
- `edgequake/crates/edgequake-api/src/handlers/documents.rs` - Task processing
- `edgequake/crates/edgequake-pipeline/` - LLM provider resolution

**Verification**:
1. Confirm workspace config is fetched at task processing time
2. Confirm LLM provider factory uses workspace.llm_provider/llm_model
3. If cached, add cache invalidation on workspace update

---

### Phase 4: Document Reprocessing

**Task 4.1: Enhance Error Visibility**

**Current behavior**: Silent skipping of documents without content

**Enhancement needed**: 
- Log skipped documents with reasons
- Return skip counts in API response (already done)
- Show skip reasons in UI

**Files**:
- Backend already logs reasons (verified in observe)
- Frontend needs to display `documents_skipped` in toast

---

## Commit Strategy

| Commit | Description | Files |
|--------|-------------|-------|
| OODA-01a | Add stats cards to dashboard | `page.tsx` |
| OODA-01b | Fix workspace name truncation | `header-tenant-selector.tsx` |
| OODA-01c | Verify KG rebuild model usage | Documentation/verification |
| OODA-01d | Enhance reprocess feedback | `rebuild-knowledge-graph-button.tsx` |

---

## Acceptance Criteria

- [ ] Dashboard shows 4 stats cards with correct counts
- [ ] Stats update when workspace changes
- [ ] Workspace name shows at least 25 chars
- [ ] Tooltip shows full workspace name
- [ ] KG rebuild uses new model configuration
- [ ] Reprocess shows skipped document count
