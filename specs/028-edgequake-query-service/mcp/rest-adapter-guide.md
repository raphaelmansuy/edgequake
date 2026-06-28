# MCP REST Adapter Guide

**Spec:** 028-edgequake-query-service  
**Cross-ref:** [007-mcp-exposure-lens.md](../007-mcp-exposure-lens.md) | [mcp/000-index.md](./000-index.md)

---

## Overview

Two integration paths:

| Path | When | Doc |
|------|------|-----|
| **Native Streamable HTTP** | Production — Claude, Codex, Grok | [004-streamable-http-transport-sota.md](./004-streamable-http-transport-sota.md) |
| **REST adapter / stdio bridge** | Local dev, Cursor stdio | This guide |

Current minimal handler: `POST /api/v1/mcp` (JSON-RPC). SOTA target: `POST /mcp` with full 2026-07-28 headers — see [007-sota-implementation-roadmap.md](./007-sota-implementation-roadmap.md).

---

## Tool → REST Mapping

| MCP Tool | REST Endpoint | Method |
|----------|---------------|--------|
| `edgequake_search` | `/api/v1/query/context/search` | POST |
| `edgequake_fetch` | `/api/v1/query/context/{retrieval_id}` | GET |
| `edgequake_retrieve` | `/api/v1/query/context` | POST |
| `tools/list` (native) | `/api/v1/mcp` JSON-RPC | POST |

---

## OAuth (Remote Clients)

For Claude Cowork, Codex, ChatGPT — do **not** embed secrets in URLs.

1. MCP server returns 401 + [Protected Resource Metadata](./002-oauth2-authorization-crossref.md)
2. Client completes OAuth 2.1 + PKCE
3. Subsequent calls: `Authorization: Bearer {access_token}`

**Notion reference:** OAuth-only hosted pattern at `https://mcp.notion.com/mcp`.

---

## Example: Cursor MCP Config

See [cursor.example.json](./cursor.example.json) — three options (remote HTTP, stdio bridge, local direct).

---

## Example: Codex CLI

See [codex.example.toml](./codex.example.toml):

```bash
codex mcp add edgequake --url https://api.edgequake.example/mcp
codex mcp login edgequake   # OAuth
# or: bearer_token_env_var in config.toml
```

---

## Example: xAI Grok Remote MCP

See [grok.example.json](./grok.example.json) — pass `authorization: Bearer …` in remote MCP tool config.

---

## Manual REST Proxy

### search

```bash
curl -s -X POST http://localhost:8080/api/v1/query/context/search \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "EdgeQuake entity extraction pipeline",
    "mode": "local",
    "max_results": 5
  }'
```

### fetch

```bash
curl -s "http://localhost:8080/api/v1/query/context/ret_7f3a9c2e-4b1d-4e8a-9f0c-1d2e3f4a5b6c?content_granularity=agent" \
  -H "Authorization: Bearer $TOKEN"
```

### native MCP tools/list

```bash
curl -s -X POST http://localhost:8080/api/v1/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```

---

## Stateless Compliance Checklist

- [ ] Pass `workspace_id` in every tool call (no server session)
- [ ] Store `retrieval_id` client-side between search and fetch
- [ ] Re-search if fetch returns expired retrieval (410 / MCP -32004)
- [ ] Forward `traceparent` header for distributed tracing
- [ ] Do **not** rely on `Mcp-Session-Id` (removed in 2026-07-28)
- [ ] Do **not** implement MCP Sampling — host LLM synthesizes ([003](./003-sampling-deprecation-and-llm-policy.md))
- [ ] Send `Accept: application/json, text/event-stream` on Streamable HTTP

---

## Auth Tiers

| Environment | Method | Doc |
|-------------|--------|-----|
| Local dev | `EDGEQUAKE_DEV_MODE` or API key | T0 |
| Grok / CI | Bearer token | T1 |
| Claude / Codex / ChatGPT | OAuth 2.1 + PKCE | T2 |
| Enterprise | OAuth + mTLS gateway | T3 |

Header: `Authorization: Bearer <token>` or `X-API-Key: <key>`

---

## Full Schema & Edge Cases

- [tool-schemas.json](./tool-schemas.json) — JSON Schema 2020-12
- [006-edge-cases-invariants.md](./006-edge-cases-invariants.md) — EC-MCP-01..48
- [005-client-compatibility-matrix.md](./005-client-compatibility-matrix.md) — per-client quirks
