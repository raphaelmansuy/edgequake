# SPEC-037 — Product Owner Lens

**Lens:** Product Owner  
**Persona:** Knowledge worker verifying RAG answers against source documents  
**Evidence:** Screenshots + codebase audit (2026-07-01)

---

## Problem Statement

Users running queries in EdgeQuake cannot:

1. **Access System Prompt** in Query Settings — panel is clipped on standard laptop viewports.
2. **Read full retrieved passages** in stream results — snippets cut mid-word, undermining trust in retrieval quality.

Both issues block the core value proposition: **answer with verifiable provenance**.

---

## User Stories

### US-037-01 — Scrollable Query Settings

> **As a** power user configuring RAG behavior  
> **I want** to scroll through all Query Settings sections  
> **So that** I can set System Prompt and generation parameters without resizing the browser

**Acceptance Criteria:**

| AC | Given | When | Then |
| -- | ----- | ---- | ---- |
| AC1 | Query Settings open, viewport 768×1024 | User scrolls down | System Prompt textarea is visible and focusable |
| AC2 | Content exceeds viewport | User scrolls | Scroll shadow indicators appear (top/bottom) |
| AC3 | Settings open | User closes sheet | Scroll position resets on next open (optional, nice-to-have) |

### US-037-02 — Full Passage Text Toggle

> **As a** researcher validating retrieval  
> **I want** an option to show full passage text in query results  
> **So that** I can verify the model retrieved the right context without opening the document viewer

**Acceptance Criteria:**

| AC | Given | When | Then |
| -- | ----- | ---- | ---- |
| AC4 | Toggle OFF (default) | Stream query completes | Passages show ≤200 char snippets (current behavior) |
| AC5 | Toggle ON | Stream query completes | Passages show full chunk content from API |
| AC6 | Toggle ON | User refreshes page | Setting persists (localStorage via settings store) |
| AC7 | Toggle ON | Mobile viewport | Passages scroll/wrap; no horizontal overflow |

### US-037-03 — API Parity

> **As an** API integrator  
> **I want** `content_granularity` on `/query/stream` and `/chat/completions`  
> **So that** my agent can request full chunks without a separate `/query/context` call

**Acceptance Criteria:**

| AC | Given | When | Then |
| -- | ----- | ---- | ---- |
| AC8 | `content_granularity: "agent"` in POST body | Stream context event | `sources[].snippet` equals full chunk text |
| AC9 | Field omitted | Stream context event | Defaults to `citation` (200 chars) — no breaking change |
| AC10 | `content_granularity: "debug"` + non-admin JWT | Request | 403 Forbidden (reuse EC-MCP-29 policy) |

---

## Business Value

| Metric | Before | After |
| ------ | ------ | ----- |
| Settings completion rate | Users abandon at Generation (System Prompt unreachable) | Full configuration possible |
| Citation click-through | High (users must open doc to read context) | Lower when full text enabled |
| Support tickets "wrong snippet" | Common | Reduced — users self-verify |
| API surface consistency | `/query/context` has granularity; stream does not | Unified contract |

---

## Prioritization

| Priority | Item | Rationale |
| -------- | ---- | --------- |
| P0 | Scroll fix | Zero API risk; unblocks System Prompt (SPEC-004) |
| P0 | API `content_granularity` on stream | Code is law — UI cannot fix alone |
| P1 | Settings toggle + wire to chat stream | Primary user path (`use-query-streaming.ts`) |
| P1 | Citation UI respects full mode | Avoid double truncation |
| P2 | OpenAPI + SDK type updates | Integrator parity |
| P3 | Playwright scroll + granularity E2E | Regression guard |

---

## Out of Scope (this spec)

- Word-boundary snippet truncation algorithm
- Per-passage expand/collapse (future UX enhancement)
- Changing LLM context token budgets (`truncation.rs`)
- Exposing `debug` tier in Query Settings UI

---

## Success Metrics (30 days post-ship)

| KPI | Target |
| --- | ------ |
| Query Settings scroll E2E pass | 100% CI green |
| Default stream payload size | Unchanged (citation default) |
| Full-chunk adoption | Track via settings export (optional analytics) |
| Zero regressions on citation mode | Existing E2E stream tests pass |

---

## REQ Traceability

| REQ ID | User Story | Priority |
| ------ | ---------- | -------- |
| REQ-037-01 | US-037-01 AC1–AC2 | P0 |
| REQ-037-02 | US-037-02 AC4–AC7 | P1 |
| REQ-037-03 | US-037-02 AC6 | P1 |
| REQ-037-04 | US-037-03 AC8–AC10 | P0 |
| REQ-037-05 | US-037-03 AC8 | P0 |
| REQ-037-06 | US-037-02 AC5 | P1 |
| REQ-037-07 | US-037-03 AC9 | P0 |
| REQ-037-08 | US-037-03 AC10 | P0 |
