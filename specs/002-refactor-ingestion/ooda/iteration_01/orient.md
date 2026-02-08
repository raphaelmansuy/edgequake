# OODA Iteration 01 - Orient

**Mission Re-read**: ✅ `specs/002-refactor-ingestion.md`
**Focus**: Critical Issue #1 - Race Condition in Re-ingestion
**Date**: 2026-02-08

---

## First Principles Analysis

### The Core Problem

**Question**: What is the minimum requirement to prevent the race condition?

**Answer**: We need an **atomic compare-and-swap** operation:

1. Read current status
2. If status matches expected value, perform operation
3. If status changed, abort with conflict error

This is the fundamental primitive needed - all other solutions build on this.

---

## Solution Analysis

### Option A: PostgreSQL Advisory Locks

```sql
-- Acquire lock on document
SELECT pg_advisory_lock(hashtext($document_id));
-- Perform operations
-- Release lock
SELECT pg_advisory_unlock(hashtext($document_id));
```

**Pros:**

- Built into PostgreSQL
- No schema changes
- Familiar pattern

**Cons:**

- PostgreSQL-specific (no memory storage fallback)
- Session-level locking requires connection management
- Deadlock risk if multiple locks per transaction

**Risk Assessment**: 🟡 MEDIUM - Adds PostgreSQL dependency for locking

---

### Option B: Optimistic Locking with Version Column

```sql
-- Add version column to KV table
ALTER TABLE eq_kv ADD COLUMN version INTEGER DEFAULT 1;

-- Atomic update with version check
UPDATE eq_kv
SET value = $new_value, version = version + 1
WHERE key = $doc_id AND version = $expected_version;
-- Check affected rows: 0 = conflict, 1 = success
```

**Pros:**

- Standard pattern (ETags, HTTP preconditions)
- Works with any database
- Can implement in memory storage too

**Cons:**

- Schema migration required
- Retry logic for conflicts
- Slightly more complex code

**Risk Assessment**: 🟢 LOW - Standard pattern, well understood

---

### Option C: Status Transition Machine

Instead of arbitrary status checks, enforce state transitions:

```rust
enum DocumentStatus {
    Pending,      // Initial state
    Processing,   // In pipeline
    Completed,    // Success
    Failed,       // Error
    Deleting,     // Being cleaned up <-- NEW
}

// Allowed transitions:
// Pending → Processing (start ingestion)
// Processing → Completed | Failed (finish)
// Completed | Failed → Deleting (cleanup start)
// Deleting → (removed) (cleanup done)
```

**Pros:**

- Models the problem correctly
- Self-documenting code
- Easy to reason about

**Cons:**

- Still needs atomic transition
- More states to handle

**Risk Assessment**: 🟢 LOW - Good abstraction, combines with Option B

---

### Option D: Document Lock Service (Abstract)

```rust
#[async_trait]
pub trait DocumentLockService: Send + Sync {
    /// Acquire exclusive lock on document, returns guard that releases on drop
    async fn acquire(&self, document_id: &str, timeout: Duration) -> Result<DocumentLockGuard>;

    /// Try to acquire, immediately returns None if locked
    async fn try_acquire(&self, document_id: &str) -> Result<Option<DocumentLockGuard>>;
}

// Implementations:
struct PostgresDocumentLock { ... }  // Uses pg_advisory_lock
struct MemoryDocumentLock { ... }    // Uses DashMap<String, ()>
```

**Pros:**

- Clean abstraction
- Testable with memory implementation
- Feature-gated for PostgreSQL

**Cons:**

- New trait, implementation effort
- Additional dependency injection

**Risk Assessment**: 🟡 MEDIUM - Good design, significant code change

---

## Recommended Approach

**Combine Option B + Option C** (Optimistic Locking + State Machine)

```
┌─────────────────────────────────────────────────────────────────────┐
│                    RECOMMENDED SOLUTION                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  1. Add `version` column to KV storage (or use `updated_at`)        │
│                                                                     │
│  2. Add atomic status transition method:                            │
│     - transition_status(doc_id, from_status, to_status)             │
│     - Returns Ok(true) if transition succeeded                      │
│     - Returns Ok(false) if current status != from_status            │
│                                                                     │
│  3. Refactor delete_document_for_reingestion:                       │
│     - Try transition: "failed" → "deleting"                         │
│     - If fails: Return conflict error (document state changed)      │
│     - If succeeds: Perform cleanup safely                           │
│     - On completion: Delete document row entirely                   │
│                                                                     │
│  4. Add state machine enum for type safety                          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

Why this approach:

1. **First Principles**: Uses atomic operation at database level
2. **Portable**: Works with PostgreSQL and memory storage
3. **Minimal Changes**: No new services, just extend existing KV trait
4. **Type Safe**: State machine prevents invalid transitions

---

## Risk Analysis

| Risk                          | Probability | Impact | Mitigation                          |
| ----------------------------- | ----------- | ------ | ----------------------------------- |
| Schema migration fails        | Low         | High   | Test in staging first               |
| Optimistic conflict rate high | Low         | Low    | Add retry with backoff              |
| State machine incomplete      | Medium      | Medium | Review all status transitions       |
| Breaking existing API         | Low         | High   | Ensure backward compat in responses |

---

## Gap Analysis

| Current State           | Target State         | Gap                               |
| ----------------------- | -------------------- | --------------------------------- |
| Non-atomic status check | Atomic CAS operation | Add `transition_status` method    |
| String status           | Enum status          | Add `DocumentStatus` enum         |
| No version tracking     | Optimistic locking   | Add `version` or use `updated_at` |
| No conflict response    | HTTP 409 Conflict    | Add error variant                 |

---

## Implementation Checkpoint

Before proceeding to Decide phase:

- [x] Identified root cause (TOCTOU)
- [x] Evaluated 4 solutions
- [x] Selected approach (B + C)
- [x] Assessed risks
- [x] Mapped gaps

**Ready for Decide phase**: YES
