# SPEC-006: Unified Streaming Response — Get All Information in a Single Call

> **Issue**: [#56 — Get all information (streamed request, entities, relations, sources) in a unique call](https://github.com/raphaelmansuy/edgequake/issues/56)
> **Status**: Draft
> **Priority**: High
> **Complexity**: Medium

---

## Summary

Ensure that **all RAG query metadata** — entities, relationships, document sources, retrieval statistics, and query mode used — is delivered within **a single streaming (SSE) call**, eliminating the need for a follow-up request.

### Motivation (from issue)

> "Currently, one calls EdgeQuake to get the streamed response, and once finished, makes a complementary call with the conversation id to get the Entities, Relationships and Sources. Could it be possible to finish the first call with the later information so that everything is retrieved in one cycle?"

---

## Current State Analysis

### Two Streaming Endpoints Exist

| Endpoint                    | Path                                   | Protocol                           | Context Included?                                                          |
| --------------------------- | -------------------------------------- | ---------------------------------- | -------------------------------------------------------------------------- |
| **Chat Completions Stream** | `POST /api/v1/chat/completions/stream` | Structured SSE (typed JSON events) | **Partial** — sends `context` event with `SourceReference[]` before tokens |
| **Query Stream**            | `POST /api/v1/query/stream`            | Raw SSE (plain text chunks)        | **No** — only streams raw LLM text tokens                                  |

### Chat Completions Stream (`/chat/completions/stream`) — Already Partial

The primary endpoint used by the WebUI already uses a structured SSE protocol with typed events:

```
Event Flow:
  1. { type: "conversation", conversation_id, user_message_id }
  2. { type: "context",      sources: SourceReference[] }          ← entities + relationships + chunks
  3. { type: "token",        content: "..." }   × N               ← streamed LLM tokens
  4. { type: "done",         assistant_message_id, tokens_used, duration_ms, llm_provider?, llm_model? }
  5. { type: "title_update", conversation_id, title }              ← optional, for new conversations
```

**What's already working:**

- ✅ `context` event sent BEFORE tokens (entities, relationships, chunks as `SourceReference[]`)
- ✅ Frontend `ChatStreamEvent` TypeScript type handles all event types
- ✅ `reduceStreamingEvent()` state reducer accumulates sources from `context` event
- ✅ `SourceCitations` component renders entities, relationships, and chunks from `QueryContext`

**What's missing from this endpoint (gaps):**

| Gap | Description                                                                      | Impact                                                                       |
| --- | -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| G-1 | Entity `entity_type` lost in conversion (hardcoded to `"UNKNOWN"`)               | Source citations cannot categorize entities by type                          |
| G-2 | Entity `degree` (graph connections count) not passed through                     | UI cannot show entity importance/centrality                                  |
| G-3 | Entity `source_chunk_ids` not passed through                                     | Cannot link entity citations back to specific chunks                         |
| G-4 | `QueryStats` not included in `done` event (only `tokens_used` and `duration_ms`) | No visibility into embedding_time, retrieval_time, generation_time breakdown |
| G-5 | `query_mode` used (after adaptive mode selection) not reported                   | Client doesn't know if adaptive selection overrode the requested mode        |

### Query Stream (`/query/stream`) — Does Not Return Context

The simpler endpoint only streams raw text tokens via SSE. No structured events, no context metadata.

**Root cause:** The handler calls `sota_engine.query_stream(request)` which returns `BoxStream<String>` (text-only). The engine **already has** `query_stream_with_context(request)` which returns `(QueryContext, QueryMode, BoxStream<String>)` — but the handler doesn't use it.

**Current handler (simplified):**

```rust
// query_stream.rs — CURRENT (broken for issue #56)
let stream = state.sota_engine.query_stream(engine_request).await?;
let sse_stream = stream.map(|res| match res {
    Ok(text) => Ok(Event::default().data(text)),    // raw text only
    Err(e) => Ok(Event::default().data(format!("Error: {}", e))),
});
Ok(Sse::new(sse_stream))
```

---

## Requirements

### FR-001: Unified Streaming Protocol for `/query/stream`

**As a** developer using the EdgeQuake API directly, **I want** the `/query/stream` endpoint to return structured SSE events that include entities, relationships, sources, and statistics alongside the streamed LLM tokens, **so that** I do not need a separate follow-up call.

**Acceptance Criteria:**

- The `/query/stream` endpoint MUST emit structured JSON SSE events (same typed protocol as `/chat/completions/stream`)
- Events MUST include a `type` field for discrimination
- A `context` event MUST be emitted before any `token` events
- A `done` or `stats` event MUST be emitted after all tokens

### FR-002: Enrich `SourceReference` with Entity Metadata

**As a** frontend developer, **I want** entity source references to include `entity_type`, `degree`, and `source_chunk_ids`, **so that** I can display richer entity citations.

**Acceptance Criteria:**

- `SourceReference` for entities MUST include `entity_type` (e.g., `"PERSON"`, `"ORGANIZATION"`)
- `SourceReference` for entities SHOULD include `degree` (number of graph connections)
- `SourceReference` for entities SHOULD include `source_chunk_ids` (provenance chain)

### FR-003: Include Retrieval Statistics in Streaming Events

**As a** developer, **I want** retrieval timing breakdown (embedding, retrieval, generation) in the streaming response, **so that** I can measure and optimize query performance.

**Acceptance Criteria:**

- A `stats` event or enriched `done` event MUST include:
  - `embedding_time_ms`
  - `retrieval_time_ms`
  - `generation_time_ms`
  - `total_time_ms`
  - `sources_retrieved` count
  - `query_mode` actually used (after adaptive selection)
  - `llm_provider` and `llm_model` used

### FR-004: Backward Compatibility for SDKs

**As a** maintainer of SDKs (Python, Java, TypeScript), **I want** the new protocol to be backward-compatible, **so that** existing SDK consumers are not broken.

**Acceptance Criteria:**

- Old-format clients that read plain `data:` SSE lines as raw text SHOULD still receive the LLM content (graceful degradation)
- A `stream_format` request parameter (`"v1"` or `"v2"`) MAY be introduced for explicit version negotiation
- If `stream_format` is omitted, default to `"v2"` (new structured protocol)

---

## Design

### Unified SSE Event Protocol (v2)

Both `/query/stream` and `/chat/completions/stream` converge on an identical typed SSE event protocol. Each SSE `data:` field is a JSON object with a `type` discriminator.

#### Event Types

| Event      | Cardinality | When Emitted                        | Payload                                      |
| ---------- | ----------- | ----------------------------------- | -------------------------------------------- |
| `context`  | 0..1        | After retrieval, before first token | `{ sources, query_mode, retrieval_time_ms }` |
| `token`    | 0..N        | During LLM generation               | `{ content }`                                |
| `thinking` | 0..N        | During chain-of-thought             | `{ content }`                                |
| `done`     | 1           | After last token                    | `{ stats, llm_provider?, llm_model? }`       |
| `error`    | 0..1        | On failure                          | `{ message, code }`                          |

**Chat-specific events** (only on `/chat/completions/stream`):

| Event          | Cardinality | When Emitted   | Payload                                |
| -------------- | ----------- | -------------- | -------------------------------------- |
| `conversation` | 1           | Before context | `{ conversation_id, user_message_id }` |
| `title_update` | 0..1        | After done     | `{ conversation_id, title }`           |

#### Event Schemas

**`context` event (enriched):**

```json
{
  "type": "context",
  "sources": [
    {
      "source_type": "entity",
      "id": "SARAH_CHEN",
      "score": 0.95,
      "snippet": "AI researcher at MIT specializing in NLP",
      "entity_type": "PERSON",
      "degree": 12,
      "source_chunk_ids": ["chunk-abc", "chunk-def"],
      "document_id": "doc-123",
      "file_path": "research_paper.pdf",
      "reference_id": 1
    },
    {
      "source_type": "relationship",
      "id": "SARAH_CHEN->MIT",
      "score": 0.88,
      "snippet": "SARAH_CHEN AFFILIATED_WITH MIT",
      "document_id": "doc-123",
      "file_path": "research_paper.pdf",
      "reference_id": 2
    },
    {
      "source_type": "chunk",
      "id": "doc-123-chunk-0",
      "score": 0.92,
      "snippet": "Sarah Chen published a groundbreaking paper on...",
      "document_id": "doc-123",
      "file_path": "research_paper.pdf",
      "start_line": 10,
      "end_line": 25,
      "chunk_index": 0,
      "reference_id": 3
    }
  ],
  "query_mode": "hybrid",
  "retrieval_time_ms": 45
}
```

**`done` event (enriched):**

```json
{
  "type": "done",
  "stats": {
    "embedding_time_ms": 12,
    "retrieval_time_ms": 45,
    "generation_time_ms": 1200,
    "total_time_ms": 1257,
    "sources_retrieved": 8,
    "tokens_used": 342,
    "tokens_per_second": 285.0,
    "query_mode": "hybrid"
  },
  "llm_provider": "ollama",
  "llm_model": "gemma3:12b"
}
```

**Chat-specific `done` event** (extends with message IDs):

```json
{
  "type": "done",
  "assistant_message_id": "550e8400-e29b-41d4-a716-446655440000",
  "stats": {
    "embedding_time_ms": 12,
    "retrieval_time_ms": 45,
    "generation_time_ms": 1200,
    "total_time_ms": 1257,
    "sources_retrieved": 8,
    "tokens_used": 342,
    "tokens_per_second": 285.0,
    "query_mode": "hybrid"
  },
  "llm_provider": "openai",
  "llm_model": "gpt-5-nano"
}
```

### Request DTO Changes

#### `/query/stream` — Expand `StreamQueryRequest`

Current:

```rust
pub struct StreamQueryRequest {
    pub query: String,
    pub mode: Option<String>,
    pub system_prompt: Option<String>,
}
```

Proposed:

```rust
pub struct StreamQueryRequest {
    pub query: String,
    pub mode: Option<String>,
    pub system_prompt: Option<String>,
    // --- New fields (SPEC-006) ---
    /// Document filter (same as FR-003 from SPEC-005).
    pub document_filter: Option<DocumentFilter>,
    /// LLM provider override (same as SPEC-032).
    pub llm_provider: Option<String>,
    /// LLM model override (same as SPEC-032).
    pub llm_model: Option<String>,
    /// Enable/disable reranking.
    pub enable_rerank: Option<bool>,
    /// Stream format version: "v1" (raw text) or "v2" (structured JSON events, default).
    pub stream_format: Option<String>,
}
```

#### New: `QueryStreamEvent` enum (for `/query/stream`)

```rust
/// Streaming SSE event types for the query endpoint.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueryStreamEvent {
    /// Context/sources retrieved before generation starts.
    Context {
        sources: Vec<SourceReference>,
        query_mode: String,
        retrieval_time_ms: u64,
    },
    /// Token generated during LLM streaming.
    Token { content: String },
    /// Chain-of-thought reasoning content.
    Thinking { content: String },
    /// Stream complete — includes full statistics.
    Done {
        stats: QueryStreamStats,
        #[serde(skip_serializing_if = "Option::is_none")]
        llm_provider: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        llm_model: Option<String>,
    },
    /// Error occurred during streaming.
    Error { message: String, code: String },
}

/// Statistics emitted in the `done` event.
#[derive(Debug, Clone, Serialize)]
pub struct QueryStreamStats {
    pub embedding_time_ms: u64,
    pub retrieval_time_ms: u64,
    pub generation_time_ms: u64,
    pub total_time_ms: u64,
    pub sources_retrieved: usize,
    pub tokens_used: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_per_second: Option<f32>,
    pub query_mode: String,
}
```

### `SourceReference` Enrichment

```rust
pub struct SourceReference {
    // --- Existing fields (unchanged) ---
    pub source_type: String,
    pub id: String,
    pub score: f32,
    pub rerank_score: Option<f32>,
    pub snippet: Option<String>,
    pub reference_id: Option<usize>,
    pub document_id: Option<String>,
    pub file_path: Option<String>,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub chunk_index: Option<usize>,
    // --- New fields (SPEC-006, FR-002) ---
    /// Entity type (e.g., "PERSON", "ORGANIZATION"). Only set for source_type="entity".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    /// Number of graph connections. Only set for source_type="entity".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub degree: Option<usize>,
    /// Source chunk IDs where entity was mentioned (provenance). Only set for source_type="entity".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_chunk_ids: Option<Vec<String>>,
}
```

---

## Implementation Plan

### Phase 1: Enrich `SourceReference` (Backend only, no breaking changes)

**Files to modify:**

| File                                                                                                                    | Change                                                                        |
| ----------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| [edgequake-api/src/handlers/query_types.rs](edgequake/crates/edgequake-api/src/handlers/query_types.rs)                 | Add `entity_type`, `degree`, `source_chunk_ids` fields to `SourceReference`   |
| [edgequake-api/src/handlers/chat/mod.rs](edgequake/crates/edgequake-api/src/handlers/chat/mod.rs)                       | Update `build_sources()` to populate new entity fields from `RetrievedEntity` |
| [edgequake-api/src/handlers/query/query_execute.rs](edgequake/crates/edgequake-api/src/handlers/query/query_execute.rs) | Update entity source building to populate new fields                          |

**Key code change in `build_sources()`:**

```rust
// BEFORE (current):
for entity in &context.entities {
    sources.push(SourceReference {
        source_type: "entity".to_string(),
        id: entity.name.clone(),
        score: entity.score,
        snippet: Some(entity.description.chars().take(200).collect()),
        entity_type: None,    // ← missing
        degree: None,         // ← missing
        source_chunk_ids: None, // ← missing
        // ...
    });
}

// AFTER (enriched):
for entity in &context.entities {
    sources.push(SourceReference {
        source_type: "entity".to_string(),
        id: entity.name.clone(),
        score: entity.score,
        snippet: Some(entity.description.chars().take(200).collect()),
        entity_type: Some(entity.entity_type.clone()),
        degree: if entity.degree > 0 { Some(entity.degree) } else { None },
        source_chunk_ids: if entity.source_chunk_ids.is_empty() {
            None
        } else {
            Some(entity.source_chunk_ids.clone())
        },
        // ...
    });
}
```

**Estimated effort:** Small. Additive fields with `skip_serializing_if`, zero breaking change risk.

### Phase 2: Upgrade `/query/stream` to Structured SSE

**Files to modify:**

| File                                                                                                                  | Change                                                                                             |
| --------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| [edgequake-api/src/handlers/query_types.rs](edgequake/crates/edgequake-api/src/handlers/query_types.rs)               | Add new fields to `StreamQueryRequest`, add `QueryStreamEvent` enum, add `QueryStreamStats` struct |
| [edgequake-api/src/handlers/query/query_stream.rs](edgequake/crates/edgequake-api/src/handlers/query/query_stream.rs) | Rewrite handler to use `query_stream_with_context()` and emit structured events                    |

**Handler rewrite (query_stream.rs):**

```rust
pub async fn stream_query(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<StreamQueryRequest>,
) -> ApiResult<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>> {
    validate_query(&request.query, state.config.max_query_length)?;

    let mode = request.mode.as_ref()
        .and_then(|m| QueryMode::parse(m))
        .unwrap_or(QueryMode::Hybrid);

    let mut engine_request = EngineQueryRequest::new(&request.query).with_mode(mode);
    // ... tenant/workspace resolution (existing code) ...

    let retrieval_start = std::time::Instant::now();

    // KEY CHANGE: Use query_stream_with_context instead of query_stream
    let (context, used_mode, stream) = state.sota_engine
        .query_stream_with_context(engine_request)
        .await
        .map_err(|e| ApiError::Internal(format!("Streaming query failed: {}", e)))?;

    let retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;

    // Build and emit events via channel
    let (tx, rx) = mpsc::channel::<QueryStreamEvent>(100);

    // Emit context event
    let sources = build_sources(&context);
    tx.send(QueryStreamEvent::Context {
        sources,
        query_mode: used_mode.to_string(),
        retrieval_time_ms,
    }).await;

    // Stream tokens
    tokio::spawn(async move {
        let gen_start = std::time::Instant::now();
        let mut accumulator = StreamAccumulator::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(text) => {
                    accumulator.append_content(&text);
                    let _ = tx.send(QueryStreamEvent::Token { content: text }).await;
                }
                Err(e) => {
                    let _ = tx.send(QueryStreamEvent::Error {
                        message: e.to_string(),
                        code: "STREAM_ERROR".to_string(),
                    }).await;
                    return;
                }
            }
        }

        // Emit done with stats
        let generation_time_ms = gen_start.elapsed().as_millis() as u64;
        let _ = tx.send(QueryStreamEvent::Done {
            stats: QueryStreamStats {
                embedding_time_ms: 0, // TODO: propagate from engine
                retrieval_time_ms,
                generation_time_ms,
                total_time_ms: retrieval_time_ms + generation_time_ms,
                sources_retrieved: context.chunks.len() + context.entities.len(),
                tokens_used: accumulator.estimated_tokens(),
                tokens_per_second: Some(
                    accumulator.estimated_tokens() as f32
                        / (generation_time_ms as f32 / 1000.0)
                ),
                query_mode: used_mode.to_string(),
            },
            llm_provider: None, // populated if LLM override used
            llm_model: None,
        }).await;
    });

    // Convert channel to SSE stream
    let sse_stream = ReceiverStream::new(rx).map(|event| {
        Ok(Event::default().data(serde_json::to_string(&event).unwrap_or_default()))
    });

    Ok(Sse::new(sse_stream))
}
```

**Estimated effort:** Medium. Requires handler rewrite but follows the existing pattern from `chat/streaming.rs`.

### Phase 3: Enrich Chat Streaming `done` Event with Stats

**Files to modify:**

| File                                                                                                          | Change                                                |
| ------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| [edgequake-api/src/handlers/chat_types.rs](edgequake/crates/edgequake-api/src/handlers/chat_types.rs)         | Add optional `stats` field to `ChatStreamEvent::Done` |
| [edgequake-api/src/handlers/chat/streaming.rs](edgequake/crates/edgequake-api/src/handlers/chat/streaming.rs) | Compute and emit timing stats in the `done` event     |

**Schema change:**

```rust
/// Stream complete - assistant message saved.
Done {
    assistant_message_id: Uuid,
    tokens_used: u32,
    duration_ms: u64,
    // --- New (SPEC-006) ---
    #[serde(skip_serializing_if = "Option::is_none")]
    stats: Option<QueryStreamStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    llm_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    llm_model: Option<String>,
},
```

**Context event enrichment:**

```rust
/// Context/sources retrieved.
Context {
    sources: Vec<SourceReference>,
    // --- New (SPEC-006) ---
    #[serde(skip_serializing_if = "Option::is_none")]
    query_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retrieval_time_ms: Option<u64>,
},
```

**Estimated effort:** Small. Additive fields with `skip_serializing_if`, serialization compatible.

### Phase 4: Frontend TypeScript Updates

**Files to modify:**

| File                                                                                                                   | Change                                                     |
| ---------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| [edgequake_webui/src/lib/api/chat.ts](edgequake_webui/src/lib/api/chat.ts)                                             | Update `ChatStreamEvent` type union with new fields        |
| [edgequake_webui/src/types/index.ts](edgequake_webui/src/types/index.ts)                                               | Update `QueryStreamChunk` type with structured events      |
| [edgequake_webui/src/lib/api/chat.ts](edgequake_webui/src/lib/api/chat.ts)                                             | Update `reduceStreamingEvent()` to handle new stats        |
| [edgequake_webui/src/lib/utils/source-mapper.ts](edgequake_webui/src/lib/utils/source-mapper.ts)                       | Map new `entity_type`, `degree`, `source_chunk_ids` fields |
| [edgequake_webui/src/components/query/source-citations.tsx](edgequake_webui/src/components/query/source-citations.tsx) | Display entity types and degree in citations               |

**TypeScript type changes:**

```typescript
// Updated ChatStreamEvent
export type ChatStreamEvent =
  | { type: "conversation"; conversation_id: string; user_message_id: string }
  | {
      type: "context";
      sources: SourceReference[];
      query_mode?: string;
      retrieval_time_ms?: number;
    }
  | { type: "token"; content: string }
  | { type: "thinking"; content: string }
  | {
      type: "done";
      assistant_message_id: string;
      tokens_used: number;
      duration_ms: number;
      stats?: QueryStreamStats;
      llm_provider?: string;
      llm_model?: string;
    }
  | { type: "title_update"; conversation_id: string; title: string }
  | { type: "error"; message: string; code: string };

// New stats type
export interface QueryStreamStats {
  embedding_time_ms: number;
  retrieval_time_ms: number;
  generation_time_ms: number;
  total_time_ms: number;
  sources_retrieved: number;
  tokens_used: number;
  tokens_per_second?: number;
  query_mode: string;
}

// Updated SourceReference
export interface SourceReference {
  source_type: string;
  id: string;
  score: number;
  rerank_score?: number;
  snippet?: string;
  reference_id?: number;
  document_id?: string;
  file_path?: string;
  // New (SPEC-006)
  entity_type?: string;
  degree?: number;
  source_chunk_ids?: string[];
}
```

**Estimated effort:** Small-Medium. Type additions, no structural changes.

### Phase 5: SDK Updates (Python, TypeScript, Java)

Update any published SDK types to match the enriched protocol. This is an additive change — new optional fields — so existing SDK consumers continue to work.

---

## Roadblocks and Risks

### RB-1: Timing Metrics Propagation from Engine

**Risk:** The SOTA engine's `query_stream_with_context()` does not return embedding or retrieval timing separately. The `QueryContext` has no timing fields.

**Mitigation:** Wrap the engine call with `Instant::now()` measurements at the API handler level. This gives accurate wall-clock times for:

- Retrieval: time from handler start to `query_stream_with_context()` returning
- Generation: time from first token to last token

For `embedding_time_ms` specifically, the engine would need to be extended to report it. As a first pass, include it in `retrieval_time_ms` (they overlap since embedding is part of retrieval).

### RB-2: Backward Compatibility for `/query/stream`

**Risk:** Existing clients consuming `/query/stream` as raw text will break if the event format changes to structured JSON.

**Mitigation options:**

1. **Version parameter** (`stream_format=v1` for raw text, `v2` for structured, default `v2`) — adds complexity but safe
2. **New endpoint** (`/query/stream/v2`) — clean separation but URL proliferation
3. **Break without version** — acceptable if the old endpoint had minimal external adoption

**Recommendation:** Option 1 (version parameter) for the `/query/stream` endpoint. The chat endpoint already uses structured events and doesn't need versioning.

### RB-3: `entity_type` Lost in Conversion

**Risk:** The `build_sources()` function currently drops `entity_type` from `RetrievedEntity` because `SourceReference` doesn't have the field.

**Mitigation:** Phase 1 adds the field. Low risk — purely additive struct change with `skip_serializing_if`.

### RB-4: Streaming Context Size

**Risk:** For queries that match many entities/relationships/chunks, the `context` event could be large (100+ source references). This is sent as a single SSE event before tokens.

**Mitigation:**

- The context is already truncated by the engine's `balance_context()` function (token budget enforcement)
- Typical context: 10-30 chunks, 5-15 entities, 5-10 relationships → ~10-20 KB JSON
- This is well within SSE limits and browser memory

### RB-5: Workspace-Specific Providers for `/query/stream`

**Risk:** The current `/query/stream` handler does not support workspace-specific embedding providers or LLM overrides (unlike the chat handler which has full support).

**Mitigation:** Phase 2 handler rewrite should adopt the same provider resolution pattern from `chat/streaming.rs`:

- `WorkspaceProviderResolver` for LLM selection
- `get_workspace_embedding_provider()` + `get_workspace_vector_storage()` for workspace isolation
- This is copy-adapt from existing code, not new development

### RB-6: No `query_stream_with_full_config` Context Return

**Risk:** The `query_stream_with_full_config()` engine method (used for workspace isolation) already returns `(QueryContext, QueryMode, BoxStream)`, which is exactly what's needed. No engine changes required.

**Status:** ✅ No risk — engine API already supports this.

---

## Dependency Map

```
Phase 1: SourceReference enrichment
  ├── No dependencies
  └── Can be merged independently

Phase 2: /query/stream upgrade
  ├── Depends on Phase 1 (enriched SourceReference)
  ├── Uses existing engine methods (no engine changes)
  └── Requires timing instrumentation in handler

Phase 3: Chat streaming stats enrichment
  ├── Depends on Phase 1 (enriched SourceReference)
  └── Can be merged independently from Phase 2

Phase 4: Frontend TypeScript updates
  ├── Depends on Phase 1 (new SourceReference fields)
  ├── Depends on Phase 3 (new stats in events)
  └── Safe to merge after backend phases

Phase 5: SDK updates
  └── Depends on all backend phases being released
```

---

## Test Strategy

### Unit Tests (Rust)

| Test                                                              | Validates |
| ----------------------------------------------------------------- | --------- |
| `SourceReference` with `entity_type` serializes correctly         | Phase 1   |
| `QueryStreamEvent` enum serialization (all variants)              | Phase 2   |
| `build_sources()` populates entity_type, degree, source_chunk_ids | Phase 1   |
| `QueryStreamStats` serialization includes all timing fields       | Phase 2   |
| Backward compatibility: v1 format produces raw text events        | Phase 2   |

### Integration Tests (Rust)

| Test                                                                     | Validates |
| ------------------------------------------------------------------------ | --------- |
| `POST /query/stream` returns structured events with context              | Phase 2   |
| `POST /query/stream?stream_format=v1` returns raw text (backward compat) | Phase 2   |
| `POST /chat/completions/stream` includes stats in done event             | Phase 3   |
| Context event includes entities with `entity_type` field                 | Phase 1+2 |

### E2E Tests (Frontend)

| Test                                                       | Validates |
| ---------------------------------------------------------- | --------- |
| Streaming chat shows entity type badges in citations       | Phase 4   |
| Stats panel displays retrieval/generation timing breakdown | Phase 4   |
| Query via `/query/stream` v2 renders citations correctly   | Phase 4   |

---

## Timeline Estimate

| Phase   | Scope                      | Dependencies |
| ------- | -------------------------- | ------------ |
| Phase 1 | SourceReference enrichment | None         |
| Phase 2 | `/query/stream` rewrite    | Phase 1      |
| Phase 3 | Chat streaming stats       | Phase 1      |
| Phase 4 | Frontend updates           | Phase 1, 3   |
| Phase 5 | SDK updates                | All backend  |

---

## Decision Log

| #   | Decision                                                    | Rationale                                                                                                    |
| --- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| D-1 | Use typed JSON SSE events (not separate SSE `event:` field) | Aligns with existing chat endpoint, simpler parsing, matches OpenAI pattern                                  |
| D-2 | Send context BEFORE tokens                                  | Allows UI to show "retrieval complete" indicator and pre-render citation skeletons before LLM output arrives |
| D-3 | Add `stream_format` version parameter for `/query/stream`   | Backward compatibility for existing raw-text consumers without endpoint proliferation                        |
| D-4 | Enrich `SourceReference` in-place (not new struct)          | Avoids type explosion, additive optional fields are non-breaking                                             |
| D-5 | Emit `stats` inside `done` event (not separate event)       | Reduces event count, stats are inherently tied to completion                                                 |
| D-6 | Reuse `build_sources()` for `/query/stream`                 | Single source of truth for SourceReference construction, DRY                                                 |

---

## Appendix: Current vs. Proposed Event Flows

### `/query/stream` — Current (v1)

```
SSE: data: The answer to your question is...
SSE: data: [more text tokens...]
SSE: data: Based on the retrieved context...
```

_(No structure, no sources, no stats.)_

### `/query/stream` — Proposed (v2, default)

```
SSE: data: {"type":"context","sources":[...],"query_mode":"hybrid","retrieval_time_ms":42}
SSE: data: {"type":"token","content":"The answer "}
SSE: data: {"type":"token","content":"to your question "}
SSE: data: {"type":"token","content":"is based on..."}
SSE: data: {"type":"done","stats":{"embedding_time_ms":10,"retrieval_time_ms":42,"generation_time_ms":1100,"total_time_ms":1152,"sources_retrieved":8,"tokens_used":156,"query_mode":"hybrid"},"llm_provider":"ollama","llm_model":"gemma3:12b"}
```

### `/chat/completions/stream` — Current

```
SSE: data: {"type":"conversation","conversation_id":"...","user_message_id":"..."}
SSE: data: {"type":"context","sources":[...]}
SSE: data: {"type":"token","content":"The answer "}
SSE: data: {"type":"token","content":"..."}
SSE: data: {"type":"done","assistant_message_id":"...","tokens_used":156,"duration_ms":1200}
```

### `/chat/completions/stream` — Proposed (enriched)

```
SSE: data: {"type":"conversation","conversation_id":"...","user_message_id":"..."}
SSE: data: {"type":"context","sources":[{"source_type":"entity","id":"SARAH_CHEN","entity_type":"PERSON","degree":12,...}],"query_mode":"hybrid","retrieval_time_ms":42}
SSE: data: {"type":"token","content":"The answer "}
SSE: data: {"type":"token","content":"..."}
SSE: data: {"type":"done","assistant_message_id":"...","tokens_used":156,"duration_ms":1200,"stats":{"embedding_time_ms":10,"retrieval_time_ms":42,"generation_time_ms":1100,"total_time_ms":1152,"sources_retrieved":8,"tokens_used":156,"query_mode":"hybrid"},"llm_provider":"ollama","llm_model":"gemma3:12b"}
```

---

## References

- [GitHub Issue #56](https://github.com/raphaelmansuy/edgequake/issues/56)
- [OpenAI Streaming API — Typed SSE Events](https://platform.openai.com/docs/api-reference/streaming)
- SPEC-004: System Prompt Extension Point
- SPEC-005: Document Date and Pattern Filters
- SPEC-032: Workspace-Specific Embedding and Provider Selection
