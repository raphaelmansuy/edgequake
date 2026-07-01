# SPEC-037 — Full Stack Developer Lens

**Lens:** Full Stack Developer  
**Principles:** DRY, SOLID, code is law  
**Stack:** Rust (Axum/utoipa) + React 19 + Zustand + SSE

---

## Architecture Overview

```text
┌─ Frontend ─────────────────────────────────────────────────────┐
│  QuerySettingsSheet                                            │
│    fullChunkContent toggle → use-settings-store                │
│  use-query-streaming.ts                                        │
│    chatCompletionStream({ content_granularity: ... })          │
│  source-citations.tsx                                          │
│    display full text when settings.fullChunkContent            │
└────────────────────────────┬───────────────────────────────────┘
                             │ POST /api/v1/chat/completions (SSE)
                             │ POST /api/v1/query/stream (SSE)
┌────────────────────────────▼───────────────────────────────────┐
│  edgequake-api                                                 │
│  ChatCompletionRequest / StreamQueryRequest                    │
│    + content_granularity: ContentGranularity (default Citation)│
│  query_stream.rs / chat/streaming.rs                         │
│    → build_sources_from_context(ctx, granularity, ...)         │
│    → MappingOptions { granularity, ... }                       │
│  source_reference_builder.rs (SSOT for SourceReference.snippet)│
│  context_bundle_mapper.rs (SSOT for ContextBundle chunks)      │
└────────────────────────────────────────────────────────────────┘
```

---

## SOLID Mapping

| Principle | Application |
| --------- | ----------- |
| **S** | `source_reference_builder` — only builds flat citations; `context_bundle_mapper` — only builds bundles |
| **O** | New granularity tiers extend enum match arms; handlers unchanged |
| **L** | `ContentGranularity` serializes same across context + stream APIs |
| **I** | UI depends on `QuerySettings.fullChunkContent`; maps at API boundary |
| **D** | Handlers depend on `build_sources_from_context`, not inline `.take(200)` |

---

## DRY — Single Sources of Truth

| Concern | SSOT | Consumers |
| ------- | ---- | --------- |
| Snippet length constant | `SNIPPET_LEN` in `source_reference_builder.rs` | Mapper imports or shares constant |
| Granularity enum | `context_types::ContentGranularity` | Context, stream, chat, MCP |
| Snippet truncation logic | `truncate_content_for_granularity(content, granularity)` — **new shared fn** | Builder + mapper |
| Scroll flex contract | `flex-1 min-h-0` on ScrollArea | All sheet panels |

### Proposed shared helper (new module or in `source_reference_builder.rs`)

```rust
pub fn truncate_for_granularity(content: &str, granularity: ContentGranularity) -> String {
    match granularity {
        ContentGranularity::Citation => content.chars().take(SNIPPET_LEN).collect(),
        ContentGranularity::Agent | ContentGranularity::Debug => content.to_string(),
    }
}
```

Refactor `context_bundle_mapper.rs` and `source_reference_builder.rs` to call this — eliminates duplicate match arms.

---

## Backend Changes

### 1. DTOs — add field

**File:** `edgequake-api/src/handlers/query_types.rs`

```rust
// StreamQueryRequest — after include_subgraph
/// Payload tier for source snippets in context events.
/// @implements SPEC-037 + SPEC-028
#[serde(default)]
pub content_granularity: ContentGranularity,
```

Import `ContentGranularity` from `context_types`.

**File:** `edgequake-api/src/handlers/chat_types.rs` — same field on `ChatCompletionRequest`.

**File:** `edgequake-api/src/handlers/query_types.rs` — optional on non-stream `QueryRequest` for `POST /query` parity.

### 2. Source builder — accept granularity

**File:** `edgequake-api/src/services/source_reference_builder.rs`

```rust
pub fn build_sources_from_context(
    context: &QueryContext,
    include_reference_ids: bool,
    rerank_top_k: Option<usize>,
    reranked: bool,
    granularity: ContentGranularity,  // NEW
) -> Vec<SourceReference>
```

Replace line 48:

```rust
snippet: Some(truncate_for_granularity(&chunk.content, granularity)),
```

Entity descriptions: same treatment (line 93).

Update `build_sources` wrapper in `chat/mod.rs`:

```rust
pub(crate) fn build_sources(
    context: &QueryContext,
    granularity: ContentGranularity,
) -> Vec<SourceReference> {
    build_sources_from_context(context, true, None, false, granularity)
}
```

### 3. Stream handlers — thread granularity

**File:** `query_stream.rs` (~307–320)

```rust
let granularity = request.content_granularity;
let mut sources = build_sources(&context, granularity);
let mapping_opts = MappingOptions {
    granularity,  // was hardcoded Citation
    // ...
};
```

**File:** `chat/streaming.rs` (~418–440) — read from `ChatCompletionRequest.content_granularity`.

**File:** `chat/completion.rs`, `query/mod.rs` (non-stream) — same pattern.

### 4. Admin gate for debug

Reuse `tool_validation::enforce_debug_granularity` or extract shared fn:

```rust
if request.content_granularity == ContentGranularity::Debug {
    enforce_debug_granularity_for_role(&tenant_ctx.role)?;
}
```

### 5. OpenAPI

`ContentGranularity` already has `ToSchema`. Regenerate snapshot:

```bash
cd edgequake_webui && bun run openapi:sync  # or project equivalent
```

---

## Frontend Changes

### 1. Settings type

**File:** `edgequake_webui/src/types/settings.ts`

```typescript
export interface QuerySettings {
  // ...existing
  /**
   * When true, stream context events use full chunk text (content_granularity: agent).
   * @implements SPEC-037
   */
  fullChunkContent?: boolean;
}
```

**File:** `use-settings-store.ts` — default `fullChunkContent: false`.

### 2. Query Settings Sheet

**File:** `query-settings-sheet.tsx`

Scroll fix:

```tsx
<SheetContent className="w-[400px] sm:w-[480px] flex flex-col p-0 overflow-hidden">
  ...
  <ScrollArea className="flex-1 min-h-0" showShadows>
    <div className="px-6 py-4 pb-6 space-y-5">
```

Toggle in Response Mode section (after Streaming switch).

Extend local `QuerySettings` interface with `fullChunkContent?: boolean`.

### 3. API request wiring

**File:** `edgequake_webui/src/lib/api/chat.ts`

```typescript
export interface ChatCompletionRequest {
  // ...
  content_granularity?: 'citation' | 'agent' | 'debug';
}
```

**File:** `edgequake_webui/src/types/query.ts` — same on `QueryRequest`.

**File:** `use-query-streaming.ts` (~113–128)

```typescript
content_granularity: querySettings.fullChunkContent ? 'agent' : 'citation',
```

### 4. Citation display

**File:** `source-citations.tsx`

Pass `fullChunkContent` prop from parent (or read settings store).

```tsx
// When fullChunkContent:
<p className="text-xs ... break-words whitespace-pre-wrap">
  {stripMarkdownSyntax(chunk.content)}
</p>

// When false — keep existing 220-char + line-clamp-3
```

**Alternative (DRY):** If API sends full text but UI always clamps, check `chunk.content.length > 220` AND setting — when setting ON, skip slice.

### 5. Query settings sheet props

**File:** `query-interface.tsx` — pass `fullChunkContent` in settings object to sheet.

---

## Edge Cases & Mitigations

| Edge Case | Risk | Mitigation |
| --------- | ---- | ---------- |
| 50 chunks × 2KB each in agent mode | Large SSE context event (~100KB) | Default citation; document bandwidth in toggle description; consider `top_k` interaction (user controls count) |
| Unicode grapheme split at 200 chars | Mid-emoji cut in citation mode | Pre-existing; out of scope; agent mode sends full UTF-8 |
| `fullChunkContent` true + old backend | API ignores unknown field | Graceful: UI still truncated at 220 until backend deployed |
| Debug granularity from UI | Admin data leak | Do not expose debug in UI; API returns 403 for non-admin |
| Markdown in full chunk | Raw `##` visible | Keep `stripMarkdownSyntax` in display |
| Memory in stream accumulator | Large context in React state | Same as today for agent API clients; monitor |
| Settings sheet on iOS Safari | `-webkit-overflow-scrolling` | Radix ScrollArea handles; E2E on mobile viewport |
| Concurrent queries different settings | Race | Each request sends its own granularity at fire time |
| Entity snippets in agent mode | Long descriptions | Same granularity applies to entities — acceptable |
| Injection sources | Leak injection text | `is_injection_source` filter unchanged |
| Rerank top-k after full content | Order only | `rerank_top_k` truncates count, not content length |

---

## Test Plan (battle-tested)

### Rust unit tests

| Test | File |
| ---- | ---- |
| `citation_truncates_at_200` | `source_reference_builder.rs` |
| `agent_returns_full_chunk` | `source_reference_builder.rs` |
| `truncate_for_granularity_shared` | new or builder |
| `stream_request_deserializes_granularity_default_citation` | `query_types.rs` tests |

### Rust integration

| Test | File |
| ---- | ---- |
| Stream with `content_granularity: agent` → snippet len > 200 | new `spec037_stream_granularity.rs` |
| Stream default → snippet len ≤ 200 | same |
| Chat stream parity | extend `spec028_context_e2e.rs` patterns |
| Debug without admin → 403 | reuse `spec028_mcp_oauth_e2e` helper |

### Frontend unit

| Test | File |
| ---- | ---- |
| `buildDocumentFilter` unchanged | existing |
| Map `fullChunkContent` → `content_granularity` | new test in `use-query-streaming` or chat API test |
| Snippet display full vs clamped | `source-citations.test.tsx` |

### E2E Playwright

| Test | Assertion |
| ---- | --------- |
| `query-settings-scroll.spec.ts` | Open settings, scroll to `#system-prompt`, `toBeVisible()` |
| `query-full-chunk.spec.ts` | Toggle on, mock/stream query, passage text not ending with partial word |

### Manual smoke

```bash
make dev-bg
# 1. Open Query → Settings → scroll to System Prompt ✓
# 2. Enable Full passage text → query → passages show full sentences ✓
curl -X POST http://localhost:8080/api/v1/query/stream \
  -H "Content-Type: application/json" \
  -d '{"query":"test","content_granularity":"agent","stream_format":"v2"}'
# Inspect first context event sources[].snippet length ✓
```

---

## File Change List

| File | Action |
| ---- | ------ |
| `query-settings-sheet.tsx` | Fix scroll + add toggle |
| `types/settings.ts` | Add `fullChunkContent` |
| `use-settings-store.ts` | Default false |
| `use-query-streaming.ts` | Wire `content_granularity` |
| `lib/api/chat.ts` | Type field |
| `types/query.ts` | Type field |
| `source-citations.tsx` | Conditional display |
| `query_types.rs` | DTO field |
| `chat_types.rs` | DTO field |
| `source_reference_builder.rs` | Granularity param + shared fn |
| `context_bundle_mapper.rs` | Use shared fn |
| `query_stream.rs` | Thread granularity |
| `chat/streaming.rs` | Thread granularity |
| `chat/mod.rs` | Update `build_sources` signature |
| `openapi.rs` / snapshot | Schema regen |
| `sdks/typescript/src/types/query.ts` | Type field |

---

## Migration / Compatibility

- **API:** `#[serde(default)]` on `content_granularity` → `Citation` — zero breaking change.
- **Frontend store:** `fullChunkContent` optional, defaults false on hydrate.
- **Deploy order:** Backend first (forward compatible), then frontend.

---

## REQ Mapping

| REQ | Implementation |
| --- | -------------- |
| REQ-037-01 | `query-settings-sheet.tsx` scroll classes |
| REQ-037-02 | Toggle component |
| REQ-037-03 | `use-settings-store.ts` |
| REQ-037-04 | DTOs in `query_types.rs`, `chat_types.rs` |
| REQ-037-05 | `source_reference_builder.rs` |
| REQ-037-06 | `source-citations.tsx` |
| REQ-037-07 | `Default for ContentGranularity` / serde default |
| REQ-037-08 | Admin check in handlers |
