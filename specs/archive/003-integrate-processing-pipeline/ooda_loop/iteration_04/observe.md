# OODA Iteration 04: Observe - Multi-Tenant Isolation & UX Issues

## Mission Re-Read

As mandated, re-read mission file at start of each iteration.
Mission: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/003-integrate-processing-pipeline.md`

Key new requirements:

1. Fix multi-tenant isolation for live indicators
2. Improve Pipeline UX/UI feedback
3. Fix Pipeline Status modal button order
4. Verify integration & deletion scenarios
5. Validate all tests
6. First principles improvements
7. Full verification

---

## Observed Issues

### Issue 1: Queue Metrics NOT Tenant-Isolated

**Location**: `edgequake_webui/src/components/pipeline/pipeline-monitor.tsx:545`

```typescript
const { data: metrics, isLoading } = useQuery<QueueMetrics>({
  queryKey: scopedQueryKey(
    "queue-metrics",
    selectedTenantId,
    selectedWorkspaceId,
  ),
  queryFn: getQueueMetrics, // ❌ Does NOT pass tenant/workspace!
  refetchInterval: 3000,
});
```

**Backend Issue**: `edgequake/crates/edgequake-api/src/handlers/pipeline.rs:135-157`

```rust
pub async fn get_queue_metrics(
    State(state): State<AppState>,
    // ❌ Missing: tenant_ctx: TenantContext
) -> ApiResult<Json<QueueMetricsResponse>> {
    let metrics = state
        .task_storage
        .get_queue_metrics()  // ❌ Queries ALL tenants
        .await
```

**Root Cause**:

- Backend `get_queue_metrics` doesn't accept tenant context
- Backend `task_storage.get_queue_metrics()` doesn't filter by tenant/workspace
- Frontend passes scoped query key but doesn't pass params to API

---

### Issue 2: Pipeline Status Dialog Button Order

**Location**: `edgequake_webui/src/components/documents/pipeline-status-dialog.tsx:662-685`

**Current State**: ✅ Already correct!

- Close button is first (left, outline)
- Cancel Pipeline is second (right, destructive)

**Screenshot Analysis**:
From the user's screenshot, I see "Close" and "Cancel Pipeline" buttons. The order appears correct but let me verify the default focus behavior.

---

### Issue 3: Messages Show IDs Instead of Document Names

**Location**: `edgequake_webui/src/components/documents/pipeline-status-dialog.tsx`

**Current Messages**:

- "Chunking document 549aa1fe-3d7b-4908-996b-60c46a91da33..."
- "776d2ee5-66f4-42f8-b121-4553ee0e9643 (624 entities) - 3/0"
- "Extracting entities from 776d2ee5-66f4-42f8-b121-4553ee0e9643..."

**Expected Messages**:

- "Chunking document 'My Research Paper.pdf' (doc-549aa1fe)..."
- "'Annual Report 2024' completed: 624 entities, 312 relationships"
- "Extracting entities from 'Technical Spec v2.md' chunk 5/23..."

---

### Issue 4: Enhanced Pipeline Status Query

**Location**: `edgequake_webui/src/lib/api/edgequake.ts:1297-1338`

```typescript
export async function getEnhancedPipelineStatus(
  tenant_id?: string,
  workspace_id?: string,
): Promise<EnhancedPipelineStatus> {
  // ✅ Correctly passes tenant_id and workspace_id as query params
```

**Backend**: The backend properly filters by tenant/workspace in the pipeline status.

---

## Current Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        FRONTEND                                      │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │
│  │ QueueMetricsCard│  │ PipelineStatus  │  │ TaskQueueCard       │  │
│  │ ❌ No filtering │  │ ✅ Filtered     │  │ ? Need to check     │  │
│  └────────┬────────┘  └────────┬────────┘  └──────────┬──────────┘  │
│           │                    │                       │             │
│           └────────────────────┴───────────────────────┘             │
│                                │                                     │
│                       getQueueMetrics()  ❌ No params                │
│                       getEnhancedPipelineStatus(tenant, workspace) ✅│
└────────────────────────────────┼─────────────────────────────────────┘
                                 │
┌────────────────────────────────┼─────────────────────────────────────┐
│                        BACKEND (Axum)                                │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │
│  │ get_queue_      │  │ get_enhanced_   │  │ get_tasks_list      │  │
│  │ metrics         │  │ pipeline_status │  │                     │  │
│  │ ❌ No tenant    │  │ ✅ Has tenant   │  │ ✅ Has tenant       │  │
│  └────────┬────────┘  └────────┬────────┘  └──────────┬──────────┘  │
│           │                    │                       │             │
│           │        ┌───────────┴───────────────────────┘             │
│           │        │                                                 │
│           └────────┴─────────────────────────────────────────────────│
│                                │                                     │
│                    TaskStorage::get_queue_metrics()                  │
│                    ❌ Queries ALL tenants                            │
└──────────────────────────────────────────────────────────────────────┘
```

---

## Files to Modify

| File                                                           | Issue                               | Action Required                    |
| -------------------------------------------------------------- | ----------------------------------- | ---------------------------------- |
| `edgequake-api/src/handlers/pipeline.rs`                       | Queue metrics not tenant-scoped     | Add TenantContext, pass to storage |
| `edgequake-tasks/src/storage.rs`                               | Storage trait missing tenant filter | Add tenant/workspace params        |
| `edgequake-tasks/src/postgres.rs`                              | PostgreSQL impl not filtered        | Add WHERE clause for tenant        |
| `edgequake-tasks/src/memory.rs`                                | Memory impl not filtered            | Add filter logic                   |
| `edgequake_webui/src/lib/api/edgequake.ts`                     | getQueueMetrics no params           | Add tenant/workspace params        |
| `edgequake_webui/src/components/pipeline/pipeline-monitor.tsx` | QueueMetricsCard                    | Pass params to API                 |

---

## Next Steps (Orient Phase)

1. Analyze the impact of adding tenant filtering to queue metrics
2. Design the API changes for backward compatibility
3. Identify all places where queue metrics are used
4. Plan message improvements for document names
