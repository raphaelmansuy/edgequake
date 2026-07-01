# SPEC-037 — Decision Matrix

**Purpose:** Evaluate implementation options for full-chunk stream flag + settings scroll  
**Method:** Weighted scoring + adversarial attacks  
**Decision date:** 2026-07-01

---

## Decision 1 — API Flag Shape

| Option | Description | Score /10 | Verdict |
| ------ | ----------- | --------- | ------- |
| **A — Reuse `content_granularity` enum** | Add SPEC-028 field to stream DTOs; UI maps boolean → `agent`/`citation` | **9** | ✅ **CHOSEN** |
| B — New `include_full_chunks: bool` | Parallel boolean in Rust + OpenAPI | 5 | ❌ DRY violation |
| C — Query param `?full=1` | Non-RESTful; inconsistent with POST body patterns | 3 | ❌ Rejected |
| D — Client-side fetch `/query/context` after stream | Second round-trip; race with answer | 4 | ❌ Rejected |

### Why A wins

- `context_bundle_mapper.rs` and `ContextRetrievalRequest` already implement tier logic
- MCP gateway documents same enum (`tools.rs:150`)
- One OpenAPI schema component reused
- UI boolean is a **presentation mapping**, not a second API contract

### Attack on A — "Boolean is simpler for users"

**Defense:** Users see a boolean toggle. API remains enum for agent/MCP parity. Mapping is one line in `use-query-streaming.ts`.

---

## Decision 2 — Default Granularity on Stream

| Option | Default | Score | Verdict |
| ------ | ------- | ----- | ------- |
| **A — `citation`** | 200-char snippets | **9** | ✅ **CHOSEN** |
| B — `agent` | Full chunks always | 4 | ❌ Payload regression |
| C — Smart auto (top_k < 5 → agent) | Heuristic | 5 | ❌ Surprising behavior |

### Attack on B — "Users want full text"

**Defense:** Opt-in toggle satisfies power users; default protects mobile and existing API clients (`spec028_context_e2e.rs` tests citation at 200 chars).

---

## Decision 3 — Settings Scroll Fix

| Option | Description | Score | Verdict |
| ------ | ----------- | ----- | ------- |
| **A — `min-h-0` + `overflow-hidden`** | Established pattern from `right-panel.tsx` | **10** | ✅ **CHOSEN** |
| B — `max-h-[80vh]` on SheetContent | Magic number; breaks on mobile browser chrome | 6 | ❌ |
| C — Native `overflow-y-auto` on div | Loses ScrollArea shadows + Radix styling | 7 | ⚠️ Fallback only |
| D — Reduce sections / accordion collapse | Hides problem; still fails with i18n long labels | 4 | ❌ |

---

## Decision 4 — UI Truncation Strategy

| Option | Description | Score | Verdict |
| ------ | ----------- | ----- | ------- |
| **A — Respect API + setting** | Skip 220-char slice when `fullChunkContent` | **9** | ✅ **CHOSEN** |
| B — API only | Full API text still `line-clamp-3` | 5 | ❌ User still sees ~3 lines |
| C — Always show full in UI | Ignores API; fetches chunk by ID | 3 | ❌ Over-engineered |

---

## Decision 5 — Scope of API Endpoints

| Endpoint | Include `content_granularity`? | Verdict |
| -------- | ------------------------------ | ------- |
| `POST /api/v1/chat/completions` | Yes | ✅ Primary UI path |
| `POST /api/v1/query/stream` | Yes | ✅ User request + SDK |
| `POST /api/v1/query` (non-stream) | Yes | ✅ Parity |
| `POST /api/v1/query/context` | Already has it | ✅ No change |

---

## Weighted Summary

| Criterion | Weight | Option A (chosen stack) |
| --------- | ------ | ----------------------- |
| DRY / maintainability | 25% | 9 |
| Backward compatibility | 20% | 10 |
| Time to implement | 15% | 8 |
| User value | 20% | 9 |
| Testability | 10% | 9 |
| Security (debug gate) | 10% | 9 |
| **Weighted total** | | **9.1** |

---

## Final Decision Record

```
DECISION-037-01: Add ContentGranularity to StreamQueryRequest + ChatCompletionRequest
                  (default: Citation). UI exposes boolean fullChunkContent mapped to
                  agent|citation.

DECISION-037-02: Fix QuerySettingsSheet scroll via flex-1 min-h-0 overflow-hidden
                  pattern (no new components).

DECISION-037-03: Refactor truncate_for_granularity() as SSOT for builder + mapper.

DECISION-037-04: Do not expose debug granularity in Query Settings UI.
```
