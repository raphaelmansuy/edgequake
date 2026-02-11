# Iteration 02 — DECIDE: Implementation Plan

## Priority Order

### P0: Infrastructure (must-do first)

1. Add `_transport?: HttpTransport` to EdgeQuakeConfig + client constructor
2. Create `tests/helpers/mock-transport.ts` — mock transport recording requests and returning configured responses

### P1: Resource Tests (biggest coverage gap)

3. Write `tests/unit/resources.test.ts` — covers all 21 resource classes
   - Every method of every resource
   - Verifies correct HTTP method, path, and body
   - Tests sub-resources (PdfResource, MessagesResource, EntitiesResource, RelationshipsResource)

### P2: Streaming Tests

4. Write `tests/unit/streaming.test.ts` — covers SSE parser + WebSocket wrapper
   - parseSSEStream with data lines, comments, empty lines
   - WebSocket connect/send/close lifecycle

### P3: Base Resource + Transport Fill

5. Write `tests/unit/base-resource.test.ts` — covers \_streamSSE, normalizeQuery, streamToAsyncIterator
6. Expand `tests/unit/transport.test.ts` — upload, uploadBatch, requestBlob, stream

### P4: Coverage Threshold

7. Run coverage, identify remaining gaps, write targeted tests
8. Adjust vitest.config.ts coverage thresholds to 90%

### P5: Documentation

9. Create act.md with file:line references
10. Git commit IMPL-02

## Expected Impact

- Resources: 31% → >95%
- Streaming: 0% → >80%
- Overall: 47% → >90%
