# 001 — Protocol Baseline: MCP 2026-07-28

**Cross-ref:** [000-index.md](./000-index.md) | [004-streamable-http-transport-sota.md](./004-streamable-http-transport-sota.md)  
**Source:** [2026-07-28 RC blog](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/)

---

## Version Strategy

EdgeQuake MCP gateway MUST target **`2026-07-28`** as primary protocol version while supporting negotiated fallback to **`2025-11-25`** for clients not yet upgraded.

| Version | EdgeQuake support | Notes |
|---------|-------------------|-------|
| `2026-07-28` | **Primary** | Stateless, no initialize handshake |
| `2025-11-25` | Compatibility shim | Accept legacy initialize; map to stateless internally |
| `2025-06-18` | Best-effort | Missing `MCP-Protocol-Version` → treat as `2025-03-26` per spec |
| `2024-11-05` HTTP+SSE | Legacy fallback only | Separate `/mcp/sse` if needed; not MVP |

---

## Headline Changes (2025-11-25 → 2026-07-28)

```
  BEFORE (2025-11-25)                    AFTER (2026-07-28)
  ─────────────────────                  ───────────────────
  initialize → Mcp-Session-Id            NO handshake
  sticky sessions                        round-robin safe
  GET /mcp SSE listener                  POST-only core; subscriptions/listen for changes
  server sends sampling/roots on SSE     MRTR InputRequiredResult embedded in response
  -32002 resource not found              -32602 Invalid Params (SEP-2164)
```

### SEP cross-reference

| SEP | Change | EdgeQuake action |
|-----|--------|----------------|
| SEP-2575 | Remove initialize/initialized | Implement `server/discover` optional |
| SEP-2567 | Remove Mcp-Session-Id | Use `retrieval_id` tool handles only |
| SEP-2243 | Mcp-Method + Mcp-Name headers | Validate on every POST |
| SEP-2549 | ttlMs + cacheScope on list results | Return on tools/list |
| SEP-2106 | JSON Schema 2020-12 tools | Already in tool-schemas.json |
| SEP-2322 | Multi Round-Trip Requests (MRTR) | Support InputRequiredResult for elicitation only |
| SEP-2577 | Deprecate Roots, Sampling, Logging | Do NOT advertise these capabilities |
| SEP-414 | W3C trace context in `_meta` | Forward traceparent to QueryContextService |

---

## Stateless Core (FP-MCP-01)

**Invariant:** EdgeQuake MCP server holds **no protocol-level session state**.

State that spans tool calls MUST use **explicit handles** visible to the model:

```
  edgequake_search({ query: "RAG pipeline" })
       └── results[0].retrieval_id = "ret_abc…"

  edgequake_fetch({ retrieval_id: "ret_abc…" })
       └── full ContextBundle
```

This pattern satisfies SEP-2567 and works with Grok, OpenAI deep-research, and Claude tool loops.

TTL cache for `retrieval_id` is **application state**, not MCP session state — keyed by opaque ID, not `Mcp-Session-Id`.

---

## Capability Advertisement (2026-07-28)

### Server capabilities (target)

```json
{
  "capabilities": {
    "tools": { "listChanged": false },
    "resources": { "subscribe": false, "listChanged": false },
    "prompts": { "listChanged": false },
    "extensions": {}
  }
}
```

### MUST NOT advertise (deprecated SEP-2577)

| Capability | Status | EdgeQuake |
|------------|--------|-----------|
| `sampling` | **Deprecated** | ❌ Never implement |
| `roots` | **Deprecated** | ❌ Use `workspace_id` tool param |
| `logging` (MCP channel) | **Deprecated** | ❌ Use OpenTelemetry/tracing |

Deprecation window: features remain in spec ≥12 months from 2026-07-28; EdgeQuake skips them entirely.

---

## `_meta` Request Envelope (required for 2026-07-28)

Every client POST MUST include (in body `_meta` or params `_meta`):

```json
{
  "_meta": {
    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
    "io.modelcontextprotocol/clientInfo": {
      "name": "cursor",
      "version": "1.0.0"
    },
    "io.modelcontextprotocol/clientCapabilities": {},
    "traceparent": "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
  }
}
```

Server MUST reject header/body protocol version mismatch with `400 HeaderMismatch`.

---

## Tool Schema Requirements (SEP-2106)

| Rule | EdgeQuake |
|------|-----------|
| `inputSchema` root MUST be `type: object` | ✅ |
| `outputSchema` unrestricted type | ✅ ContextRetrievalResponse object |
| No auto-dereference external `$ref` | ✅ inline $defs only |
| Bound validation time | Implement 50ms schema validation timeout |
| Tool names: `[a-zA-Z0-9_-]{1,128}` (SEP-986) | ✅ edgequake_* |

---

## Error Code Migration

| Condition | 2025-11-25 | 2026-07-28 | EdgeQuake |
|-----------|------------|------------|-----------|
| Unknown resource URI | -32002 | **-32602** | Map 404 → -32602 |
| Invalid params | -32602 | -32602 | ✅ |
| Unauthorized | HTTP 401 + WWW-Authenticate | same | See 002 |

---

## Backward Compatibility Detection

Per Streamable HTTP spec, clients MAY probe modern vs legacy:

1. Send modern POST with `MCP-Protocol-Version: 2026-07-28` + required headers
2. On `400` with modern JSON-RPC error → inspect error kind (do not assume legacy)
3. On connection failure to `/mcp` → optional fallback `/mcp/sse` (legacy SSE)

EdgeQuake SHOULD log client protocol version from `_meta` for adoption metrics.
