# 009 — Edge Cases & Invariants

**Spec:** 028-edgequake-query-service  
**Date:** 2026-06-28  
**Cross-ref:** [005-dto-model-contract.md](./005-dto-model-contract.md) | [003-code-is-law-current-pipeline.md](./003-code-is-law-current-pipeline.md)

---

## Edge Case Catalog

### EC-01 — Empty Retrieval (No Matching Content)

**Trigger:** Query about topic not in corpus; over-restrictive document_filter.

**Behavior:**
- HTTP 200 (not error)
- `bundle.subgraph.entities = []`, `bundle.chunks = []`
- `retrieval_quality.empty_context = true`
- `retrieval_quality.coverage_score = 0.0`
- `agent_hints.suggested_followups` may suggest broader queries

**Agent implication:** Iterate with broader query or different mode — do not treat as failure.

---

### EC-02 — Truncation at Token Budget

**Trigger:** `balance_context` exceeds 30k token default.

**Behavior:**
- `truncation.is_truncated = true`
- `truncation.dropped.{chunks,entities,relationships}` > 0
- Items in bundle are **kept** items (highest score first)
- `agent_hints` includes `"truncation_applied": true`

**Agent implication:** Fetch with narrower filter, increase specificity, or request `max_results` reduction then re-query subtopics.

---

### EC-03 — Document Filter Matches Zero Documents

**Trigger:** `document_filter` date range excludes all docs.

**Behavior:**
- Same as EC-01 — empty bundle, 200 OK
- Stats include `document_filter_applied: true`, `documents_matched: 0`

**Distinction from EC-01:** Agent should relax filter, not change query text.

---

### EC-04 — Keyword LLM Unavailable

**Trigger:** Keyword extraction enabled but LLM provider down.

**Behavior:**
- Fallback: embedding-only retrieval (existing engine behavior)
- `stats.keywords_extracted = []`
- `mode_selection.adaptive = false`
- Log warning; no 5xx if retrieval succeeds

---

### EC-05 — Embedding Provider Failure

**Trigger:** Ollama/OpenAI embedding endpoint unreachable.

**Behavior:**
- HTTP 502 `EMBEDDING_FAILED`
- No partial bundle
- MCP: JSON-RPC -32603

**Agent implication:** Retry with backoff; switch workspace embedding if multi-provider.

---

### EC-06 — Reranker Failure

**Trigger:** Cohere rerank API error when `enable_rerank=true`.

**Behavior:**
- **Fail open:** return pre-rerank results (existing pattern)
- `stats.reranked = false`
- `rerank_error` in stats metadata (debug tier only)

---

### EC-07 — Injection Documents Excluded

**Trigger:** Knowledge injection docs with `document_id` prefix `injection::`.

**Behavior:**
- Excluded from bundle (existing citation rule)
- Not listed in `documents[]`
- If only matches were injection docs → EC-01

**Code ref:** `query_execute.rs` injection exclusion.

---

### EC-08 — Cross-Workspace Access Attempt

**Trigger:** Agent passes wrong `workspace_id` or token scoped to workspace A requests B.

**Behavior:**
- HTTP 403 `FORBIDDEN`
- No data leakage (empty body)
- RLS enforced at storage layer (defense in depth)

---

### EC-09 — Expired retrieval_id (MCP Fetch)

**Trigger:** Fetch after TTL (default 15 min).

**Behavior:**
- HTTP 410 Gone
- Body: `{ "code": "RETRIEVAL_EXPIRED", "message": "Re-run edgequake_search" }`
- MCP: -32004

**Agent implication:** Re-run search — do not cache retrieval_id long-term.

---

### EC-10 — Concurrent Identical Queries

**Trigger:** Agent loop fires parallel identical retrieves.

**Behavior:**
- Engine result cache deduplicates (context_only path)
- Second response: `cached: true`
- Same `retrieval_fingerprint`

---

### EC-11 — Bypass Mode on Context Endpoint

**Trigger:** `mode: "bypass"` on `/query/context`.

**Behavior:**
- HTTP 400 `INVALID_MODE`
- Bypass skips retrieval — meaningless for context service
- Bypass remains on `/query` only

---

### EC-12 — Conversation History Token Overflow

**Trigger:** Long multi-turn history in `conversation_history`.

**Behavior:**
- Engine truncates history for keyword extraction (existing)
- Full history echoed in response metadata count only
- `stats.conversation_turns_used: N`

---

### EC-13 — Entity Without Chunk Provenance

**Trigger:** Graph entity exists but source chunks deleted.

**Behavior:**
- Entity included with `lineage.source_chunk_ids = []`
- `lineage.source_document_id` populated if known
- `agent_hints.data_quality_warnings: ["entity_missing_chunk_provenance"]`

---

### EC-14 — Chunk Hydration from KV Fails

**Trigger:** Vector index returns chunk ID; KV body missing.

**Behavior:**
- Chunk omitted from bundle (not empty placeholder)
- `truncation.dropped.chunks` incremented
- Warning log with chunk_id

---

### EC-15 — Mix Mode with Zero Weight Arm

**Trigger:** `mix_weights: { "local": 0, "global": 0, "naive": 1 }`.

**Behavior:**
- Only naive arm runs (SPEC-022 verified)
- `stats.mode_arms.local` absent or zero counts

---

### EC-16 — content_granularity=citation via /query/context

**Trigger:** Agent explicitly requests citation tier.

**Behavior:**
- Valid — returns snippets
- `agent_hints` includes warning: `"granularity_citation_not_recommended_for_agents"`

---

### EC-17 — Very Large Single Chunk

**Trigger:** One chunk exceeds token budget alone.

**Behavior:**
- Chunk truncated at token boundary
- `chunks[0].content` truncated with `\n...[truncated]` suffix
- `chunks[0].is_truncated = true`

---

### EC-18 — Adaptive Mode Selection

**Trigger:** Mode omitted, adaptive enabled in engine config.

**Behavior:**
- `mode_selection.adaptive = true`
- `mode_selection.effective` = selected mode
- `mode_selection.intent` = QueryIntent enum string

---

## Invariant Checklist (Must Hold in All Cases)

| ID | Invariant |
|----|-----------|
| INV-01 | Every chunk in bundle has `lineage.document_id` OR explicit null with warning |
| INV-02 | `retrieval_fingerprint` deterministic for same inputs + corpus version |
| INV-03 | No answer LLM call in `QueryContextService::retrieve` |
| INV-04 | Workspace tenant_id matches RLS context for all storage reads |
| INV-05 | injection:: documents never appear in bundle |
| INV-06 | `reference_id` sequential 1..N within chunks array |
| INV-07 | Entity `id` stable across fetches of same retrieval_id |
| INV-08 | GET fetch returns identical bundle to POST retrieve (same granularity) |

---

## Failure Mode Summary Table

| Category | Retry? | Broaden query? | Change mode? | Relax filter? |
|----------|--------|----------------|--------------|---------------|
| EC-01 Empty | — | ✅ | ✅ | ✅ |
| EC-02 Truncated | — | ✅ subtopic | — | ✅ narrower docs |
| EC-03 Filter empty | — | — | — | ✅ |
| EC-05 Embed fail | ✅ backoff | — | — | — |
| EC-09 Expired id | ✅ re-search | — | — | — |

---

## Load & Abuse Edge Cases

| Case | Mitigation |
|------|------------|
| Agent infinite loop | Client-side step budget; future rate limit per API key |
| Huge `max_results` | Cap at engine config max (default 60) |
| Retrieval ID enumeration | UUID v4; auth required; 404 for wrong tenant |
| Cache stampede | Single-flight dedup on fingerprint |

---

## Test Matrix (E2E)

Each EC-xx maps to `spec028_context_e2e` test function:

```
ec_01_empty_context_returns_200
ec_03_filter_zero_documents
ec_08_cross_workspace_forbidden
ec_09_expired_retrieval_id_410
ec_11_bypass_mode_rejected
ec_14_missing_kv_chunk_omitted
```
