# SPEC-037 — Cross-Reference Matrix

**Purpose:** Maps every claim across all lenses to live source evidence.  
**Method:** Code is law — file + line verification (2026-07-01)

---

## Symptom Evidence

| Claim | File | Line(s) | Verified |
| ----- | ---- | ------- | -------- |
| Query Settings uses ScrollArea without `min-h-0` | `edgequake_webui/src/components/query/query-settings-sheet.tsx` | 108–119 | ✅ |
| Established scroll pattern uses `flex-1 min-h-0` | `edgequake_webui/src/components/layout/right-panel.tsx` | 137–140 | ✅ |
| Metadata sidebar uses same pattern | `edgequake_webui/src/components/document/metadata-sidebar.tsx` | 54 | ✅ |
| API snippets hard-truncated to 200 chars | `edgequake/crates/edgequake-api/src/services/source_reference_builder.rs` | 7, 48 | ✅ |
| `SNIPPET_LEN` used in bundle mapper | `edgequake/crates/edgequake-api/src/services/context_bundle_mapper.rs` | 15, 72–74 | ✅ |
| Agent granularity returns full chunk in mapper | `edgequake/crates/edgequake-api/src/services/context_bundle_mapper.rs` | 74, 440–465 | ✅ |
| Stream query hardcodes `Citation` granularity | `edgequake/crates/edgequake-api/src/handlers/query/query_stream.rs` | 313 | ✅ |
| Chat stream hardcodes `Citation` granularity | `edgequake/crates/edgequake-api/src/handlers/chat/streaming.rs` | 433 | ✅ |
| `StreamQueryRequest` lacks granularity field | `edgequake/crates/edgequake-api/src/handlers/query_types.rs` | 211–255 | ✅ |
| `ChatCompletionRequest` lacks granularity field | `edgequake/crates/edgequake-api/src/handlers/chat_types.rs` | 28–80 | ✅ |
| `ContextRetrievalRequest` has granularity | `edgequake/crates/edgequake-api/src/handlers/context_types.rs` | 33–35 | ✅ |
| UI truncates passages to 220 chars | `edgequake_webui/src/components/query/source-citations.tsx` | 270–275 | ✅ |
| UI uses `line-clamp-3` on passages | `edgequake_webui/src/components/query/source-citations.tsx` | 270 | ✅ |
| Primary query path uses chat stream | `edgequake_webui/src/hooks/use-query-streaming.ts` | 113 | ✅ |
| Settings default stream true | `edgequake_webui/src/stores/use-settings-store.ts` | 47 | ✅ |
| `QuerySettings` type has no fullChunk field | `edgequake_webui/src/types/settings.ts` | 24–63 | ✅ |
| Debug granularity admin-gated in MCP | `edgequake/crates/edgequake-api/src/mcp/gateway/tool_validation.rs` | 44–58 | ✅ |
| LLM context truncation separate from snippets | `edgequake/crates/edgequake-query/src/truncation.rs` | 134–155 | ✅ |

---

## Requirement Traceability

| Requirement | Source Lens | Implementation Target |
| ----------- | ----------- | --------------------- |
| REQ-037-01 | `003-product-owner-lens.md` AC1 | `query-settings-sheet.tsx` ScrollArea |
| REQ-037-02 | `004-ux-ui-designer-lens.md` | Response Mode toggle |
| REQ-037-03 | `003-product-owner-lens.md` AC6 | `use-settings-store.ts` |
| REQ-037-04 | `005-fullstack-developer-lens.md` | `query_types.rs`, `chat_types.rs` |
| REQ-037-05 | `005-fullstack-developer-lens.md` | `source_reference_builder.rs` |
| REQ-037-06 | `004-ux-ui-designer-lens.md` | `source-citations.tsx` |
| REQ-037-07 | `007-decision-matrix.md` Decision 2 | `ContentGranularity::default()` |
| REQ-037-08 | `003-product-owner-lens.md` AC10 | `tool_validation.rs` reuse |

---

## Decision Cross-Reference

| Decision | Justified In | Adversarially Tested In |
| -------- | ------------ | ----------------------- |
| Reuse `content_granularity` enum | `002-first-principles.md` P3, P6 | `007-decision-matrix.md` Attack on B |
| Default `citation` | `002-first-principles.md` P5 | `007-decision-matrix.md` Attack on B |
| Scroll `min-h-0` fix | `001-five-whys.md` WHY 2 | `007-decision-matrix.md` Decision 3 |
| Shared `truncate_for_granularity` | `005-fullstack-developer-lens.md` DRY | `008-implementation-plan.md` Phase 2 |

---

## Edge Case Cross-Reference

| Edge Case | Specified In | Mitigation In |
| --------- | ------------ | ------------- |
| Large SSE payload (50 × 2KB chunks) | `005-fullstack-developer-lens.md` | Default citation; toggle description |
| Unicode mid-char truncation | `005-fullstack-developer-lens.md` | Agent mode bypass |
| Old backend ignores field | `005-fullstack-developer-lens.md` | Deploy backend first |
| Debug without admin | `003-product-owner-lens.md` AC10 | Handler admin gate |
| UI double truncation | `001-five-whys.md` WHY 4 | `source-citations.tsx` conditional |
| Injection sources in full mode | `005-fullstack-developer-lens.md` | `is_injection_source` unchanged |
| Markdown in full passage | `005-fullstack-developer-lens.md` | `stripMarkdownSyntax` |
| Concurrent queries / settings race | `005-fullstack-developer-lens.md` | Per-request granularity at fire time |

---

## Test Cross-Reference

| Test Type | Plan Location | Covers REQ |
| --------- | ------------- | ---------- |
| `truncate_for_granularity` unit | `008-implementation-plan.md` 2.11 | REQ-037-05 |
| `spec037_stream_granularity` integration | `008-implementation-plan.md` 2.12 | REQ-037-04, 07 |
| Playwright scroll | `008-implementation-plan.md` 1.4 | REQ-037-01 |
| Playwright full chunk | `008-implementation-plan.md` Phase 5 | REQ-037-02, 06 |
| Existing `agent_granularity_includes_full_chunk` | `context_bundle_mapper.rs:440` | Pattern reference |

---

## Lens → Document Map

| Lens | Document | Primary Outputs |
| ---- | -------- | --------------- |
| Root Cause | `001-five-whys.md` | Causal chains, structural failures |
| First Principles | `002-first-principles.md` | P1–P8 truths, non-goals |
| Product Owner | `003-product-owner-lens.md` | User stories, AC, KPIs |
| UX/UI | `004-ux-ui-designer-lens.md` | Layout, toggle placement, a11y |
| Full Stack | `005-fullstack-developer-lens.md` | Architecture, files, tests |
| Decision | `007-decision-matrix.md` | Chosen options |
| Implementation | `008-implementation-plan.md` | Phased tasks, DoD |

---

## Related Specs

| Spec | Relationship |
| ---- | ------------ |
| SPEC-028 | `ContentGranularity` SSOT — extend to stream |
| SPEC-006 | Stream SSE protocol — context event shape unchanged |
| SPEC-004 | System Prompt — unblocked by scroll fix |
| SPEC-031 | Document scope — coexists in same settings sheet |
| SPEC-033 | Passage page groups — display logic in `source-citations.tsx` |
