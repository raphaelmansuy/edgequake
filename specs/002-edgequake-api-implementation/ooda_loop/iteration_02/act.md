# Iteration 02 — Act

## What Was Done

### Infrastructure

- Added `_transport?: HttpTransport` to `EdgeQuakeConfig` for test injection
- Updated `EdgeQuake` constructor to use injected transport when provided
- Created `tests/helpers/mock-transport.ts` with `createMockTransport()` helper

### Test Files Created

| File                                 | Tests | Purpose                                                                               |
| ------------------------------------ | ----- | ------------------------------------------------------------------------------------- |
| `tests/unit/resources.test.ts`       | 126   | All 21 resources + sub-resources + client injection                                   |
| `tests/unit/base-resource.test.ts`   | 12    | BaseResource protected methods (\_get, \_post, \_put, \_patch, \_del, \_streamSSE)    |
| `tests/unit/streaming.test.ts`       | 8     | SSE parser (parseSSEStream) edge cases                                                |
| `tests/unit/fetch-transport.test.ts` | 23    | FetchTransport with mock fetchFn (request, stream, upload, blob, middleware, signals) |
| `tests/unit/websocket.test.ts`       | 6     | EdgeQuakeWebSocket async iterable (messages, close, error, buffer, malformed)         |
| `tests/unit/retry.test.ts`           | 8     | Retry middleware (429/503 retry, AbortError bypass, exhaustion, network errors)       |
| `tests/unit/barrel.test.ts`          | 1     | resources/index.ts barrel export coverage                                             |

### Coverage Results

| Metric     | Before | After      | Target  |
| ---------- | ------ | ---------- | ------- |
| Lines      | 47.34% | **98.52%** | >90% ✅ |
| Functions  | 26.15% | **97.02%** | >90% ✅ |
| Branches   | 92.30% | **85.43%** | >80% ✅ |
| Statements | 47.34% | **98.52%** | >90% ✅ |

### Test Count

- **Before iteration 02:** 59 tests (6 files)
- **After iteration 02:** 243 tests (12 files)
- **Net new tests:** +184

### Key Debugging Insights

Multiple path mismatches discovered between test expectations and actual resource implementations:

- Ollama uses `/api/version`, `/api/tags` etc. (not `/api/v1/ollama/...`)
- Lineage uses `/api/v1/lineage/entities/` and `/api/v1/lineage/documents/`
- Provenance uses `/api/v1/entities/{id}/provenance`
- Settings uses singular `/api/v1/settings/provider/status`
- Models uses `/api/v1/models/{provider}` not `.../providers/{provider}`
- Workspaces uses `/metrics-history` (hyphenated, not `/metrics/history`)
- `messages.update()` takes `(messageId, request)` — not `(convId, msgId, request)`

## Outcome

All 243 tests pass. Coverage exceeds 90% on all metrics except branches (85.43%, acceptable given pure-type files report 0%). Ready for IMPL-02 commit.
