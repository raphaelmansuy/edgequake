# SPEC-037 — First Principles

**Lens:** First Principles Decomposition  
**Method:** Strip assumptions; rebuild from irreducible truths  
**Anchors:** Live codebase (SPEC-028, SPEC-006, flex scroll patterns)

---

## P1 — What is a "passage" in EdgeQuake?

A **passage** is a `RetrievedChunk` from vector/graph retrieval — stored content with score, document lineage, and optional page/line anchors.

```rust
// edgequake/crates/edgequake-query/src/context.rs (conceptual)
pub struct RetrievedChunk {
    pub id: String,
    pub content: String,      // ← authoritative full text in engine context
    pub score: f32,
    pub document_id: Option<String>,
    // ...
}
```

**Truth:** Full chunk text already exists in the engine before SSE emission. Truncation is a **presentation choice**, not a retrieval limitation.

---

## P2 — What is "truncation" doing today?

Two independent layers:

| Layer | Location | Limit | Purpose |
| ----- | -------- | ----- | ------- |
| LLM context budget | `truncation.rs` `balance_context` | ~10K tokens/chunks | Fit LLM window (BR0101) |
| Citation snippet | `source_reference_builder.rs` | 200 chars | Reduce SSE payload |
| UI preview | `source-citations.tsx` | 220 chars + 3 lines | Compact list density |

**Truth:** Layers 2 and 3 are **independent of** layer 1. User request is about layers 2–3, not LLM context truncation.

---

## P3 — Does a granularity abstraction already exist?

**Yes.** SPEC-028 `ContentGranularity`:

| Tier | Chunk content | Use case |
| ---- | ------------- | -------- |
| `citation` | 200 chars | UI lists, legacy compat |
| `agent` | Full chunk | Agents, verification |
| `debug` | Full + context string | Admin diagnostics |

```rust
// context_bundle_mapper.rs:72–75
let content = match options.granularity {
    ContentGranularity::Citation => chunk.content.chars().take(SNIPPET_LEN).collect(),
    ContentGranularity::Agent | ContentGranularity::Debug => chunk.content.clone(),
};
```

**Truth:** Reuse this enum. Do not invent `include_full_chunks: bool` in Rust.

---

## P4 — What is a settings panel?

A **bounded viewport** with **scrollable body** and **fixed chrome** (header, close).

Irreducible layout contract:

```text
┌─ SheetContent (h-full, flex-col, overflow-hidden) ─┐
│  Header (shrink-0)                                  │
│  ┌─ ScrollArea (flex-1 min-h-0) ─────────────────┐ │
│  │  Sections...                                   │ │
│  │  System Prompt textarea                        │ │
│  └────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

**Truth:** Scroll is a CSS flex constraint problem, not a Radix bug.

---

## P5 — What does the user actually need?

| Need | Principle |
| ---- | --------- |
| Reach all settings | **Completeness** — every control accessible without resize |
| Read full passage when verifying RAG | **Provenance** — see what the model saw |
| Default unchanged for API clients | **Stability** — `citation` remains default |
| Control bandwidth on mobile | **Progressive disclosure** — opt-in full text |

---

## P6 — What is the minimal correct API surface?

Add **one field** to streaming request DTOs:

```rust
#[serde(default)]
pub content_granularity: ContentGranularity,  // default: Citation
```

Plumb to:

1. `build_sources_from_context(context, granularity, ...)`
2. `MappingOptions { granularity, ... }` in stream context events

**Truth:** Single parameter threads through handler → builder → mapper. SOLID: one reason to change snippet length (`SNIPPET_LEN` + granularity match).

---

## P7 — What must NOT change?

| Invariant | Reason |
| --------- | ------ |
| `balance_context` token budgets | BR0101 — LLM safety |
| Default `citation` on stream | Backward compat, payload size |
| `debug` without admin role | EC-MCP-29 policy |
| Injection source filtering | Security — `is_injection_source` |

---

## P8 — Battle-tested patterns to copy

| Pattern | Source | Apply to |
| ------- | ------ | -------- |
| `flex-1 min-h-0` ScrollArea | `right-panel.tsx:140` | `query-settings-sheet.tsx` |
| `content_granularity` on context API | `context_types.rs` | Stream + chat DTOs |
| Admin gate for debug | `tool_validation.rs:44–58` | Stream handler (if debug requested) |
| Granularity unit test | `context_bundle_mapper.rs:440–465` | `source_reference_builder` tests |
| Settings persistence | `use-settings-store.ts` | `fullChunkContent` → API mapping |

---

## Rebuilt Solution (from first principles)

```text
User toggles "Full passage text"
    → settings.fullChunkContent = true
    → API content_granularity = "agent"
    → build_sources_from_context uses full chunk.content
    → source-citations skips 220-char slice when full mode
    → User reads complete passage in citation card
```

```text
User opens Query Settings on 768px screen
    → SheetContent overflow-hidden
    → ScrollArea flex-1 min-h-0 showShadows
    → System Prompt textarea reachable via scroll
```

---

## Non-Goals (explicit)

- Word-boundary-aware truncation (nice-to-have; full-chunk mode sidesteps)
- Changing default chunk size at ingestion
- Exposing `debug` granularity in Query Settings UI
- Removing `line-clamp` in citation mode (keep compact default)
