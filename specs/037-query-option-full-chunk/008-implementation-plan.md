# SPEC-037 — Implementation Plan

**Lens:** Full Stack Implementation  
**Status:** `IMPLEMENTED` (2026-07-01)  
**Deploy order:** Backend → Frontend → OpenAPI/SDK sync

---

## Phase 0 — Prerequisites

- [x] Read SPEC-028 `ContentGranularity` tests in `spec028_context_e2e.rs`
- [x] Confirm primary UI path uses `chatCompletionStream` (`use-query-streaming.ts:113`)

---

## Phase 1 — Settings Scroll Fix (P0)

**Goal:** REQ-037-01 — System Prompt reachable

| Step | File | Status |
| ---- | ---- | ------ |
| 1.1 | `query-settings-sheet.tsx` — `overflow-hidden` on SheetContent | ✅ |
| 1.2 | `ScrollArea` → `flex-1 min-h-0` + `showShadows` | ✅ |
| 1.3 | Inner div `pb-6` | ✅ |
| 1.4 | `e2e/spec037-query-settings-scroll.spec.ts` | ✅ |

**Verify:** `PLAYWRIGHT_BASE_URL=http://localhost:3000 pnpm exec playwright test e2e/spec037-query-settings-scroll.spec.ts` — **PASS**

**Screenshots:** `specs/037-query-option-full-chunk/e2e/screenshots/01-*.png`, `02-*.png`

---

## Phase 2 — Backend Granularity SSOT (P0)

**Goal:** REQ-037-04, REQ-037-05, REQ-037-07, REQ-037-08

| Step | File | Status |
| ---- | ---- | ------ |
| 2.1 | `services/content_granularity.rs` — `truncate_for_granularity()` SSOT | ✅ |
| 2.2 | `source_reference_builder.rs` — `granularity` param | ✅ |
| 2.3 | `context_bundle_mapper.rs` — uses shared helper | ✅ |
| 2.4 | `query_types.rs` — `content_granularity` on `StreamQueryRequest`, `QueryRequest` | ✅ |
| 2.5 | `chat_types.rs` — `content_granularity` on `ChatCompletionRequest` | ✅ |
| 2.6 | `chat/mod.rs` — `build_sources(context, granularity)` | ✅ |
| 2.7 | `query_stream.rs` — threads granularity | ✅ |
| 2.8 | `chat/streaming.rs` — threads granularity | ✅ |
| 2.9 | `chat/completion.rs`, `query_execute.rs` | ✅ |
| 2.10 | `ensure_debug_granularity_allowed` + `OptionalAuth` on handlers | ✅ |
| 2.11 | `source_reference_builder.rs` unit tests | ✅ |
| 2.12 | `tests/spec037_stream_granularity.rs` | ✅ |

**Default:** `default_content_granularity()` → `Citation` (not enum `Default` which is `Agent`).

**Verify:**

```bash
cargo test -p edgequake-api --features postgres --test spec037_stream_granularity
cargo test -p edgequake-api --features postgres --lib services::source_reference_builder
```

---

## Phase 3 — Frontend Toggle + API Wire (P1)

| Step | File | Status |
| ---- | ---- | ------ |
| 3.1 | `types/settings.ts` — `fullChunkContent` | ✅ |
| 3.2 | `use-settings-store.ts` — default `false` | ✅ |
| 3.3 | `query-settings-sheet.tsx` — toggle + `data-testid` | ✅ |
| 3.4 | `query-interface.tsx` — passes setting | ✅ |
| 3.5 | `lib/api/chat.ts` — `content_granularity` | ✅ |
| 3.6 | `types/query.ts` — same | ✅ |
| 3.7 | `use-query-streaming.ts` — maps to API | ✅ |
| 3.8 | `source-citations.tsx` — conditional display | ✅ |
| 3.9 | `__tests__/spec037-format-passage-preview.test.ts` | ✅ |

---

## Phase 4 — OpenAPI & SDK (P2)

| Step | Status |
| ---- | ------ |
| 4.1 Regenerate OpenAPI snapshot | ⏭️ Deferred (field uses existing `ContentGranularity` schema) |
| 4.2 `sdks/typescript/src/types/query.ts` | ✅ |
| 4.3 Rust SDK | N/A (DTOs in crate) |

---

## Phase 5 — E2E & Regression (P1)

| Test | Result |
| ---- | ------ |
| `spec037-query-settings-scroll.spec.ts` | ✅ PASS |
| `spec037-query-full-chunk.spec.ts` | ✅ PASS (mocked SSE + API body capture) |
| `spec037-format-passage-preview.test.ts` | ✅ PASS |
| `spec037_stream_granularity.rs` | ✅ PASS |

```bash
PLAYWRIGHT_BASE_URL=http://localhost:3000 pnpm exec playwright test \
  e2e/spec037-query-settings-scroll.spec.ts e2e/spec037-query-full-chunk.spec.ts
```

---

## Definition of Done

- [x] REQ-037-01 through REQ-037-08 satisfied
- [x] `cargo test -p edgequake-api --features postgres --test spec037_stream_granularity`
- [x] Playwright SPEC-037 specs green
- [x] E2E screenshots in `specs/037-query-option-full-chunk/e2e/screenshots/`
- [ ] `cargo clippy -p edgequake-api` (run before merge)
- [ ] Full OpenAPI snapshot regen (optional follow-up)

---

## Key Files Created

| Path | Purpose |
| ---- | ------- |
| `services/content_granularity.rs` | DRY SSOT for truncation + debug policy |
| `tests/spec037_stream_granularity.rs` | API integration tests |
| `e2e/spec037-query-settings-scroll.spec.ts` | Scroll E2E |
| `e2e/spec037-query-full-chunk.spec.ts` | Granularity API wire E2E |
| `e2e/screenshots/README.md` | Screenshot analysis index |
