# Iteration 06: Orient

## Gap Analysis

| Current State                           | Desired State                           | Gap                | Priority |
| --------------------------------------- | --------------------------------------- | ------------------ | -------- |
| No PdfPageProgress event                | Event for page-level progress           | Add variant        | HIGH     |
| processor.rs uses extract_to_markdown() | Use extract_to_markdown_with_progress() | Integrate callback | HIGH     |
| No ProgressCallback→WebSocket bridge    | BroadcastingProgressCallback            | Create adapter     | HIGH     |

## Risk Assessment

- **Risk 1**: Breaking WebSocket clients - Mitigation: New event type is additive
- **Risk 2**: Performance impact - Mitigation: Broadcast channel is fast (< 1ms per send)
- **Risk 3**: Thread safety - Mitigation: Broadcast already uses Arc + mutex internally

## First Principles Analysis

- **Core problem**: PDF extraction progress doesn't reach WebSocket clients
- **Fundamental constraint**: We need to bridge ProgressCallback trait to broadcast channel
- **Minimal solution**: Create adapter struct that implements ProgressCallback
- **Why this matters**: Users need page-by-page feedback for large PDFs (30+ pages)

## Alternative Approaches

1. **Option A: Add PdfPageProgress event + BroadcastingProgressCallback** ✅ CHOSEN
   - Pros: Clean separation, specific event type
   - Cons: More code

2. **Option B: Reuse DocumentProgress event**
   - Pros: Less code
   - Cons: Overloads meaning of DocumentProgress

## Implementation Plan (split across OODA-06, 07, 08)

**OODA-06**: Add `PdfPageProgress` event to websocket_types.rs
**OODA-07**: Create `BroadcastingProgressCallback` in processor.rs
**OODA-08**: Integrate callback into PDF processing flow
