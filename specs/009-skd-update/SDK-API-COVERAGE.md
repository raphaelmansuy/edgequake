# SDK ↔ HTTP API coverage (brutal assessment)

**Authority:** `edgequake/crates/edgequake-api/src/routes.rs` + [API-CANON.md](./API-CANON.md).  
**Last updated:** 2026-04-24 (iteration 3 — document list law + Tier 2 bulk bodies + docs/sdks).  
**Scope:** Routed REST surface vs **first-class** client methods and **primary** DTO shapes.

## How to read this

| Symbol | Meaning |
|--------|---------|
| ✅ | Correct path + verb; response wrapper matches handlers for the common case. |
| ⚠️ | Wrapped, but streaming, optional fields, or niche params still thinner than OpenAPI. |
| ❌ | No dedicated client method (raw HTTP only). |
| — | Out of SDK scope or not a REST route. |

OpenAPI remains the source of truth for every field on `ConversationResponse`, `MessageResponse`, etc.

---

## Tier-1 SDKs (Rust, Python, TypeScript)

### Unversioned & misc

| Endpoint | Rust | Python | Node/TS | Notes |
|----------|------|--------|---------|--------|
| `GET /health` | ✅ | ✅ | ✅ | |
| `GET /ready`, `/live`, `/metrics` | ✅ text | ✅ | ✅ | |
| `GET /ws/...` | — | — | ⚠️ | TS helpers exist; not a full WS spec. |

### Auth through workspaces

| Area | Rust | Python | Node/TS |
|------|------|--------|---------|
| Auth, users, keys, tenants, workspaces, admin, effective config | ✅ | ✅ | ✅ |

### Documents & PDF

| Area | Rust | Python | Node/TS | Notes |
|------|------|--------|---------|--------|
| Documents list — `page`, `page_size`, `date_from`, `date_to`, `document_pattern` | ✅ `list_with_query` | ✅ `DocumentListParams` | ✅ `ListDocumentsQuery` |
| Documents list — full response (`total`, `has_more`, `status_counts`, …) | ✅ | ✅ | ✅ |
| Documents + PDF family | ✅ | ✅ | ✅ | |
| PDF progress SSE | ⚠️ | ⚠️ | ⚠️ | Raw stream handling varies. |

### Query & chat

| Area | Rust | Python | Node/TS |
|------|------|--------|---------|
| Query + stream | ✅ | ✅ | ✅ |
| Chat + stream | ✅ | ✅ | ✅ |

### Conversations (aligned with `ListConversationsParams` / `ListMessagesParams`)

| Capability | Rust | Python | Node/TS |
|------------|------|--------|---------|
| List — `cursor`, `limit`, `filter[…]`, `sort`, `order` | ✅ `list_with_query` + encode helper | ✅ `ConversationListParams` + legacy shim | ✅ `ListConversationsQuery` |
| List — paginated response `{ items, pagination }` | ✅ | ✅ `PaginatedConversations` | ✅ |
| Get — `{ conversation, messages }` | ✅ | ✅ validator | ✅ |
| Messages list — `cursor`, `limit`, paginated | ✅ `list_messages_with_query` | ✅ `ListMessagesParams` + `PaginatedMessages` | ✅ |
| PATCH/DELETE `/messages/{id}` | ✅ | ✅ | ✅ |
| Bulk ops — `conversation_ids`, `affected` | ✅ | ✅ | ✅ |
| `bulk_move` — optional `folder_id` | ✅ `Option<&str>` | ✅ already optional | ✅ |
| `GET /shared/{share_id}` | ✅ `client.shared()` | ✅ | ✅ |
| Share DTO — `share_url` (+ `url` alias) | ✅ | ✅ | ✅ |

### Graph, tasks, pipeline, costs

| Area | Rust | Python | Node/TS | Notes |
|------|------|--------|---------|--------|
| Graph / entities / relationships | ✅ | ✅ | ✅ | |
| Tasks | ✅ | ✅ | ✅ | |
| Pipeline | ✅ | ✅ | ✅ | |
| Costs summary/history/budget | ✅ | ✅ | ✅ | |
| Pipeline costs pricing + estimate | ✅ | ✅ | ✅ | TS: **single DRY path** in [`api-paths.ts`](../../sdks/typescript/src/constants/api-paths.ts) shared by `PipelineResource` + `CostsResource`. |
| Invented `GET /costs/workspace` | — | — | ❌ removed | Never in `routes.rs`. |

### Lineage, settings, models

| Area | Rust | Python | Node/TS |
|------|------|--------|---------|
| Lineage, chunks, provenance | ✅ | ✅ | ✅ |
| Settings + models | ✅ | ✅ | ✅ |

---

## Tier-2+ SDKs (Swift, Kotlin, Go, Ruby, Java, C#, …)

These track the same canon; conversation **list query** parity may still lag Tier 1. **Bulk conversation** requests now use **`conversation_ids`** and responses **`affected`** in: Kotlin, Swift, Go, Java, C# (Java: JSON aliases for legacy mock shapes). When changing `routes.rs`, update using [API-CANON.md](./API-CANON.md). Human-readable SDK guides: [docs/sdks/README.md](../../docs/sdks/README.md). Brutal tiering: [docs/sdks/BRUTAL-ASSESSMENT.md](../../docs/sdks/BRUTAL-ASSESSMENT.md).

---

## Design notes (SOLID / DRY)

1. **Single encoding of filter query keys** — Rust: `conversation_list_query_string` in `types/conversations.rs` (one place). Python: `ConversationListParams.to_query_dict` + `_conversation_list_query_dict` for legacy kwargs only.
2. **TS pipeline cost URLs** — `PIPELINE_COSTS_PRICING_PATH` / `PIPELINE_COSTS_ESTIMATE_PATH` consumed by both `pipeline.ts` and `costs.ts` so path drift cannot recur.
3. **Default `list()`** — All three tier-1 SDKs use server defaults when no query is supplied (no fake `page`/`page_size` on the wire unless callers pass Python legacy kwargs).

---

## Residual ⚠️ (honest)

1. **Full OpenAPI field parity** on every struct (tokens, context blobs, tenant UUIDs) — still trimmed for ergonomics.
2. **SSE** for PDF progress / graph stream — clients expose bytes or helpers, not a typed event model everywhere.
3. **WebSocket** progress — not modeled uniformly across languages.

---

## Full re-assessment (2026-04-24 — iteration 3)

| Question | Answer |
|----------|--------|
| Are tier-1 conversation **list** and **messages** queries canon-complete? | **Yes** for `cursor`, `limit`, bracket filters, `sort`, `order` (Rust + Python + TS). |
| Are tier-1 **document list** queries canon-complete? | **Yes** — `page`, `page_size`, `date_from`, `date_to`, `document_pattern` (no spurious `status`/`search` on the wire). |
| Are tier-1 conversation **responses** canon-shaped? | **Yes** for paginated wrappers and nested get/shared bodies (Python + Rust + TS). |
| Are bulk conversation bodies canon-shaped? | **Yes** in Tier 1 + Tier 2 JVM/Go/Kotlin/Swift/C# (`conversation_ids`, `affected`). |
| Is cost estimation routed correctly in TS? | **Yes**, with **DRY** path constants. |
| What still needs human judgment? | Tier-2 conversation **list** filter parity; SSE/WebSocket; exhaustive OpenAPI field coverage. |

**Verdict:** **Tier 1** matches `routes.rs` for documents list, conversations, and costs paths addressed here. **Tier 2** bulk conversation POST bodies were a real drift; fixed in-repo for the listed languages. **Ruby** remains unreliable until `lib/` packaging is restored ([BRUTAL-ASSESSMENT.md](../../docs/sdks/BRUTAL-ASSESSMENT.md)).

---

## Cross-references

- [API-CANON.md](./API-CANON.md)  
- [RUST-SDK-GAP.md](./RUST-SDK-GAP.md)  
- [python-skd/GAP.md](./python-skd/GAP.md)  
- [node-js/GAP.md](./node-js/GAP.md)
