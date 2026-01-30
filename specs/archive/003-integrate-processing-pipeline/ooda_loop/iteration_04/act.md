# OODA-04: Act - Multi-Tenant Queue Metrics Isolation

**Date**: 2026-01-29
**Objective**: Fix queue metrics endpoint to filter by tenant/workspace

## Changes Implemented

### 1. Backend: TaskStorage Trait (storage.rs)

- Added new method `get_queue_metrics_filtered(tenant_id, workspace_id)` to trait
- Made original `get_queue_metrics()` a default implementation that calls filtered version with None
- This ensures backward compatibility while enabling tenant isolation

### 2. Backend: PostgreSQL Implementation (postgres.rs)

- Renamed `get_queue_metrics` to `get_queue_metrics_filtered`
- Added WHERE clause: `WHERE ($1::uuid IS NULL OR tenant_id = $1) AND ($2::uuid IS NULL OR workspace_id = $2)`
- Added `.bind(tenant_id).bind(workspace_id)` for parameter binding
- Uses optional filtering pattern: NULL parameter means "no filter"

### 3. Backend: Memory Implementation (memory.rs)

- Renamed to `get_queue_metrics_filtered`
- Added filter logic in task iteration loop:
  - Skip tasks not matching tenant_id if provided
  - Skip tasks not matching workspace_id if provided

### 4. Backend: API Handler (pipeline.rs)

- Added imports: `Query`, `Deserialize`, `IntoParams`, `TenantContext`
- Created `QueueMetricsQuery` struct for query params (tenant_id, workspace_id)
- Updated handler signature to accept `TenantContext` and `Query<QueueMetricsQuery>`
- Added UUID parsing logic with priority chain:
  1. Query params take precedence
  2. Fall back to TenantContext
  3. Fall back to None (no filter)

### 5. Frontend: API Function (edgequake.ts)

- Updated `getQueueMetrics()` to accept optional `tenantId` and `workspaceId` parameters
- Builds query params and appends to URL if provided
- Added comprehensive JSDoc documentation with @implements annotations

### 6. Frontend: QueueMetricsCard Component (pipeline-monitor.tsx)

- Updated queryFn to pass tenant/workspace context:
  ```typescript
  queryFn: () =>
    getQueueMetrics(
      selectedTenantId ?? undefined,
      selectedWorkspaceId ?? undefined,
    );
  ```
- Added @implements OODA-04 annotation
- Added CRITICAL comment explaining why isolation is necessary

## Verification

- ✅ Backend builds successfully (`cargo build --package edgequake-tasks --package edgequake-api`)
- ✅ Frontend TypeScript compiles with no errors (`pnpm tsc --noEmit`)
- ✅ All files properly annotated with @implements tags

## Impact

**Before**: Users could see "Live" indicator and queue metrics from other tenants' document processing activity.

**After**: Queue metrics are filtered by tenant_id and workspace_id, ensuring users only see activity from their own workspace. This is a critical privacy fix.

## Files Modified

1. `edgequake/crates/edgequake-tasks/src/storage.rs`
2. `edgequake/crates/edgequake-tasks/src/postgres.rs`
3. `edgequake/crates/edgequake-tasks/src/memory.rs`
4. `edgequake/crates/edgequake-api/src/handlers/pipeline.rs`
5. `edgequake_webui/src/lib/api/edgequake.ts`
6. `edgequake_webui/src/components/pipeline/pipeline-monitor.tsx`

## Next Steps

- OODA-05: Improve UX/UI feedback (document names in progress messages)
- OODA-06: Verify Pipeline Status modal button order
- OODA-07: Verify integration/deletion scenarios
- OODA-08: Run all tests
- OODA-09: First principles improvements
