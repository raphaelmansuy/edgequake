# 004 — Streamable HTTP Transport (SOTA)

**Cross-ref:** [001-protocol-baseline-2026-07-28.md](./001-protocol-baseline-2026-07-28.md) | [002-oauth2-authorization-crossref.md](./002-oauth2-authorization-crossref.md)  
**Source:** [Streamable HTTP spec (draft / 2026-07-28)](https://modelcontextprotocol.io/specification/draft/basic/transports/streamable-http)

---

## Endpoint Layout

| Path | Methods | Purpose |
|------|---------|---------|
| `POST /mcp` | POST | **Primary** JSON-RPC (2026-07-28) |
| `GET /mcp` | — | **Removed** in 2026-07-28 (was SSE listener) |
| `POST /mcp/sse` | POST | Optional legacy 2025-11-25 compat |
| `GET /.well-known/oauth-protected-resource` | GET | OAuth PRM |

**Current EdgeQuake:** `POST /api/v1/mcp` — must migrate or alias to `/mcp` at gateway root for client ergonomics.

---

## Required Client → Server Headers (2026-07-28)

| Header | Required | Value example |
|--------|----------|---------------|
| `Content-Type` | ✅ | `application/json` |
| `Accept` | ✅ | `application/json, text/event-stream` |
| `MCP-Protocol-Version` | ✅ | `2026-07-28` |
| `Mcp-Method` | ✅ | `tools/call` |
| `Mcp-Name` | ✅ for tool/resource/prompt calls | `edgequake_search` |
| `Authorization` | Production | `Bearer {access_token}` |
| `Origin` | Browser clients | Validated by server |

### EC-MCP-03: Accept header enforcement

Clients **MUST** send both content types. Server **SHOULD** return `406` with clear message if missing (Gemini CLI bug class).

```
Accept: application/json, text/event-stream
```

---

## Required Server → Client Behavior

| Request type | Response |
|--------------|----------|
| JSON-RPC notification | `202 Accepted` empty body |
| JSON-RPC request (simple) | `200` + `Content-Type: application/json` |
| JSON-RPC request (streaming) | `200` + `Content-Type: text/event-stream` |
| Invalid Accept | `406 Not Acceptable` |
| Header/body method mismatch | `400 HeaderMismatch` |
| Unknown protocol version | `400 UnsupportedProtocolVersionError` |
| Unknown RPC method | `404` + JSON-RPC `-32601` |

SSE streams SHOULD include `X-Accel-Buffering: no` and periodic `:` comment keep-alives.

---

## Request Example (canonical)

```http
POST /mcp HTTP/1.1
Host: api.edgequake.example
Content-Type: application/json
Accept: application/json, text/event-stream
MCP-Protocol-Version: 2026-07-28
Mcp-Method: tools/call
Mcp-Name: edgequake_search
Authorization: Bearer eyJ…
traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01

{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "edgequake_search",
    "arguments": {
      "query": "LightRAG dual-level retrieval",
      "mode": "mix",
      "workspace_id": "default"
    },
    "_meta": {
      "io.modelcontextprotocol/protocolVersion": "2026-07-28",
      "io.modelcontextprotocol/clientInfo": { "name": "codex", "version": "0.136" },
      "io.modelcontextprotocol/clientCapabilities": {}
    }
  }
}
```

---

## `x-mcp-header` Tool Parameter Mirroring

For multi-tenant routing without parsing body at gateway:

```json
{
  "workspace_id": {
    "type": "string",
    "x-mcp-header": "Workspace-Id"
  }
}
```

Emits: `Mcp-Param-Workspace-Id: default`

EdgeQuake SHOULD annotate `workspace_id` in tool-schemas.json for load balancer affinity.

---

## tools/list Caching (SEP-2549)

```json
{
  "tools": [ "…" ],
  "ttlMs": 3600000,
  "cacheScope": "public"
}
```

| Field | EdgeQuake value | Rationale |
|-------|-----------------|-----------|
| `ttlMs` | 3600000 (1h) | Schemas change on deploy only |
| `cacheScope` | `public` | Same tools for all users |

Per-user tool differences (future) → `cacheScope: "private"`.

---

## Security Requirements

| Control | Requirement |
|---------|-------------|
| Origin validation | 403 if Origin present and not allowlisted |
| Local bind | Dev: `127.0.0.1` only |
| TLS | Production: HTTPS mandatory |
| DNS rebinding | Origin check + no 0.0.0.0 bind in dev |
| Rate limiting | Per IP + per token |
| Body size | Max 1MB JSON-RPC body |

---

## Backward Compatibility Strategy

```
  Client probe flow:
  ┌─────────────────────────────────────────────────────────┐
  │ 1. Modern POST /mcp (2026-07-28 headers)                │
  │ 2. If 400 HeaderMismatch → fix headers                  │
  │ 3. If legacy server suspected → POST /mcp/sse + init    │
  │ 4. Grok/xAI may skip initialize (stateless) — OK        │
  └─────────────────────────────────────────────────────────┘
```

EdgeQuake gateway SHOULD support **both**:
- **2026-07-28** stateless (primary)
- **2025-11-25** shim: accept `initialize`, emit synthetic capabilities, ignore session ID

---

## Gap vs Current Handler

| Requirement | Current `handlers/mcp/mod.rs` | Target |
|-------------|------------------------------|--------|
| Accept validation | ❌ | ✅ |
| MCP-Protocol-Version | ❌ | ✅ |
| Mcp-Method / Mcp-Name | ❌ | ✅ |
| `_meta` parsing | ❌ | ✅ |
| SSE response option | ❌ | ✅ for long retrieve |
| 401 + WWW-Authenticate | ❌ | ✅ |
| Separate /mcp path | ❌ uses /api/v1/mcp | ✅ alias both |

---

## Trace Context (SEP-414)

Propagate from `_meta` or HTTP headers:

| Key | Forward to |
|-----|------------|
| `traceparent` | QueryContextService span |
| `tracestate` | OTel baggage |
| `baggage` | tenant hints (never override auth claims) |
