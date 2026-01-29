# OODA Iteration 04: Orient - Analysis & Gap Assessment

## Gap Analysis

### Gap 1: Queue Metrics Multi-Tenant Isolation

| Aspect             | Current State             | Required State                               | Gap        |
| ------------------ | ------------------------- | -------------------------------------------- | ---------- |
| Backend API        | No tenant filter          | Filter by tenant/workspace                   | ❌ Missing |
| PostgreSQL         | `SELECT FROM tasks` (all) | `WHERE tenant_id = $1 AND workspace_id = $2` | ❌ Missing |
| Memory Storage     | No filter                 | Filter by tenant/workspace                   | ❌ Missing |
| Frontend API       | `getQueueMetrics()`       | `getQueueMetrics(tenant, workspace)`         | ❌ Missing |
| Frontend Component | No params passed          | Pass context params                          | ❌ Missing |

**Impact Assessment**:

- **Security Risk**: HIGH - Users see processing activity from other tenants
- **Privacy Violation**: Users can infer competitor activity from metrics
- **User Confusion**: Metrics don't match what user expects for their workspace

### Gap 2: Pipeline Status Modal UX

| Aspect              | Current State              | Required State                 | Gap           |
| ------------------- | -------------------------- | ------------------------------ | ------------- |
| Button Order        | Close first, Cancel second | ✅ Already correct             | None          |
| Default Focus       | Unknown                    | Close button should have focus | ? Need verify |
| Cancel Confirmation | Has confirmation dialog    | ✅ Already correct             | None          |

### Gap 3: Message Quality

| Aspect                  | Current State | Required State               | Gap              |
| ----------------------- | ------------- | ---------------------------- | ---------------- |
| Document ID in messages | UUID only     | Document name + truncated ID | ❌ Missing       |
| Chunk progress          | Not shown     | "Chunk 5/23"                 | ❌ Missing       |
| Entity count            | Shown at end  | Show incrementally           | ❌ Could improve |
| Cost                    | Shown at end  | Real-time tracking           | ✅ Already works |

---

## Risk Assessment

### Change 1: Add Tenant Filter to Queue Metrics

| Risk               | Probability | Impact | Mitigation                               |
| ------------------ | ----------- | ------ | ---------------------------------------- |
| Breaking change    | Low         | Medium | Make filter params optional with default |
| Performance impact | Low         | Low    | Tenant/workspace already indexed         |
| Test coverage gap  | Medium      | Medium | Add explicit tenant-scoped tests         |

### Change 2: Message Improvements

| Risk                   | Probability | Impact | Mitigation                        |
| ---------------------- | ----------- | ------ | --------------------------------- |
| Missing document names | Medium      | Low    | Graceful fallback to ID           |
| Performance overhead   | Low         | Low    | Document names already in context |

---

## Proposed Architecture

### Queue Metrics with Tenant Isolation

```
┌─────────────────────────────────────────────────────────────────────┐
│                        FRONTEND                                      │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │ QueueMetricsCard                                                 ││
│  │  queryFn: () => getQueueMetrics(tenantId, workspaceId)          ││
│  └─────────────────────────┬───────────────────────────────────────┘│
└────────────────────────────┼────────────────────────────────────────┘
                             │ GET /api/v1/pipeline/queue-metrics
                             │     ?tenant_id=xxx&workspace_id=yyy
                             ▼
┌────────────────────────────┼────────────────────────────────────────┐
│                        BACKEND                                       │
│  ┌─────────────────────────┴───────────────────────────────────────┐│
│  │ get_queue_metrics(State, Query<QueueMetricsQuery>)               ││
│  │   → tenant_id from TenantContext or query                        ││
│  │   → workspace_id from query                                      ││
│  └─────────────────────────┬───────────────────────────────────────┘│
│                            │                                         │
│  ┌─────────────────────────┴───────────────────────────────────────┐│
│  │ task_storage.get_queue_metrics_filtered(tenant_id, workspace_id) ││
│  │   SELECT ... FROM tasks                                          ││
│  │   WHERE ($1 IS NULL OR tenant_id = $1)                          ││
│  │   AND ($2 IS NULL OR workspace_id = $2)                         ││
│  └─────────────────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Strategy

### Phase 1: Backend Changes (Safe, Backward Compatible)

1. Add `get_queue_metrics_filtered(tenant_id, workspace_id)` to `TaskStorage` trait
2. Keep existing `get_queue_metrics()` as wrapper calling filtered version with None
3. Update `get_queue_metrics` API handler to use TenantContext
4. Add query params for explicit tenant/workspace filtering

### Phase 2: Frontend Changes

1. Update `getQueueMetrics()` to accept optional tenant/workspace params
2. Update `QueueMetricsCard` to pass context params
3. Update `TaskQueueCard` if it also needs filtering

### Phase 3: Message Improvements

1. Include document_name in pipeline progress events
2. Update message formatters to prefer name over ID
3. Add chunk progress to extraction messages

---

## Decision Matrix

| Change                      | Value  | Risk | Effort | Priority |
| --------------------------- | ------ | ---- | ------ | -------- |
| Queue metrics tenant filter | HIGH   | LOW  | MEDIUM | **P0**   |
| Message improvements        | MEDIUM | LOW  | LOW    | P1       |
| Button focus behavior       | LOW    | LOW  | LOW    | P2       |

---

## Next Steps (Decide Phase)

1. Implement queue metrics tenant filtering (P0)
2. Verify all tests pass after changes
3. Manual verification with multiple workspaces
4. Document the changes
