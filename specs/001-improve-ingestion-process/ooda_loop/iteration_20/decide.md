# Iteration 20: Decide

## Action Plan

### Step 1: Add QueueMetricsResponse DTO

File: `edgequake-api/src/handlers/pipeline_types.rs`

- Add QueueMetricsResponse struct with Serialize, Deserialize, ToSchema

### Step 2: Add get_queue_metrics Handler

File: `edgequake-api/src/handlers/pipeline.rs`

- Add handler function following existing pattern
- Call state.task_storage.get_queue_metrics()
- Map to QueueMetricsResponse

### Step 3: Register Route

File: `edgequake-api/src/routes.rs`

- Add route: `/pipeline/queue-metrics` → GET → `handlers::get_queue_metrics`

### Step 4: Validate

- Build: `cargo build -p edgequake-api`
- Test: `cargo test -p edgequake-api`

## Priority

- HIGH: Core requirement for Objective B
- Dependencies: Iteration 19 (QueueMetrics struct) ✅

## Risk Assessment

- LOW: Simple CRUD-style endpoint
- Follows established patterns exactly
