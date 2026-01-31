# OODA-10: Orient

## Gap Analysis

| Current State                               | Desired State                    | Gap                                      | Priority |
| ------------------------------------------- | -------------------------------- | ---------------------------------------- | -------- |
| PipelineState emits PipelineEvent           | WebSocket receives ProgressEvent | Need bridge                              | HIGH     |
| Two separate broadcast channels             | Unified event flow               | Forward events                           | HIGH     |
| WebSocket subscribes to ProgressBroadcaster | Should also receive PDF events   | Wire PipelineState → ProgressBroadcaster | HIGH     |

## Two Event Systems Discovered

### 1. PipelineState (edgequake-tasks)

- `PipelineEvent` enum: Log, Progress, StateChange, ChunkProgress, ChunkFailure, **PdfPageProgress**
- Broadcast via `tokio::sync::broadcast<PipelineEvent>`
- Used internally for pipeline coordination

### 2. ProgressBroadcaster (edgequake-api)

- `ProgressEvent` enum: Connected, JobStarted, DocumentProgress, etc., **PdfPageProgress**
- Broadcast via `tokio::sync::broadcast<ProgressEvent>`
- Used for WebSocket client updates

## Risk Assessment

- **Risk 1**: Duplicated event types in two systems - Mitigation: Consider unifying or bridging
- **Risk 2**: Event translation complexity - Mitigation: Simple 1:1 mapping for now

## First Principles Analysis

- **Core problem**: PDF page events go to PipelineState but WebSocket reads ProgressBroadcaster
- **Fundamental constraint**: Can't easily merge crates (different responsibilities)
- **Minimal solution**: Forward PipelineEvent::PdfPageProgress to ProgressBroadcaster
- **Why this matters**: Users won't see PDF progress without this bridge

## Alternative Approaches

1. **Option A: Modify PipelineProgressCallback to use ProgressBroadcaster directly**
   - Pros: Skip PipelineState for PDF events
   - Cons: Inconsistent with other events

2. **Option B: Create a forwarding task that subscribes to PipelineState and sends to ProgressBroadcaster**
   - Pros: Clean separation, can translate events
   - Cons: Extra async task

3. **Option C: Have processor.rs call progress_broadcaster.send() directly**
   - Pros: Direct, no extra plumbing
   - Cons: processor.rs would need ProgressBroadcaster access

**Chosen: Option C** - Most direct, processor.rs already has state access
