# Iteration 02 — OBSERVE: Test Coverage Gap Analysis

## Current State

- **Line coverage**: 47.34% (target: >90%)
- **Function coverage**: 26.15% (target: >90%)
- **Branch coverage**: 92.3% (above target)
- **Statement coverage**: 47.34% (target: >90%)

## Coverage Breakdown by Module

| Module         | Lines  | Functions | Gap              |
| -------------- | ------ | --------- | ---------------- |
| src/ (core)    | 94.77% | 80.64%    | Almost there     |
| src/resources/ | 31.24% | 5.67%     | **Critical gap** |
| src/streaming/ | 0%     | 100%      | **Untested**     |
| src/transport/ | 58.78% | 76.19%    | Needs work       |

## Key Findings

1. **Resources are the largest gap** — 21 resource files, each with 3-15 methods, almost none called in tests. The methods are simple wrappers over `_get/_post/_put/_patch/_del` but need coverage.

2. **Streaming untested** — `sse.ts` (SSE parser) and `websocket.ts` (WebSocket wrapper) have 0% line coverage.

3. **Transport fetch.ts gaps** — Core methods `request()`, `stream()`, `upload()`, `uploadBatch()`, `requestBlob()` have paths untested (error handling, retries, actual fetch calls).

4. **Base resource** — `_streamSSE()`, `streamToAsyncIterator()`, `normalizeQuery()` only 13.84% covered.

5. **Client constructor** — Doesn't accept mock transport for testing. Design spec shows `{ _transport: mockTransport }` pattern.

## Design Spec Testing Patterns (from 10-testing-strategy.md)

- `createMockTransport()` helper mapping `"METHOD /path"` → response objects
- `mockTransport.lastRequest` for request verification
- Direct resource testing via mock transport constructor injection
- MSW (Mock Service Worker) for integration tests
