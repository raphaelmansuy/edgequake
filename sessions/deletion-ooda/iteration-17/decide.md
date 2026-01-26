# OODA-17 Decide: Historical Metrics Implementation

## Decision

Create PostgreSQL migration for workspace metrics history table.

## Implementation Steps

### Step 1: Create Migration File

Location: `edgequake/crates/edgequake-storage/migrations/016_workspace_metrics_history.sql`

Contents:

- Create `workspace_metrics_history` table
- Create indexes for efficient queries
- Add trigger_type column to distinguish event vs scheduled samples

### Step 2: Add Metrics Recording Function

In `workspace_service_impl.rs`:

```rust
async fn record_metrics_snapshot(
    &self,
    workspace_id: &str,
    trigger_type: &str,  // "event" or "scheduled"
) -> Result<(), Error>
```

### Step 3: Integrate with Document Handlers

In document add/delete handlers:

- After successful operation, call record_metrics_snapshot
- Pass trigger_type = "event"

## Acceptance Criteria

- [ ] Migration 016 creates workspace_metrics_history table
- [ ] Table includes all required columns
- [ ] Indexes created for efficient queries
- [ ] Build passes with new migration
- [ ] Tests pass

## Files to Create/Modify

1. **CREATE**: `edgequake/crates/edgequake-storage/migrations/016_workspace_metrics_history.sql`

## Run Commands

```bash
# Apply migration (when using PostgreSQL)
cargo run --package edgequake-api -- --migrate

# Verify table exists
psql $DATABASE_URL -c "\\d workspace_metrics_history"
```
