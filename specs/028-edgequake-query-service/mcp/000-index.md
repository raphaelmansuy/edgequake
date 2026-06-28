# MCP Cross-Reference Specification Suite

**Spec:** 028-edgequake-query-service / MCP  
**Date:** 2026-06-28  
**Authority:** SOTA target for EdgeQuake remote MCP server  
**Protocol target:** MCP `2026-07-28` (Release Candidate, final 2026-07-28)  
**Cross-ref:** [007-mcp-exposure-lens.md](../007-mcp-exposure-lens.md) | [013-implementation-assessment.md](../013-implementation-assessment.md)

---

## Purpose

This directory is the **cross-reference specification** for upgrading EdgeQuake from the current minimal JSON-RPC handler (`POST /api/v1/mcp`) to a **state-of-the-art, multi-client compatible** remote MCP server.

Target clients (must interoperate):

| Client | Transport | Auth model |
|--------|-----------|------------|
| **Cursor** | stdio bridge or Streamable HTTP | API key / OAuth |
| **Claude Desktop / Claude.ai** | Streamable HTTP (+ legacy SSE fallback) | OAuth 2.1 + PKCE (DCR/CIMD) |
| **Claude Code** | Streamable HTTP | OAuth loopback + CIMD |
| **Claude Cowork** | Streamable HTTP via custom connector | OAuth (public AS endpoints) |
| **OpenAI Codex CLI** | Streamable HTTP | OAuth DCR + bearer token |
| **OpenAI ChatGPT / Apps** | Streamable HTTP | OAuth + search/fetch pattern |
| **Notion MCP** (reference) | Streamable HTTP | OAuth-only (hosted reference) |
| **xAI Grok / Grok Build** | Streamable HTTP / SSE | Bearer token in `authorization` |

---

## Document Map

| # | Document | Read when |
|---|----------|-----------|
| [001-protocol-baseline-2026-07-28.md](./001-protocol-baseline-2026-07-28.md) | Protocol version, stateless core, breaking changes vs 2025-11-25 |
| [002-oauth2-authorization-crossref.md](./002-oauth2-authorization-crossref.md) | OAuth 2.1, PKCE, RFC 9728/8414/8707, EdgeQuake ↔ SPEC-027 mapping |
| [003-sampling-deprecation-and-llm-policy.md](./003-sampling-deprecation-and-llm-policy.md) | Sampling deprecated — what EdgeQuake must NOT do |
| [004-streamable-http-transport-sota.md](./004-streamable-http-transport-sota.md) | Headers, Accept, MRTR, caching, backward compat |
| [005-client-compatibility-matrix.md](./005-client-compatibility-matrix.md) | Per-client quirks (Cursor, Claude*, Codex, Grok, Notion) |
| [006-edge-cases-invariants.md](./006-edge-cases-invariants.md) | Edge-case catalog EC-MCP-01..40 |
| [007-sota-implementation-roadmap.md](./007-sota-implementation-roadmap.md) | Gap analysis vs current code + phased build plan |
| [server.json](./server.json) | Official MCP Registry publish manifest (remote Streamable HTTP) |

### Artifacts (schemas & examples)

| File | Purpose |
|------|---------|
| [tool-schemas.json](./tool-schemas.json) | JSON Schema 2020-12 tool SSOT |
| [server.json](./server.json) | MCP Registry metadata (`io.github.raphaelmansuy/edgequake`) |
| [rest-adapter-guide.md](./rest-adapter-guide.md) | REST proxy mapping (Phase 5a bridge) |
| [cursor.example.json](./cursor.example.json) | Cursor local MCP config |
| [codex.example.toml](./codex.example.toml) | OpenAI Codex CLI config |
| [claude-cowork.example.md](./claude-cowork.example.md) | Claude Cowork / Connector OAuth checklist |
| [grok.example.json](./grok.example.json) | xAI Grok remote MCP tool config |

---

## Official Sources (authoritative)

| Topic | URL |
|-------|-----|
| MCP 2026-07-28 RC blog | https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/ |
| Streamable HTTP (draft) | https://modelcontextprotocol.io/specification/draft/basic/transports/streamable-http |
| Authorization 2025-11-25 | https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization |
| Authorization tutorial | https://modelcontextprotocol.io/docs/tutorials/security/authorization |
| SEP-2577 Sampling deprecation | https://modelcontextprotocol.io/seps/2577-deprecate-roots-sampling-and-logging.md |
| SEP-2567 Sessionless handles | https://modelcontextprotocol.io/seps/2567-sessionless-mcp.md |
| OpenAI Codex MCP | https://developers.openai.com/codex/mcp |
| xAI Remote MCP Tools | https://docs.x.ai/developers/tools/remote-mcp |
| Notion MCP integration | https://developers.notion.com/guides/mcp/build-mcp-client |
| Claude Connector auth | https://claude.com/docs/connectors/building/authentication |

---

## Architecture Target

```
  ┌─────────────┐  Streamable HTTP   ┌──────────────────────────────────┐
  │ MCP Hosts   │  POST /mcp         │ EdgeQuake MCP Gateway            │
  │ Cursor      │  MCP-Protocol-     │  • RFC 9728 PRM + OAuth 2.1      │
  │ Claude*     │  Version: 2026-    │  • Mcp-Method / Mcp-Name headers │
  │ Codex/Grok  │  07-28             │  • tools/list + tools/call       │
  │ OpenAI Apps │  Authorization:    │  • NO sampling / NO Mcp-Session  │
  └─────────────┘  Bearer …          └──────────────┬───────────────────┘
                                                    │
                                                    v
                                         QueryContextService (SSOT)
                                         retrieval_id explicit handles
```

---

## Current Implementation Status

| Capability | Status | Doc |
|------------|--------|-----|
| Tool schemas (search/fetch/retrieve) | ✅ | tool-schemas.json |
| Streamable HTTP gateway (`POST /mcp`) | ✅ | 004, 007 |
| OAuth PRM + WWW-Authenticate | ✅ | 002, oauth e2e |
| Accept + protocol headers | ✅ | validate.rs |
| `_meta` + trace propagation | ✅ | meta.rs |
| MCP Registry manifest | ✅ | server.json, registry.rs |
| Registry publish workflow | ✅ | 007, mcp-registry-publish.yml |
| Sampling capability | ❌ must not implement | 003 |
| SSE streaming responses | ✅ | 004 EC-09, `gateway/sse.rs` |
| Client example configs | ✅ | *.example.* |
