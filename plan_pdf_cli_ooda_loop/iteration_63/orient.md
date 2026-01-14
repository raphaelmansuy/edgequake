# OODA Iteration 63 - Orient Phase

## Strategic Context

### Problem Domain
The document processing pipeline can take significant time for large documents or complex extractions. Users currently have no way to stop this process once started, leading to:
- Wasted compute resources
- Blocked pipeline slots
- Poor user experience
- Inability to correct mistakes

### Technical Landscape

#### Backend Architecture (Ready)
```rust
// /tasks/{track_id}/cancel endpoint
pub async fn cancel_task(State(state), Path(track_id)) {
    // Check if cancellable (not Indexed or Cancelled)
    task.mark_cancelled();
    state.task_storage.update_task(&task).await
}
```

#### Frontend Gap Analysis
| Component | Current State | Required |
|-----------|--------------|----------|
| StatusConfig | 5 statuses | + cancelled |
| Dropdown Menu | Reprocess, Delete | + Cancel Extraction |
| cancelTask API | Exists, unused | Wire to UI |
| Visual Feedback | No cancel icon | StopCircle icon |

### Solution Options

#### Option A: Minimal Cancel Button (Selected)
- Add "Cancel Extraction" to dropdown for pending/processing docs
- Add `cancelled` status to statusConfig
- Low complexity, high value

#### Option B: Bulk Cancel
- Add bulk cancel for selected documents
- More complex UI changes
- Defer to future iteration

#### Option C: Cancel in Progress Dialog
- Add cancel to pipeline-status-dialog
- Already has Cancel button (added in OODA 62)
- Already implemented

### Decision Matrix

| Criterion | Option A | Option B | Option C |
|-----------|----------|----------|----------|
| Implementation time | 30 min | 2 hours | Done |
| User value | High | Medium | High |
| Risk | Low | Medium | Low |
| **Selected** | ✅ | ❌ | ✅ |

## Alignment with Requirements
- REQ-26: Stop extraction → Directly addressed
- Backend API exists → Leverage existing infrastructure
- Minimal code changes → Low risk
