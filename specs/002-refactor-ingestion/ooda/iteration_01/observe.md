# OODA Iteration 01 - Observe

**Mission Re-read**: ✅ `specs/002-refactor-ingestion.md`
**Focus**: Critical Issue #1 - Race Condition in Re-ingestion
**Date**: 2026-02-08

---

## Observations

### 1. Race Condition Location

**File**: `edgequake/crates/edgequake-api/src/handlers/documents.rs`
**Lines**: 489-555

```rust
async fn delete_document_for_reingestion(
    document_id: &str,
    state: &AppState,
    workspace_id: &str,
) -> Result<bool, ApiError> {
    // STEP 1: Read status (line 495-503)
    let metadata_key = format!("{}-metadata", document_id);
    let status = if let Ok(Some(metadata)) = state.kv_storage.get_by_id(&metadata_key).await {
        metadata.get("status").and_then(|v| v.as_str()).map(|s| s.to_string())
    } else {
        "unknown".to_string()
    };

    // STEP 2: Check status (line 507-512)
    if status == "pending" || status == "processing" {
        return Ok(false);
    }

    // ⚠️ RACE WINDOW: Another request can change status here!

    // STEP 3: Delete data (lines 520-550)
    cleanup_document_graph_data(...).await?;
    state.kv_storage.delete(&keys_to_delete).await?;
}
```

**TOCTOU Vulnerability**: Time-of-Check (status read) differs from Time-of-Use (data deletion).

### 2. Concurrent Operations That Can Race

Found 3 code paths that modify document status:

| Code Path              | File:Line         | Status Transitions     |
| ---------------------- | ----------------- | ---------------------- |
| `spawn_ingestion_task` | documents.rs:2100 | pending → processing   |
| `complete_ingestion`   | documents.rs:2300 | processing → completed |
| `fail_ingestion`       | documents.rs:2400 | processing → failed    |

### 3. Existing Locking Mechanisms

**Found**: `tokio::sync::RwLock` used in `progress.rs:359` for in-memory progress state.

**NOT Found**:

- No PostgreSQL advisory locks (`pg_advisory_lock`)
- No distributed lock service (Redis, etcd)
- No optimistic locking (version columns)

### 4. KV Storage Architecture

**File**: `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs`

```
┌─────────────────────────────────────────────────────────────────┐
│                    PostgreSQL KV Storage                        │
├─────────────────────────────────────────────────────────────────┤
│  Table: eq_{prefix}_kv                                          │
│  ├─ key TEXT PRIMARY KEY                                        │
│  ├─ value JSONB NOT NULL                                        │
│  ├─ created_at TIMESTAMPTZ                                      │
│  └─ updated_at TIMESTAMPTZ                                      │
├─────────────────────────────────────────────────────────────────┤
│  Operations:                                                    │
│  ├─ get_by_id(key) → Option<Value>                              │
│  ├─ set(key, value) → upsert                                    │
│  └─ delete(keys) → batch delete                                 │
│                                                                 │
│  ⚠️ NO atomic read-modify-write!                                │
│  ⚠️ NO row-level locking!                                       │
└─────────────────────────────────────────────────────────────────┘
```

### 5. Race Condition Scenario

```
Timeline:
─────────────────────────────────────────────────────────────────────────
Process A (Re-ingest)                  Process B (Ingestion Pipeline)
─────────────────────────────────────────────────────────────────────────
T1: Read status = "failed" ←──────────
                                       T2: Start processing, status → "processing"
T3: Check status (stale!)
    status == "failed" ✓
                                       T4: Extract entities, write graph data
T5: DELETE graph data ←─────────────── 💥 CORRUPTED!
                                       T6: Write more entities
                                           (some orphaned, some re-created)
─────────────────────────────────────────────────────────────────────────
Result: Graph has a mix of old and new data, inconsistent state
```

### 6. Potential Solutions (For Orient Phase)

| Solution                         | Pros                 | Cons                        | Complexity |
| -------------------------------- | -------------------- | --------------------------- | ---------- |
| **PostgreSQL Advisory Locks**    | Built-in, reliable   | Single-node PostgreSQL only | Medium     |
| **Optimistic Locking (version)** | Portable, standard   | Retry logic needed          | Medium     |
| **KV Storage Lock Service**      | Abstracted, testable | New code, tests             | High       |
| **Redis distributed lock**       | Proven pattern       | New dependency              | High       |

### 7. Test Coverage Check

```bash
# Check for existing concurrency tests
grep -r "race\|concurrent\|parallel" edgequake/crates/edgequake-api/tests/
```

**Result**: No explicit race condition tests found for document operations.

---

## Data Inventory

| Item                | Location                  | Observation                               |
| ------------------- | ------------------------- | ----------------------------------------- |
| Race condition code | `documents.rs:489-555`    | TOCTOU vulnerability confirmed            |
| Status transitions  | 3 code paths              | `pending → processing → completed/failed` |
| Existing locks      | `progress.rs:359`         | In-memory only, not for documents         |
| KV storage          | `adapters/postgres/kv.rs` | No atomic CAS operations                  |
| Concurrency tests   | None                      | Missing coverage                          |

---

## Next: Orient Phase

Analyze solutions using First Principles:

1. What is the simplest atomic operation that prevents races?
2. Can PostgreSQL features be leveraged without new dependencies?
3. How to maintain backward compatibility with memory storage?
