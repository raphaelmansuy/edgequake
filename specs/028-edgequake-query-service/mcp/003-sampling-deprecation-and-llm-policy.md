# 003 — Sampling Deprecation & LLM Policy

**Cross-ref:** [001-protocol-baseline-2026-07-28.md](./001-protocol-baseline-2026-07-28.md) | [007-sota-implementation-roadmap.md](./007-sota-implementation-roadmap.md)  
**Sources:**
- [SEP-2577 Deprecate Roots, Sampling, Logging](https://modelcontextprotocol.io/seps/2577-deprecate-roots-sampling-and-logging.md)
- [2026-07-28 RC blog — Sampling deprecated](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/)
- [MCP Sampling 2025-11-25 (legacy)](https://modelcontextprotocol.io/specification/2025-11-25/client/sampling)

---

## Verdict

**EdgeQuake MUST NOT implement MCP Sampling.**

| Feature | MCP 2026-07-28 status | EdgeQuake policy |
|---------|----------------------|------------------|
| `sampling/createMessage` | **Deprecated** (≥12mo window) | ❌ Do not implement |
| `roots/list` | **Deprecated** | ❌ Use `workspace_id` tool param |
| MCP `logging/message` | **Deprecated** | ❌ Use OpenTelemetry |

Official replacement for Sampling: **direct integration with LLM provider APIs** by the MCP **host** (Cursor, Claude, Grok), not the MCP server.

---

## Why Sampling Existed (and why it leaves)

Historical MCP allowed servers to ask the **client** to run an LLM completion mid-tool-call (e.g., rerank snippets server-side via client's model). Problems:

1. **Security:** Server-initiated LLM calls bypass user consent boundaries
2. **State:** Required persistent bidirectional transport (SSE sessions)
3. **Cost attribution:** Unclear who pays for tokens
4. **2026 stateless model:** Incompatible with sessionless Streamable HTTP

SEP-2322 (MRTR) replaces server→client **requests** with embedded `InputRequiredResult` for **elicitation** only — not open-ended LLM sampling.

---

## EdgeQuake Architecture (correct split)

```
  ┌──────────────── M MCP Host (has LLM) ─────────────────┐
  │  Claude / Cursor / Grok / Codex                          │
  │    • Owns synthesis, planning, follow-up questions     │
  │    • Calls edgequake_search → edgequake_fetch          │
  └───────────────────────────┬────────────────────────────┘
                              │ tools/call (retrieval only)
                              v
  ┌──────────────── EdgeQuake MCP Server ──────────────────┐
  │  QueryContextService                                    │
  │    • Graph + vector retrieval                           │
  │    • ContextBundle DTO                                  │
  │    • NO LLM generation on MCP surface                   │
  └─────────────────────────────────────────────────────────┘
```

Retrieval + optional **keyword LLM** inside QueryContextService (for query understanding) runs on **EdgeQuake's configured provider**, not via MCP Sampling — transparent to MCP clients.

---

## Capability Negotiation

### Server initialize / discover response

```json
{
  "capabilities": {
    "tools": {}
  }
}
```

**MUST NOT include:**

```json
{
  "capabilities": {
    "sampling": {}
  }
}
```

### Client sends sampling capability

If client advertises `sampling` in `_meta.clientCapabilities`, **ignore** — do not call client sampling methods.

---

## Legacy Client Compatibility (2025-11-25)

Some clients may still attempt `sampling/createMessage` toward servers that historically advertised sampling.

| Request | EdgeQuake response |
|---------|-------------------|
| `sampling/createMessage` | `-32601 Method not found` |
| `roots/list` | `-32601 Method not found` |
| `logging/setLevel` | `-32601 Method not found` |

Do not silently no-op — explicit not-found helps clients fall back.

---

## SEP-1577 "Sampling With Tools" (do not implement)

Experimental SEP for sampling with tool use remains **non-core**. EdgeQuake targets 2026-07-28 core only; extensions require explicit opt-in via SEP-2133 framework.

---

## Relationship to EdgeQuake `/query` LLM path

| Surface | LLM generation | MCP compliant? |
|---------|----------------|----------------|
| `POST /api/v1/query` | ✅ Full RAG answer | N/A (REST) |
| `POST /api/v1/mcp` tools | ❌ Retrieval only | ✅ |
| `edgequake_retrieve` tool | ❌ Context only | ✅ |

Agents needing answers MUST use host LLM + fetched ContextBundle — the Agentic Search pattern from SPEC-028.

---

## MRTR vs Sampling

| Mechanism | Purpose | EdgeQuake |
|-----------|---------|-----------|
| **Sampling** (deprecated) | Server asks client to run LLM | ❌ |
| **MRTR InputRequiredResult** | Server asks client for **user input** (confirm, select workspace) | ✅ Phase 2 optional |
| **Elicitation** | Structured user prompts | ✅ For destructive ops only (future) |

Example MRTR (allowed — not sampling):

```json
{
  "resultType": "inputRequired",
  "inputRequests": {
    "workspace": {
      "type": "elicitation",
      "message": "Select workspace for retrieval",
      "schema": { "type": "string", "enum": ["default", "team-a"] }
    }
  },
  "requestState": "eyJ…"
}
```

EdgeQuake retrieval tools are **read-only** — MRTR rarely needed; prefer required `workspace_id` parameter.

---

## Testing Requirements

- [ ] `capabilities` in tools/list path never mentions sampling
- [ ] Contract test: `sampling/createMessage` → -32601
- [ ] Contract test: no SSE stream opened for sampling requests
- [ ] Documentation states: "LLM synthesis is host responsibility"
