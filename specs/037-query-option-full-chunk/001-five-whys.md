# SPEC-037 — 5 WHYs: Root Cause Analysis

**Lens:** Root Cause Analysis  
**Method:** 5 WHYs — iterative causal chain to structural root  
**Evidence:** All claims verified against live source code (2026-07-01)

---

## Symptom A — Query Settings Panel Not Scrollable

A user opens **Query Settings** on the Query page. They see Context, Response Mode, Retrieval, Generation — but **System Prompt is clipped** at the bottom. No scrollbar appears. They cannot configure custom instructions.

### WHY 1 — Why is System Prompt unreachable?

**Because the sheet content overflows its viewport without a working scroll container.**

```tsx
// edgequake_webui/src/components/query/query-settings-sheet.tsx:108–119
<SheetContent className="w-[400px] sm:w-[480px] flex flex-col p-0">
  <SheetHeader className="px-6 py-4 border-b shrink-0">...</SheetHeader>
  <ScrollArea className="flex-1">   {/* ← missing min-h-0 */}
```

The `ScrollArea` has `flex-1` but not `min-h-0`. In a flex column, children default to `min-height: auto`, so the scroll region **grows to fit content** instead of constraining and scrolling.

### WHY 2 — Why wasn't `min-h-0` applied?

**Because the fix pattern exists elsewhere but was not applied when Query Settings was extracted.**

```tsx
// edgequake_webui/src/components/layout/right-panel.tsx:137–140
// WHY: h-full constrains the aside to its container height so the inner
// ScrollArea (flex-1 min-h-0) can scroll instead of the aside growing
<ScrollArea className="flex-1 min-h-0" showShadows>
```

`metadata-sidebar.tsx` and `entity-edit-dialog.tsx` use the same pattern. `query-settings-sheet.tsx` does not.

### WHY 3 — Why did extraction miss the scroll constraint?

**Because `SheetContent` from Radix is `fixed` + `h-full` but the inner flex child was not given the overflow contract documented in `right-panel.tsx`.**

`SheetContent` sets `flex flex-col` (via `gap-4` default) but the scroll child needs explicit `min-h-0` + parent `overflow-hidden` to participate in height budgeting.

### WHY 4 — Why was there no visual regression test?

**No Playwright spec asserts scroll reachability for Query Settings.** E2E coverage exists for markdown/upload flows but not settings sheet overflow on short viewports.

### WHY 5 — Why is overflow testing absent?

**No component-level checklist for drawer/sheet panels requiring scroll.** Structural gap: panel components added without referencing the established `flex-1 min-h-0` contract.

---

## Symptom B — Passages Truncated Mid-Word

A user runs a streaming query. Retrieved passages show snippets ending in `uncertai`, `qu` — mid-word. Tooltip says "Click to open and highlight this passage" but the preview is unusable for verification.

### WHY 1 — Why do passages end mid-word?

**Because snippet text is truncated to 200 Unicode characters at the API layer.**

```rust
// edgequake/crates/edgequake-api/src/services/source_reference_builder.rs:48
snippet: Some(chunk.content.chars().take(SNIPPET_LEN).collect()),
// SNIPPET_LEN = 200 (line 7)
```

Character-boundary truncation without word awareness produces mid-word cuts.

### WHY 2 — Why does the stream endpoint always truncate?

**Because stream handlers hardcode `ContentGranularity::Citation` and call `build_sources` without granularity.**

```rust
// edgequake/crates/edgequake-api/src/handlers/query/query_stream.rs:307–313
let mut sources = build_sources(&context);
let mapping_opts = MappingOptions {
    granularity: ContentGranularity::Citation,  // ← always citation
```

Same in chat stream:

```rust
// edgequake/crates/edgequake-api/src/handlers/chat/streaming.rs:418, 433
let mut sources = build_sources(&context);
granularity: ContentGranularity::Citation,
```

### WHY 3 — Why isn't SPEC-028 granularity wired to stream?

**Because `content_granularity` was implemented for `/query/context` (SPEC-028) but never added to `StreamQueryRequest` or `ChatCompletionRequest`.**

```rust
// edgequake/crates/edgequake-api/src/handlers/context_types.rs:13–20
pub enum ContentGranularity {
    Citation,  // 200 chars
    Agent,     // full chunk
    Debug,
}
```

`ContextRetrievalRequest` has the field. `StreamQueryRequest` does not (`query_types.rs:211–255`).

### WHY 4 — Why does the UI also truncate?

**Defense in depth became double truncation.** Even full API text would be clipped in UI:

```tsx
// edgequake_webui/src/components/query/source-citations.tsx:273–274
const snippet = clean.length > 220 ? clean.slice(0, 220).replace(...) + '…' : clean;
```

Plus `line-clamp-3` on the paragraph (line 270).

### WHY 5 — Why was there no user control?

**Product assumption: snippets save bandwidth.** Valid for list views; invalid when users need to **verify retrieval quality** before trusting the answer. No opt-in escape hatch was exposed in Query Settings.

---

## Root Cause Statements

> **Scroll:** Query Settings sheet violates the established flex scroll contract (`flex-1 min-h-0` + parent `overflow-hidden`), causing content growth instead of viewport-constrained scrolling.

> **Truncation:** Stream APIs bypass SPEC-028 granularity, hardcode 200-char citation snippets in `build_sources_from_context`, and the UI applies a second 220-char clamp — with no settings surface to request full chunk text.

---

## Causal Chain Summary

```
SYMPTOM A: System Prompt clipped
    ↑
WHY 1: ScrollArea lacks min-h-0
    ↑
WHY 2: Pattern documented in right-panel.tsx not reused
    ↑
ROOT A: No panel scroll contract checklist

SYMPTOM B: Mid-word passage snippets
    ↑
WHY 1: API SNIPPET_LEN=200 hard truncate
    ↑
WHY 2: Stream hardcodes ContentGranularity::Citation
    ↑
WHY 3: content_granularity not on StreamQueryRequest / ChatCompletionRequest
    ↑
WHY 4: UI double-truncates at 220 chars
    ↑
ROOT B: Granularity SSOT exists but not plumbed to user-facing stream path
```

---

## Structural Failures Identified

| Failure | Impact | DRY/SOLID Violation |
| ------- | ------ | ------------------- |
| Missing `min-h-0` on settings ScrollArea | Settings unusable on laptop viewports | — |
| `build_sources_from_context` ignores granularity | Duplicate truncation logic vs mapper | DRY |
| Stream handlers hardcode Citation | User cannot opt into full chunks | — |
| UI 220-char clamp independent of API | Full chunks still look truncated | DRY |
| No E2E for settings scroll | Regression risk | — |
