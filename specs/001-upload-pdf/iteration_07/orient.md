# OODA-07: Orient

## Gap Analysis

| Current State | Desired State | Gap | Priority |
|--------------|---------------|-----|----------|
| `extract_to_markdown()` called without progress | `extract_to_markdown_with_progress()` with callback | Need adapter | HIGH |
| `PipelineEvent` lacks PDF event | `PdfPageProgress` event in pipeline_state.rs | Add event | HIGH |
| No `emit_pdf_page_progress()` method | Method on PipelineState | Add method | HIGH |
| Callback closures need pdf_id/task_id | Capture in closure | Design adapter | HIGH |

## Risk Assessment

- **Risk 1**: Two event systems (PipelineEvent vs ProgressEvent) - Mitigation: Use PipelineState as single source, forward to ProgressBroadcaster
- **Risk 2**: Circular dependency (api→pdf→tasks→api) - Mitigation: Keep adapter in edgequake-api, use Arc closures

## First Principles Analysis

- **Core problem**: Need to bridge edgequake-pdf's ProgressCallback to WebSocket events
- **Fundamental constraint**: edgequake-pdf cannot depend on edgequake-api (circular)
- **Minimal solution**: Closure-based adapter capturing PipelineState + ids
- **Why this matters**: Users need real-time page-by-page extraction feedback

## Alternative Approaches

1. **Option A: Add PdfPageProgress to PipelineEvent (edgequake-tasks)**
   - Pros: Single event system, consistent with ChunkProgress
   - Cons: edgequake-api must translate to websocket_types::ProgressEvent

2. **Option B: Closure-based adapter in processor.rs**
   - Pros: Simple, no new struct needed
   - Cons: Harder to test, less reusable

3. **Option C: BroadcastingProgressCallback struct in edgequake-api**
   - Pros: Reusable, testable, explicit
   - Cons: More code, but cleaner architecture

**Chosen: Option A + C combined**
- Add PdfPageProgress to PipelineEvent (consistent with existing events)
- Create BroadcastingProgressCallback adapter struct for testability
