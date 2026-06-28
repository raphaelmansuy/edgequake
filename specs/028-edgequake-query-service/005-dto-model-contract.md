# 005 — DTO Model Contract (Agent-Grade)

**Spec:** 028-edgequake-query-service  
**Date:** 2026-06-28  
**Cross-ref:** [004-context-service-architecture.md](./004-context-service-architecture.md) | [006-api-surface-lens.md](./006-api-surface-lens.md)  
**OpenAPI:** `edgequake-api/src/handlers/context_types.rs` (new)

---

## Design Principles

1. **Graph-native** — entities and relationships are first-class arrays, not a flattened `sources[]`.
2. **Provenance-complete** — every artifact links to document + chunk lineage.
3. **Tiered payload** — `content_granularity` controls size (citation vs agent vs debug).
4. **Agent signals** — coverage, truncation, suggested follow-ups for loop control.
5. **Stable IDs** — MCP-compatible search/fetch handles.
6. **JSON Schema 2020-12** — MCP SEP-2106 aligned tool schemas.

---

## Request DTO

### ContextRetrievalRequest

```json
{
  "query": "How does EdgeQuake relate to LightRAG entity extraction?",
  "mode": "mix",
  "content_granularity": "agent",
  "max_results": 20,
  "conversation_history": [
    { "role": "user", "content": "What is EdgeQuake?" },
    { "role": "assistant", "content": "EdgeQuake is a RAG framework..." }
  ],
  "document_filter": {
    "date_from": "2025-01-01T00:00:00Z",
    "document_pattern": "spec,architecture"
  },
  "mix_weights": { "local": 0.4, "global": 0.3, "naive": 0.3 },
  "enable_rerank": true,
  "rerank_top_k": 10,
  "include_lineage": true,
  "include_documents": true,
  "include_agent_hints": true
}
```

| Field | Type | Default | Required | Notes |
|-------|------|---------|----------|-------|
| `query` | string | — | ✅ | Natural language query |
| `mode` | enum | `mix` | — | naive\|local\|global\|hybrid\|mix |
| `content_granularity` | enum | `agent` | — | citation\|agent\|debug |
| `max_results` | usize | engine default | — | Chunk cap |
| `conversation_history` | array | null | — | Multi-turn context for keyword extract |
| `document_filter` | DocumentFilter | null | — | SPEC-005 SSOT |
| `mix_weights` | MixWeightRequest | null | — | SPEC-022 |
| `enable_rerank` | bool | true | — | Post-retrieval rerank |
| `rerank_model` | string | null | — | Optional reranker |
| `rerank_top_k` | usize | null | — | Post-rerank cap |
| `include_lineage` | bool | true | — | Document lineage blocks |
| `include_documents` | bool | true | — | Document metadata section |
| `include_agent_hints` | bool | true | — | coverage, follow-ups |
| `include_subgraph` | bool | true | — | Query-matched entities + relationships in `bundle.subgraph` |

**Not included:** `llm_provider`, `system_prompt`, `prompt_only` — generation concerns stay on `/query`.

**Workspace:** from auth context / header (same as `/query` — SPEC-027 isolation).

---

## Response DTO

### ContextRetrievalResponse (top-level)

```json
{
  "retrieval_id": "ret_7f3a9c2e-4b1d-4e8a-9f0c-1d2e3f4a5b6c",
  "query": "How does EdgeQuake relate to LightRAG entity extraction?",
  "mode": "mix",
  "mode_selection": {
    "requested": "mix",
    "effective": "mix",
    "adaptive": false,
    "intent": null
  },
  "bundle": { "...": "ContextBundle — see below" },
  "stats": { "...": "RetrievalStats — see below" },
  "retrieval_quality": {
    "coverage_score": 0.82,
    "is_sufficient": true,
    "empty_context": false
  },
  "truncation": {
    "is_truncated": false,
    "token_budget": 30000,
    "tokens_used": 8420,
    "dropped": { "chunks": 0, "entities": 0, "relationships": 0 }
  },
  "agent_hints": {
    "suggested_followups": [
      "What entity normalization rules does EdgeQuake use?",
      "Compare EdgeQuake Local mode with LightRAG low-level retrieval"
    ],
    "dominant_entity_types": ["TECHNOLOGY", "CONCEPT"],
    "documents_touched": 3
  },
  "retrieval_fingerprint": "sha256:abc123...",
  "cached": false
}
```

---

### ContextBundle (core payload)

```json
{
  "subgraph": {
    "entities": [
      {
        "id": "ent:LIGHT_RAG",
        "name": "LIGHT_RAG",
        "entity_type": "TECHNOLOGY",
        "description": "Lightweight graph-augmented RAG framework...",
        "score": 0.91,
        "degree": 12,
        "lineage": {
          "source_chunk_ids": ["chk_001", "chk_042"],
          "source_document_id": "doc_spec024",
          "source_file_path": "specs/024-lightrag-expert-lens.md",
          "start_line": 45,
          "end_line": 89
        }
      }
    ],
    "relationships": [
      {
        "id": "rel:EDGEQUAKE_IMPLEMENTS_LIGHT_RAG",
        "source": "EDGEQUAKE",
        "target": "LIGHT_RAG",
        "relation_type": "IMPLEMENTS",
        "description": "EdgeQuake implements dual-level retrieval aligned with LightRAG",
        "score": 0.87,
        "lineage": {
          "source_chunk_id": "chk_042",
          "source_document_id": "doc_spec024",
          "source_file_path": "specs/024-lightrag-expert-lens.md"
        }
      }
    ]
  },
  "chunks": [
    {
      "id": "chk_042",
      "content": "LightRAG introduces dual-level retrieval...",
      "score": 0.89,
      "rerank_score": 0.94,
      "token_count": 412,
      "reference_id": 1,
      "lineage": {
        "document_id": "doc_spec024",
        "file_path": "specs/024-lightrag-expert-lens.md",
        "start_line": 45,
        "end_line": 89,
        "chunk_index": 3
      }
    }
  ],
  "documents": [
    {
      "document_id": "doc_spec024",
      "title": "024-lightrag-expert-lens.md",
      "mime_type": "text/markdown",
      "created_at": "2026-01-15T10:00:00Z",
      "chunk_count_in_bundle": 2,
      "entity_count_in_bundle": 3
    }
  ],
  "context_string": null
}
```

**`context_string`:** populated only when `content_granularity=debug` — exact LLM prompt context.

---

### Content Granularity Tiers

| Tier | chunks[].content | subgraph | context_string | Use case |
|------|------------------|----------|----------------|----------|
| `citation` | snippet (200 chars) | entities without full description | null | UI compat, legacy |
| `agent` | **full text** | **full** | null | **default for /query/context** |
| `debug` | full text | full + metadata | **included** | prompt inspection |

---

### RetrievalStats

```json
{
  "embedding_time_ms": 45,
  "retrieval_time_ms": 230,
  "rerank_time_ms": 89,
  "total_time_ms": 364,
  "items_retrieved": {
    "chunks": 8,
    "entities": 5,
    "relationships": 4,
    "documents": 3
  },
  "keywords_extracted": ["LightRAG", "entity extraction", "EdgeQuake"],
  "mode_arms": {
    "naive": { "chunks": 3, "time_ms": 80 },
    "local": { "entities": 4, "chunks": 2, "time_ms": 95 },
    "global": { "relationships": 3, "chunks": 3, "time_ms": 55 }
  },
  "embedding_model": "embeddinggemma:latest",
  "reranked": true
}
```

---

## Search / Fetch DTOs (MCP)

### ContextSearchRequest

```json
{
  "query": "EdgeQuake entity normalization",
  "mode": "local",
  "max_results": 5
}
```

### ContextSearchResponse

```json
{
  "results": [
    {
      "retrieval_id": "ret_7f3a9c2e-...",
      "title": "Entity normalization in EdgeQuake pipeline",
      "snippet": "Entity names are normalized to UPPERCASE with underscores...",
      "url": "edgequake://workspace/default/retrieval/ret_7f3a9c2e-...",
      "score": 0.91,
      "metadata": {
        "mode": "local",
        "entity_count": 3,
        "chunk_count": 2
      }
    }
  ]
}
```

### ContextFetchRequest

```json
{
  "retrieval_id": "ret_7f3a9c2e-4b1d-4e8a-9f0c-1d2e3f4a5b6c",
  "content_granularity": "agent"
}
```

Returns full `ContextRetrievalResponse`.

---

## Rust Type Mapping

| DTO | Rust struct | Crate |
|-----|-------------|-------|
| ContextRetrievalRequest | `ContextRetrievalRequest` | edgequake-api |
| ContextBundle | `ContextBundle` | edgequake-api |
| SubgraphBundle | `SubgraphBundle` | edgequake-api |
| ContextEntity | `ContextEntity` | edgequake-api |
| ContextRelationship | `ContextRelationship` | edgequake-api |
| ContextChunk | `ContextChunk` | edgequake-api |
| LineageRef | `LineageRef` | edgequake-api |
| DocumentSummary | `DocumentSummary` | edgequake-api |

**Engine mapping:**

```
QueryContext.chunks      → ContextBundle.chunks
QueryContext.entities    → ContextBundle.subgraph.entities
QueryContext.relationships → ContextBundle.subgraph.relationships
KV metadata scan         → ContextBundle.documents
QueryContext.to_context_string() → ContextBundle.context_string (debug only)
```

---

## Backward Compatibility: Legacy SourceReference

`/query` with `context_only=true` continues returning `QueryResponse` with flat `sources[]`.

Mapping function:

```
ContextBundle ──to_legacy_sources(granularity=citation)──► Vec<SourceReference>
```

Contract test: legacy output == current output byte-for-byte (QRY-003).

---

## JSON Schema Annotations (MCP tools)

Tool `edgequake_search` input schema root: `{ "type": "object", "properties": { ... } }` (SEP-2106).

Tool output: `ContextSearchResponse` schema registered in MCP `tools/list` with `ttlMs: 300000`.

---

## Field ID Convention

| Prefix | Meaning |
|--------|---------|
| `ret_` | Retrieval bundle handle |
| `chk_` | Chunk ID (from storage) |
| `ent:` | Entity name namespaced ID |
| `rel:` | Relationship composite ID |
| `doc_` | Document ID |

Agents should use `retrieval_id` for fetch, `chunks[].id` / `entities[].id` for citation within a bundle.
