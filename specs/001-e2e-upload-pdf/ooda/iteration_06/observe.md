# OODA Iteration 06 - Observe

**Date**: 2026-02-06
**Focus**: Task Storage Migration to PostgreSQL

## Observation Summary

Tasks are currently stored in-memory (`MemoryTaskStorage`) even when running in PostgreSQL mode. This causes tasks to be lost on backend restart, breaking the cancel functionality for "stuck" documents.

## Evidence Chain

### 1. State Initialization (state.rs:793)

```rust
// Create task infrastructure
let task_storage = Arc::new(edgequake_tasks::memory::MemoryTaskStorage::new());  // ← BUG: Should be PostgresTaskStorage
let task_queue = Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(100));
```

This is inside `AppState::new_postgres()` which is the PostgreSQL mode constructor. Despite all other storage backends using PostgreSQL (kv_storage, vector_storage, graph_storage), task_storage uses in-memory.

### 2. PostgresTaskStorage Already Exists

File: `edgequake/crates/edgequake-tasks/src/postgres.rs`

```rust
pub struct PostgresTaskStorage {
    pool: Arc<PgPool>,
}

impl PostgresTaskStorage {
    pub fn new(pool: PgPool) -> Self { ... }
    pub fn from_arc(pool: Arc<PgPool>) -> Self { ... }
}

#[async_trait::async_trait]
impl TaskStorage for PostgresTaskStorage {
    async fn create_task(&self, task: &Task) -> TaskResult<()> { ... }
    async fn get_task(&self, track_id: &str) -> TaskResult<Option<Task>> { ... }
    async fn update_task(&self, task: &Task) -> TaskResult<()> { ... }
    // ... full implementation
}
```

### 3. Tasks Table Migration Exists

File: `edgequake/migrations/002_add_tasks_table.sql`

The database table is already set up with proper schema for storing tasks.

### 4. Cancel Issue Impact

Previous session fixed cancel to update document status when task not found, but the root cause is that tasks should persist across restarts.

When using MemoryTaskStorage:

- Backend restarts → all tasks lost
- Document metadata persists (PostgreSQL KV)
- Mismatch: document says "processing" but no task exists

When using PostgresTaskStorage:

- Backend restarts → tasks still available
- Cancel can find and update both task and document
- Consistent state

## Current File Analysis

### state.rs Location

Line 793 in `new_postgres()`:

```
edgequake/crates/edgequake-api/src/state.rs
```

### Pool Already Available

Line 679: `let pool = sqlx::postgres::PgPoolOptions::new()...`

The PostgreSQL pool is already created and available at the point where task_storage is initialized. We just need to use it.

## System State

| Component        | Storage Type | Persists on Restart |
| ---------------- | ------------ | ------------------- |
| KV Storage       | PostgreSQL   | ✅ Yes              |
| Vector Storage   | PostgreSQL   | ✅ Yes              |
| Graph Storage    | PostgreSQL   | ✅ Yes              |
| PDF Storage      | PostgreSQL   | ✅ Yes              |
| **Task Storage** | **Memory**   | **❌ No**           |

## Required Change

Replace line 793:

```rust
// FROM:
let task_storage = Arc::new(edgequake_tasks::memory::MemoryTaskStorage::new());

// TO:
let task_storage: SharedTaskStorage = Arc::new(
    edgequake_tasks::postgres::PostgresTaskStorage::from_arc(Arc::clone(&pool.clone().into()))
);
```

Or more cleanly using the pool already in scope:

```rust
let task_storage: SharedTaskStorage = Arc::new(
    edgequake_tasks::postgres::PostgresTaskStorage::new(pool.clone())
);
```
