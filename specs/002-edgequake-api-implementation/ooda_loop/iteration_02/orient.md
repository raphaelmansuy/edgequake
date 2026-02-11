# Iteration 02 — ORIENT: Test Coverage Strategy

## Root Cause Analysis

The 47% coverage stems from iteration_01 focusing on "build it" rather than "test it". Resources, streaming, and transport internals were scaffolded but only smoke-tested at the client construction level. To hit >90%, we need to test every resource method against a mock transport.

## Solution Approaches

### Approach A: Mock Transport + Direct Resource Tests (CHOSEN)

- Create `createMockTransport()` matching design spec pattern
- Test each resource by constructing with mock transport directly
- Fast, no HTTP server needed, tests every path/method mapping
- **Pros**: Fast (no I/O), tests exact request shapes, covers all methods
- **Cons**: Doesn't test real HTTP layer

### Approach B: MSW Integration Tests (DEFERRED to iter_03)

- Use Mock Service Worker for realistic HTTP-layer testing
- Tests SSE streaming, error responses, timeouts
- DEFERRED because it adds a dependency and is slower

### Approach C: E2E with Real Server (DEFERRED to iter_04+)

- Requires `make dev` running
- Tests real API behavior
- DEFERRED to later iteration

## Key Design Decision

Add `_transport?: HttpTransport` escape hatch to `EdgeQuakeConfig` so integration tests can inject a mock or custom transport. This matches the design spec pattern `new EdgeQuake({ _transport: mockTransport })`.

## First Principles

- Each resource method is a thin wrapper over base \_get/\_post/\_put/\_patch/\_del
- Testing every resource method via mock transport will cover both the resource AND the base class
- Streaming tests need mock ReadableStream and AsyncIterator
- The transport layer's middleware stack is already well-tested (16 tests)
