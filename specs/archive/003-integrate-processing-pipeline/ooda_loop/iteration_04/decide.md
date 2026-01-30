# OODA Iteration 04: Decide - Prioritized Action Plan

## Decision: Fix Queue Metrics Multi-Tenant Isolation

**Priority**: P0 (Critical Security/Privacy Issue)

This is the most impactful change because:

1. Users currently see activity from ALL workspaces
2. This violates multi-tenancy isolation principle
3. It confuses users about their own workspace status

---

## Action Plan

### Step 1: Update TaskStorage Trait

**File**: `edgequake/crates/edgequake-tasks/src/storage.rs`

Add new method with tenant/workspace filtering:

```rust
async fn get_queue_metrics_filtered(
    &self,
    tenant_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
) -> TaskResult<QueueMetrics>;
```

Make existing `get_queue_metrics()` call the filtered version with None values for backward compatibility.

### Step 2: Update PostgreSQL Implementation

**File**: `edgequake/crates/edgequake-tasks/src/postgres.rs`

Modify query to add optional tenant/workspace WHERE clauses:

```sql
SELECT
    COUNT(*) FILTER (WHERE status = 'pending') as pending_count,
    ...
FROM tasks
WHERE ($1::uuid IS NULL OR tenant_id = $1)
  AND ($2::uuid IS NULL OR workspace_id = $2)
```

### Step 3: Update Memory Implementation

**File**: `edgequake/crates/edgequake-tasks/src/memory.rs`

Filter in-memory tasks by tenant/workspace before counting.

### Step 4: Update API Handler

**File**: `edgequake/crates/edgequake-api/src/handlers/pipeline.rs`

Add TenantContext extraction and query params:

```rust
pub async fn get_queue_metrics(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Query(params): Query<QueueMetricsQuery>,
) -> ApiResult<Json<QueueMetricsResponse>>
```

### Step 5: Update Frontend API

**File**: `edgequake_webui/src/lib/api/edgequake.ts`

Update function signature:

```typescript
export async function getQueueMetrics(
  tenantId?: string,
  workspaceId?: string,
): Promise<QueueMetrics>;
```

### Step 6: Update QueueMetricsCard Component

**File**: `edgequake_webui/src/components/pipeline/pipeline-monitor.tsx`

Pass context to API call:

```typescript
queryFn: () => getQueueMetrics(
  selectedTenantId ?? undefined,
  selectedWorkspaceId ?? undefined
),
```

---

## Verification Checklist

- [ ] Backend builds without errors
- [ ] All Rust tests pass
- [ ] Frontend TypeScript compiles
- [ ] Queue metrics show only current workspace tasks
- [ ] Live indicator respects workspace isolation
- [ ] Empty workspace shows zero metrics

---

## Commit Strategy

Single commit for all related changes:

```
feat(api): add multi-tenant isolation to queue metrics

OODA-04: Fix queue metrics tenant isolation

- TaskStorage trait: add get_queue_metrics_filtered()
- PostgreSQL: filter by tenant_id and workspace_id
- Memory: filter by tenant_id and workspace_id
- API handler: extract TenantContext and pass to storage
- Frontend: pass tenant/workspace to API call
- QueueMetricsCard: use workspace context

Fixes privacy violation where users could see metrics from all workspaces.
```
