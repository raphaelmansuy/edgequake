# 007 — MCP Exposure Lens

**Spec:** 028-edgequake-query-service  
**Date:** 2026-06-28  
**Cross-ref:** [006-api-surface-lens.md](./006-api-surface-lens.md) | [005-dto-model-contract.md](./005-dto-model-contract.md)  
**Protocol baseline:** MCP 2026-07-28 Release Candidate

---

## Why MCP

External agents (Cursor, ChatGPT, Claude Desktop, custom LangGraph) connect via **Model Context Protocol** — not bespoke REST. EdgeQuake already has REST; MCP is a **thin adapter** over `QueryContextService` ([Tetrate MCP vs RAG](https://tetrate.io/learn/ai/mcp/mcp-vs-alternatives)).

```
  ┌──────────────┐         MCP (HTTP)          ┌─────────────────────┐
  │ Agent Host   │ ──── tools/call ──────────► │ edgequake-api       │
  │ (Cursor,etc) │ ◄─── JSON-RPC result ────── │ /mcp (future mount) │
  └──────────────┘                             └──────────┬──────────┘
                                                          │
                                                          v
                                               QueryContextService
                                               (same as REST SSOT)
```

---

## MCP 2026-07-28 Constraints

| Requirement | EdgeQuake compliance |
|-------------|---------------------|
| **Stateless** — no `Mcp-Session-Id` | ✅ `retrieval_id` as explicit handle |
| **`Mcp-Method` + `Mcp-Name` headers** | ✅ On Streamable HTTP transport |
| **JSON Schema 2020-12** tool I/O | ✅ From `ContextSearchRequest/Response` |
| **`ttlMs` + `cacheScope` on tools/list** | ✅ `ttlMs: 300000`, `cacheScope: "private"` |
| **W3C trace context in `_meta`** | ✅ Forward `traceparent` to retrieval span |
| **Self-contained handlers** | ✅ No in-memory session state |

Reference: [MCP 2026-07-28 RC blog](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/)

---

## Tool Catalog

### Tool 1: `edgequake_search`

**Purpose:** MCP/OpenAI-compatible search — returns ranked summaries + `retrieval_id` + `url`.

**Input schema:**

```json
{
  "type": "object",
  "required": ["query"],
  "properties": {
    "query": { "type": "string", "description": "Natural language search query" },
    "mode": {
      "type": "string",
      "enum": ["naive", "local", "global", "hybrid", "mix"],
      "default": "mix"
    },
    "max_results": { "type": "integer", "minimum": 1, "maximum": 50, "default": 5 },
    "workspace_id": { "type": "string", "description": "Target workspace (required for multi-tenant)" },
    "document_filter": { "$ref": "#/DocumentFilter" }
  }
}
```

**Output:**

```json
{
  "results": [
    {
      "retrieval_id": "ret_7f3a9c2e-...",
      "title": "LightRAG dual-level retrieval",
      "snippet": "LightRAG introduces dual-level retrieval...",
      "url": "edgequake://workspace/default/retrieval/ret_7f3a9c2e-...",
      "score": 0.91
    }
  ]
}
```

**Implementation:** `QueryContextService::search()` — internally runs retrieve, caches bundle, returns summary.

---

### Tool 2: `edgequake_fetch`

**Purpose:** Retrieve full `ContextBundle` for a `retrieval_id` ([OpenAI fetch pattern](https://developers.openai.com/api/docs/mcp)).

**Input schema:**

```json
{
  "type": "object",
  "required": ["retrieval_id"],
  "properties": {
    "retrieval_id": { "type": "string", "pattern": "^ret_" },
    "content_granularity": {
      "type": "string",
      "enum": ["citation", "agent", "debug"],
      "default": "agent"
    }
  }
}
```

**Output:** Full `ContextRetrievalResponse` JSON.

**Implementation:** `QueryContextService::fetch()` — cache lookup or 404/410.

---

### Tool 3: `edgequake_retrieve` (optional — full single-shot)

**Purpose:** One-shot retrieve without search/fetch split — for simple agents.

**Input:** Same as `ContextRetrievalRequest`.

**Output:** Full `ContextRetrievalResponse`.

**When to use:** Agents that don't need MCP search/fetch split. Heavier payload per call.

---

## MCP Resource (Optional — Phase 5b)

| URI pattern | Content |
|-------------|---------|
| `edgequake://workspace/{ws}/document/{doc_id}` | Document metadata |
| `edgequake://workspace/{ws}/entity/{name}` | Entity neighborhood summary |

Resources are **read-only** complements to tools — not required for MVP.

---

## Transport Architecture

```
  Phase 5a (MVP): REST-only MCP adapter
  ─────────────────────────────────────
  External MCP proxy translates tools/call → REST calls
  (community pattern — zero EdgeQuake MCP server code)

  Phase 5b (Native): Streamable HTTP MCP mount
  ──────────────────────────────────────────────
  POST /mcp
  Headers: Mcp-Method: tools/call, Mcp-Name: edgequake_search
  Body: JSON-RPC 2.0

  ┌─────────────────────────────────────────┐
  │ edgequake-api/src/handlers/mcp/mod.rs   │
  │   tools/list  → register 2-3 tools      │
  │   tools/call  → dispatch to service     │
  └─────────────────────────────────────────┘
```

**Recommendation:** Ship **5a** REST bridge for stdio clients; implement **5b** Streamable HTTP 2026-07-28 per [mcp/007-sota-implementation-roadmap.md](./mcp/007-sota-implementation-roadmap.md).

---

## Authentication for MCP

| Model | When |
|-------|------|
| OAuth 2.0 + PKCE | ChatGPT Apps / enterprise (SPEC-027 OIDC foundation) |
| API key in `_meta` | Dev / Cursor local MCP |
| mTLS | Production gateway |

MCP auth reuses SPEC-027 identity envelope — no parallel auth system.

---

## Agent Workflow Example

```
  User: "Compare EdgeQuake Mix mode with LightRAG dual retrieval"

  Agent step 1: edgequake_search({ query: "EdgeQuake Mix mode retrieval", mode: "mix" })
       └── results[0].retrieval_id = ret_aaa

  Agent step 2: edgequake_fetch({ retrieval_id: "ret_aaa" })
       └── bundle.subgraph.entities, bundle.chunks (full text)

  Agent step 3: edgequake_search({ query: "LightRAG dual level retrieval", mode: "global" })
       └── results[0].retrieval_id = ret_bbb

  Agent step 4: edgequake_fetch({ retrieval_id: "ret_bbb" })

  Agent step 5: synthesize comparison using both bundles (agent's LLM)
```

EdgeQuake provides steps 1–4; agent owns step 5.

---

## Citation URL Scheme

For OpenAI deep research citation metadata:

```
edgequake://workspace/{workspace_id}/retrieval/{retrieval_id}
```

Non-empty `url` required for citable results ([OpenAI MCP docs](https://developers.openai.com/api/docs/mcp)).

Optional HTTP redirect:

```
https://{host}/api/v1/query/context/{retrieval_id}
```

---

## tools/list Response Caching

Per SEP-2549:

```json
{
  "tools": [ "... edgequake_search ...", "... edgequake_fetch ..." ],
  "ttlMs": 3600000,
  "cacheScope": "public"
}
```

Tool schemas change only on deploy — safe to cache 1 hour.

---

## Error Mapping (MCP JSON-RPC)

| REST | MCP error code | Message |
|------|----------------|---------|
| 400 | -32602 | Invalid params |
| 401 | -32001 | Unauthorized |
| 403 | -32003 | Forbidden |
| 404 | -32004 | Not found |
| 410 | -32004 | Retrieval expired — re-run search |
| 503 | -32603 | Retrieval unavailable |

---

## Deliverables (Phase 5)

| Artifact | Path |
|----------|------|
| MCP cross-ref suite (SOTA) | `specs/028-edgequake-query-service/mcp/000-index.md` |
| OAuth / transport / client matrix | `mcp/002` … `mcp/005` |
| Edge cases EC-MCP-01..48 | `mcp/006-edge-cases-invariants.md` |
| SOTA implementation roadmap | `mcp/007-sota-implementation-roadmap.md` |
| MCP tool schemas | `specs/028-edgequake-query-service/mcp/tool-schemas.json` |
| REST→MCP mapping doc | `specs/028-edgequake-query-service/mcp/rest-adapter-guide.md` |
| Native MCP handler | `edgequake-api/src/handlers/mcp/` (Phase 5b — transport upgrade pending) |
| Example client configs | `mcp/cursor.example.json`, `codex.example.toml`, `grok.example.json`, `claude-cowork.example.md` |
