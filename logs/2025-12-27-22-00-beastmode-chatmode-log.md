# Task Log: Query Architecture Audit

**Date**: 2025-12-27-22-00
**Mode**: beastmode-chatmode

---

## Actions

- Audited frontend query-interface.tsx (959 lines) for message persistence logic
- Audited backend query.rs (492 lines) for streaming endpoint implementation
- Audited backend conversations.rs (1113 lines) for message API handlers
- Identified critical architectural flaw: client-side message persistence
- Created comprehensive audit document: plan_improve_query_page/11_architecture_audit.md (~450 lines)
- Updated plan_improve_query_page/scratchpad.md with session findings
- Updated plan_improve_query_page/plan.md with new document reference and action log

## Decisions

- Server MUST be responsible for message persistence, not client
- Proposed unified `/api/v1/chat/completions` endpoint following OpenAI pattern
- User message saved BEFORE LLM call, assistant message saved AFTER streaming
- SSE events should include message IDs for client synchronization

## Key Findings

### Critical Issue: Client-Side Persistence

The streaming query endpoint (`/api/v1/query/stream`) does NOT persist messages. The frontend is responsible for calling a separate API to save the assistant's response after streaming. This violates single-source-of-truth and creates data loss risk.

### Evidence

- **Frontend** (query-interface.tsx lines 475-490): Manual `createMessage()` call after streaming
- **Backend** (query.rs lines 360-427): `stream_query()` has NO conversation service calls

### Proposed Solution

Create unified `/api/v1/chat/completions` endpoint:

1. Accepts optional `conversation_id` (creates new if null)
2. Saves user message atomically BEFORE query
3. Streams tokens via SSE
4. Saves assistant message atomically AFTER stream completes
5. Returns message IDs in final `done` event

## Next Steps

1. Implement `handlers/chat.rs` with unified endpoint
2. Create `lib/api/chat.ts` frontend client
3. Update query-interface.tsx to use new endpoint
4. Remove manual message saving logic from frontend
5. Add E2E tests for persistence verification

## Lessons/Insights

- React hook closures cause stale state issues with mutations initialized at component mount
- Client-side persistence is fundamentally flawed for streaming scenarios
- OpenAI's chat completions API pattern is the correct model to follow
- Server-side persistence provides transactional guarantees client cannot
