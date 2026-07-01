# SPEC-037 — Query Settings: Scrollable Panel + Full-Chunk Stream Option

**Spec:** `037-query-option-full-chunk`  
**Date:** 2026-07-01  
**Method:** Code is law — all claims cross-referenced against live source files.  
**Status:** `IMPLEMENTED` (2026-07-01)  
**Triggers:** User-reported UX defects (non-scrollable settings panel, truncated passage citations)

---

## TL;DR — Executive Decision

> **Fix Query Settings sheet scroll with the established `flex-1 min-h-0` pattern. Expose SPEC-028 `content_granularity` on streaming query APIs (`/query/stream`, `/chat/completions`) and wire a UI toggle in Response Mode. Default remains `citation` (200-char snippets) for backward compatibility; `agent` returns full chunk text. DRY: one enum, one mapper, one builder — no parallel boolean flags.**

---

## The Evidence (Code is Law)

| Symptom | Root cause (file) | Line(s) |
| ------- | ----------------- | ------- |
| Settings panel clips System Prompt | `ScrollArea` missing `min-h-0`; parent flex chain unbounded | `query-settings-sheet.tsx:108–119` |
| Passages end mid-word (`uncertai`, `qu`) | API hardcodes 200-char snippet in `build_sources_from_context` | `source_reference_builder.rs:48` |
| Stream context always citation tier | `ContentGranularity::Citation` hardcoded in stream handlers | `query_stream.rs:313`, `chat/streaming.rs:433` |
| UI re-truncates even if API sent more | `source-citations.tsx` slices to 220 chars + `line-clamp-3` | `source-citations.tsx:270–275` |
| `/query/context` already supports full chunks | `ContentGranularity::Agent` in `context_bundle_mapper.rs` | `context_bundle_mapper.rs:72–75` |
| Stream DTO has no granularity field | `StreamQueryRequest` ends at `include_subgraph` | `query_types.rs:211–255` |

---

## Documents in this Spec

| File | Lens | Key Question |
| ---- | ---- | ------------ |
| [001-five-whys.md](./001-five-whys.md) | Root Cause | Why are passages truncated and settings clipped? |
| [002-first-principles.md](./002-first-principles.md) | First Principles | What are we really solving? |
| [003-product-owner-lens.md](./003-product-owner-lens.md) | Product Owner | What is the user/business value? |
| [004-ux-ui-designer-lens.md](./004-ux-ui-designer-lens.md) | UX/UI Designer | How should settings and citations behave? |
| [005-fullstack-developer-lens.md](./005-fullstack-developer-lens.md) | Full Stack Dev | How to implement correctly (DRY/SOLID)? |
| [007-decision-matrix.md](./007-decision-matrix.md) | Decision | Reuse `content_granularity` vs new boolean |
| [008-implementation-plan.md](./008-implementation-plan.md) | Implementation | Phased plan, tests, acceptance criteria |
| [009-cross-reference-matrix.md](./009-cross-reference-matrix.md) | Cross-Reference | Every claim linked to evidence |

---

## Decision Summary

```
CHOSEN: Reuse SPEC-028 ContentGranularity on stream endpoints + UI toggle
         mapped to citation (default) | agent (full chunks).

REJECTED: New parallel `include_full_chunks` boolean in Rust (DRY violation)
REJECTED: Frontend-only fix (API still sends 200 chars)
REJECTED: Always send full chunks (SSE payload + mobile bandwidth risk)
REJECTED: New ScrollArea component (pattern already exists in right-panel.tsx)
```

**Requirements (REQ-037-xx):**

| ID | Requirement |
| -- | ----------- |
| REQ-037-01 | Query Settings sheet scrolls to System Prompt on 768px viewport |
| REQ-037-02 | Toggle "Full passage text" in Response Mode section |
| REQ-037-03 | Setting persisted in `use-settings-store` |
| REQ-037-04 | `content_granularity` on `StreamQueryRequest` + `ChatCompletionRequest` |
| REQ-037-05 | `build_sources_from_context` respects granularity (SSOT) |
| REQ-037-06 | Citation UI shows full text when granularity is `agent` |
| REQ-037-07 | Default `citation` — no breaking change for API clients |
| REQ-037-08 | `debug` granularity admin-gated (reuse MCP policy) |
