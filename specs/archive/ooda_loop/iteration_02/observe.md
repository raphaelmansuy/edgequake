# Iteration 02: Observe

**Mission Re-read**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-improve-ingestion-process.md`

---

## Territory Mapping: Backend Status Updates

### 1. Current Status Update Flow

From processor.rs analysis:

```
Document Processing Pipeline Status Flow:
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                          │
│  [Task Start]  →  [status: processing]                                   │
│       │                                                                  │
│       ▼                                                                  │
│  task.update_progress("chunking", 4, 10)  ← Progress counter only        │
│       │                                                                  │
│       ▼                                                                  │
│  self.pipeline_state.info("Chunking...")  ← Log message only             │
│       │                                                                  │
│       ▼                                                                  │
│  task.update_progress("embedding", 4, 30)                                │
│       │                                                                  │
│       ▼                                                                  │
│  task.update_progress("indexing", 4, 100)                                │
│       │                                                                  │
│       ▼                                                                  │
│  [status: completed] or [status: failed + error_message]                 │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2. Key Observations

#### processor.rs (Lines 598-970)

- `update_document_status()` only sets final states: `processing`, `completed`, `failed`
- `task.update_progress()` sets progress stage name but NOT in document metadata
- Progress stage is stored in Task struct, not in KV document metadata
- Frontend reads document metadata, not task progress

**GAP**: Document metadata doesn't contain sub-state (chunking/extracting/embedding/indexing)

### 3. Task Progress vs Document Metadata

```rust
// Task progress (processor.rs:600)
task.update_progress("chunking".to_string(), 4, 10);

// Document metadata (processor.rs:994)
async fn update_document_status(&self, document_id: &str, status: &str, ...) {
    updated.insert("status".to_string(), json!(status));  // Only final status
}
```

### 4. Where Sub-States Should Be Updated

| Stage      | Location in processor.rs     | Current Action         | Needed Action               |
| ---------- | ---------------------------- | ---------------------- | --------------------------- |
| chunking   | Line 598                     | `task.update_progress` | Also update document status |
| extracting | Line 621 (after chunk store) | Log message            | Update document status      |
| embedding  | Line 625                     | `task.update_progress` | Update document status      |
| indexing   | Line 948                     | `task.update_progress` | Update document status      |

### 5. API Response Analysis

Looking at list_documents handler (documents.rs:964-1250):

- Returns `status` field from document metadata
- Already has `error_message` field
- Just needs the backend to update `status` with sub-states

### 6. Frontend Polling

Document manager polls documents every 2-5 seconds:

- Uses `useQuery` with `refetchInterval`
- Already will pick up status changes automatically

---

## Files to Modify

| Priority | File             | Change                                                       |
| -------- | ---------------- | ------------------------------------------------------------ |
| P1       | processor.rs     | Update status in document metadata for each processing stage |
| P2       | documents.rs     | Verify status field is returned (already done)               |
| P0       | status-badge.tsx | Already done in iteration 01 ✅                              |

---

## Next Step

Proceed to **Orient** phase to design the status update pattern.
