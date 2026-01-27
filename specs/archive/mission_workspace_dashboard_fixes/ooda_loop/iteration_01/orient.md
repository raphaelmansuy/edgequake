# OODA Loop - Iteration 01: Orient

## Mission Reminder

**Re-read Mission**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/mission_workspace_dashboard_fixes/MISSION.md`

---

## Analysis & Gap Assessment

### Issue 1: Workspace Name Visibility

**Root Cause Analysis**:
- Hard-coded truncation at 16 characters is too aggressive
- CSS `max-w-[120px]` further constrains visible width
- "Workspace A / Test" = 18 chars → shows as "Workspace A / Te..."

**First Principles**:
- User needs to identify which workspace is selected
- Full name should be visible OR a tooltip should show full name
- Truncation should be responsive, not fixed

**Solution Options**:

| Option | Pros | Cons | Recommendation |
|--------|------|------|----------------|
| A. Remove truncation entirely | Simple | May break layout on long names | ❌ |
| B. Increase max-width | Quick fix | Still truncates at some point | ⚠️ |
| C. Dynamic width with tooltip | Best UX | More complex | ✅ |
| D. Expand on hover | Good UX | CSS complexity | ⚠️ |

**Decision**: Option C - Increase `max-w` to `max-w-[200px]` and ensure tooltip shows full name.

---

### Issue 2: Dashboard Stats Not Displayed

**Root Cause Analysis**:
- Dashboard page (`/app/(dashboard)/page.tsx`) has NO stats cards
- `StatsCard` component exists in `components/dashboard/stats-card.tsx`
- `getWorkspaceStats()` API exists but is not called from dashboard
- Stats ARE shown on `/workspace` page but NOT on main dashboard

**First Principles**:
- Dashboard is the first thing users see
- Stats should be immediately visible
- Must react to workspace changes

**Solution**:
1. Import `StatsCard` component
2. Call `getWorkspaceStats(selectedWorkspaceId)` with useQuery
3. Display 4 stats cards: Documents, Entities, Relationships, Entity Types
4. Add dependency on `selectedWorkspaceId` for reactivity

**Architecture**:
```
Dashboard Page (Enhanced)
├── Stats Section (NEW)
│   ├── StatsCard variant="documents"
│   ├── StatsCard variant="entities"
│   ├── StatsCard variant="relationships"
│   └── StatsCard variant="types"
├── QuickActions
└── RecentActivity + SystemStatus
```

---

### Issue 3: KG Rebuild with Model Changes

**Analysis**:
- Current flow: Update workspace → Rebuild KG → Reprocess docs
- Question: Does reprocessing use the NEW LLM model?

**Evidence Check Needed**:
1. When `updateWorkspace` is called with new `llm_model`, it persists to DB
2. When `reprocessAllDocuments` creates tasks, it includes `workspace_id`
3. The ingestion pipeline should read workspace config at processing time

**Investigation**: Check ingestion task handler for LLM model resolution.

**Risk Assessment**:
- HIGH: If old model is cached, rebuild won't use new settings
- MEDIUM: Need to verify workspace config is re-fetched during processing

**Verification Steps**:
1. Check task processing code for workspace config lookup
2. Confirm LLM provider factory uses workspace config
3. Add logging if needed for debugging

---

### Issue 4: Document Reprocessing

**Analysis**:
- Backend uses KV storage scan to find documents
- Filters by `workspace_id` field in metadata
- Creates new Task with `is_reprocess: true` marker

**Potential Issues Identified**:

1. **Workspace ID Matching**:
   ```rust
   if doc_workspace != workspace_id.to_string() && doc_workspace != "default" {
       continue;
   }
   ```
   This allows "default" workspace docs to be processed for ANY workspace - may be intentional for migration but could cause issues.

2. **Content Availability**:
   ```rust
   let content_key = format!("{}-content", doc_id);
   let content = match state.kv_storage.get_by_id(&content_key).await {
   ```
   If content is missing, document is skipped silently.

3. **Task Queue Delivery**:
   - Tasks are created and queued via `task_queue.send()`
   - Need to verify pipeline is actively consuming tasks

**Solution**: 
- Add better error handling and user feedback
- Ensure pipeline status is visible in UI
- Consider adding document-level status tracking

---

## Priority Matrix

| Issue | Impact | Complexity | Priority |
|-------|--------|------------|----------|
| 1. Name visibility | Low-Medium | Low | P2 |
| 2. Dashboard stats | High | Medium | P1 |
| 3. KG rebuild | High | Medium | P1 |
| 4. Reprocessing | High | Low | P1 |

**Execution Order**: 2 → 1 → 3 → 4 (stats first for immediate user value)

---

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Breaking existing layout | Low | Test in browser |
| Stats API returns wrong data | Low | Verify backend logic |
| Reprocessing queue overload | Medium | Max documents limit exists |
| Model change not reflected | Medium | Add cache invalidation |
