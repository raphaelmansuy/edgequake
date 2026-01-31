# Iteration 01: Orient

## Mission Re-Read ✅

- [x] Re-read mission file at `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [x] Confirmed: Need 6 pipeline phases with real-time progress

## Gap Analysis

| Current State                                    | Desired State                                                                           | Gap                                     | Priority |
| ------------------------------------------------ | --------------------------------------------------------------------------------------- | --------------------------------------- | -------- |
| PDF processing is single "processing" status     | 6 distinct phases: Upload, PdfConversion, Chunking, Embedding, Extraction, GraphStorage | No phase enum or tracking               | HIGH     |
| No progress callbacks in PdfExtractor            | `ProgressCallback` trait with `on_page_start`, `on_page_complete`                       | Need to add trait + implementation      | HIGH     |
| Task progress only tracks chunks (ChunkProgress) | Track all 6 phases with current/total/percentage                                        | Need PhaseProgress struct               | HIGH     |
| Generic "Processing..." in UI                    | Timeline showing each phase with progress bars                                          | Need `<PipelinePhase />` component      | HIGH     |
| Polling every 2s for updates                     | Real-time WebSocket updates < 500ms                                                     | WebSocket exists but needs phase events | MEDIUM   |
| No page-by-page progress for PDF                 | Show "Page 5/10" during extraction                                                      | Need page-level callbacks               | MEDIUM   |
| No ETA for PDF extraction                        | Estimate based on avg page time                                                         | Need timing + calculation               | MEDIUM   |
| No error recovery UI                             | Retry button per phase, error details                                                   | Need retry endpoint + UI                | MEDIUM   |
| No upload history                                | Persistent history with filter/search                                                   | Need storage + UI components            | LOW      |

## Risk Assessment

- **Risk 1: Breaking existing ingestion flow**
  - Mitigation: Keep existing ChunkProgress, ADD PhaseProgress alongside
  - Mitigation: Feature flag for new progress system

- **Risk 2: Performance overhead from frequent progress updates**
  - Mitigation: Throttle updates (every 1% or 500ms, whichever is longer)
  - Mitigation: Use batch updates instead of per-page

- **Risk 3: Callback complexity in async Rust**
  - Mitigation: Use `tokio::sync::mpsc` channel for progress events
  - Mitigation: Trait object with `Arc<dyn ProgressCallback>`

- **Risk 4: WebSocket message format breaking changes**
  - Mitigation: Add new message type, don't modify existing
  - Mitigation: Version the WebSocket protocol

## First Principles Analysis

### Core Problem

The user cannot see what's happening during PDF processing. They see "Processing..." for 30 seconds to 5 minutes with no visibility into progress or issues.

### Fundamental Constraints

1. **PDF extraction is CPU-bound** - Can't parallelize page processing easily
2. **LLM calls have variable latency** - 100ms to 30s per call
3. **Task processor is separate from HTTP handler** - Need communication channel
4. **Frontend polls or uses WebSocket** - Already have this infrastructure

### Minimal Solution

1. Add `PipelinePhase` enum with 6 phases
2. Add `PhaseProgress` struct to Task
3. Modify `process_pdf_processing()` to update phases
4. Emit WebSocket events on phase changes
5. Add `<PipelinePhase />` component to display phases

### Why This Matters (Business Impact)

- **User trust**: Seeing progress builds confidence the system is working
- **Error detection**: Users can identify stuck/failed uploads faster
- **Debugging**: Support can pinpoint exactly where issues occur
- **UX quality**: Professional apps show detailed progress

## Alternative Approaches

### Option A: Polling with Phase State in Task

- **Description**: Store phase progress in Task.progress, frontend polls
- **Pros**: Simple, works with existing infrastructure
- **Cons**: Up to 2s delay, more DB reads
- **Verdict**: Good fallback, but not real-time

### Option B: WebSocket with Progress Events

- **Description**: Emit events on each phase change via existing WebSocket
- **Pros**: Real-time, < 500ms latency, less DB load
- **Cons**: More complex, need to handle reconnections
- **Verdict**: ✅ PREFERRED - Meets latency requirement

### Option C: Server-Sent Events (SSE)

- **Description**: Use SSE instead of WebSocket
- **Pros**: Simpler than WebSocket, HTTP-based
- **Cons**: One-way only, would need to add new infrastructure
- **Verdict**: Not worth adding when WebSocket exists

### Option D: GraphQL Subscriptions

- **Description**: Use GraphQL subscriptions for real-time updates
- **Pros**: Type-safe, schema-driven
- **Cons**: Would need to add GraphQL layer
- **Verdict**: Over-engineering for this use case

## Architecture Decision

**Selected: Option A + B Hybrid**

Store phase state in Task (for persistence and polling fallback), AND emit WebSocket events for real-time updates. This provides:

- Durability: Progress survives reconnects
- Real-time: WebSocket for < 500ms updates
- Fallback: Polling when WebSocket unavailable

## Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            PDF UPLOAD PROGRESS FLOW                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                  │
│  │   Frontend   │    │  WebSocket   │    │   Backend    │                  │
│  │   (React)    │◄──►│   Server     │◄──►│  (Axum)      │                  │
│  └──────┬───────┘    └──────────────┘    └──────┬───────┘                  │
│         │                                        │                          │
│         │ 1. Upload PDF                          │                          │
│         │───────────────────────────────────────►│                          │
│         │                                        │ 2. Create Task           │
│         │                                        │    (track_id)            │
│         │◄──────── 3. Return task_id ────────────│                          │
│         │                                        │                          │
│         │ 4. Subscribe WS(track_id)              │                          │
│         │───────────────────────────────────────►│                          │
│         │                                        │                          │
│         │              ┌─────────────────────────┤                          │
│         │              │     TaskProcessor       │                          │
│         │              │                         │                          │
│         │              │ Phase 1: Upload ✅      │                          │
│         │◄─────────────│ → emit(Upload, 100%)   │                          │
│         │              │                         │                          │
│         │              │ Phase 2: PdfConversion  │                          │
│         │◄─────────────│ → emit(PdfConv, 30%)   │  ┌─────────────────┐     │
│         │              │ → on_page_complete(3/10)│──│ edgequake-pdf   │     │
│         │◄─────────────│ → emit(PdfConv, 70%)   │  │ (ProgressCB)    │     │
│         │              │                         │  └─────────────────┘     │
│         │              │ Phase 3: Chunking       │                          │
│         │◄─────────────│ → emit(Chunking, 100%) │                          │
│         │              │                         │                          │
│         │              │ Phase 4: Embedding      │                          │
│         │◄─────────────│ → emit(Embedding, 50%) │                          │
│         │              │                         │                          │
│         │              │ Phase 5: Extraction     │                          │
│         │◄─────────────│ → emit(Extraction, 80%)│                          │
│         │              │                         │                          │
│         │              │ Phase 6: GraphStorage   │                          │
│         │◄─────────────│ → emit(Storage, 100%)  │                          │
│         │              │                         │                          │
│         │◄─────────────│ → emit(Complete!)      │                          │
│         │              └─────────────────────────┘                          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```
