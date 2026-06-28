# 005 — Client Compatibility Matrix

**Cross-ref:** [002-oauth2-authorization-crossref.md](./002-oauth2-authorization-crossref.md) | [006-edge-cases-invariants.md](./006-edge-cases-invariants.md)

---

## Summary Matrix

| Client | Transport | Protocol | Auth | Search/Fetch | EdgeQuake priority |
|--------|-----------|----------|------|--------------|-------------------|
| **Cursor** | stdio / HTTP | 2025-11-25 → 2026 | API key, OAuth | Via tools | P0 |
| **Claude Desktop** | Streamable HTTP | 2025-11-25+ | OAuth PKCE | Connectors | P0 |
| **Claude.ai web** | HTTP via proxy | 2025-11-25 | OAuth DCR | Connectors | P1 |
| **Claude Code** | Streamable HTTP | 2026 target | CIMD + loopback | MCP CLI | P0 |
| **Claude Cowork** | Streamable HTTP | 2026 target | OAuth (public AS) | Custom connector | P0 |
| **OpenAI Codex CLI** | Streamable HTTP | 2025-11-25+ | OAuth + bearer | `codex mcp` | P0 |
| **OpenAI ChatGPT Apps** | Streamable HTTP | 2025-11-25+ | OAuth | search/fetch tools | P1 |
| **Notion** (reference) | Streamable HTTP | 2025-11-25 | OAuth only | Hosted example | Reference |
| **xAI Grok** | Streamable HTTP/SSE | Stateless OK | Bearer in config | Remote MCP API | P1 |
| **Gemini CLI** | Streamable HTTP | 2025-03-26+ | Bearer | httpUrl | P2 |

---

## Cursor

**Docs:** [Connect local MCP](https://modelcontextprotocol.io/docs/develop/connect-local-servers.md)

| Aspect | Requirement |
|--------|-------------|
| Config | `.cursor/mcp.json` — stdio command or `url` for remote |
| Auth | Env `EDGEQUAKE_API_KEY` or OAuth via mcp-remote bridge |
| Transport | Often stdio wrapper → REST; remote HTTP emerging |
| Tool names | Must match `[a-zA-Z0-9_-]+` |
| Protocol | May lag 2026-07-28 — support 2025-11-25 shim |

**Example:** [cursor.example.json](./cursor.example.json)

**Edge cases:**
- EC-MCP-10: stdio bridge must forward workspace_id env
- EC-MCP-11: long ContextBundle may exceed Cursor tool result limits → use `content_granularity: citation` first

---

## Claude (Desktop,.ai, Code, Cowork)

**Docs:** [Claude remote MCP](https://modelcontextprotocol.io/docs/develop/connect-remote-servers.md), [Connector auth](https://claude.com/docs/connectors/building/authentication)

| Surface | Transport | OAuth redirect |
|---------|-----------|----------------|
| Desktop / .ai / Cowork | Streamable HTTP | `https://claude.ai/api/mcp/auth_callback` |
| Claude Code | Streamable HTTP | Loopback ephemeral port |
| Cowork custom connector | Streamable HTTP | Same + public AS metadata |

**Registration types supported:**
- `oauth_dcr` — Dynamic Client Registration ✅ target
- `oauth_cimd` — Client ID Metadata Document ✅ target
- `none` — dev only

**Critical Cowork requirements (from production incidents):**
1. Published `token_endpoint` / `issuer` MUST be **public HTTPS** (not localhost)
2. Every tool `inputSchema` MUST be valid JSON Schema object
3. `WWW-Authenticate` title case on 401
4. Refresh tokens: implement or document reconnect interval

**Example:** [claude-cowork.example.md](./claude-cowork.example.md)

---

## OpenAI Codex CLI

**Docs:** [developers.openai.com/codex/mcp](https://developers.openai.com/codex/mcp)

| Aspect | Requirement |
|--------|-------------|
| Add server | `codex mcp add edgequake --url https://…/mcp` |
| OAuth | `codex mcp login edgequake` |
| Bearer | `bearer_token_env_var = "EDGEQUAKE_API_KEY"` |
| Callback | `mcp_oauth_callback_port`, `mcp_oauth_callback_url` for devboxes |
| Concurrent tools | Read-only tools may run concurrently (v0.136+) |

**Example:** [codex.example.toml](./codex.example.toml)

**Edge cases:**
- EC-MCP-20: Devbox needs fixed callback URL registered at AS
- EC-MCP-21: DCR required — static client IDs not native

---

## OpenAI ChatGPT / Deep Research

**Pattern:** MCP search/fetch tools with citable URLs ([OpenAI MCP docs](https://developers.openai.com/api/docs/mcp))

| Requirement | EdgeQuake |
|-------------|-----------|
| Non-empty `url` on search results | ✅ `edgequake://workspace/…/retrieval/…` |
| `retrieval_id` stable within TTL | ✅ 15min cache |
| OAuth for remote servers | See 002 |
| `allowed_tools` filtering | Client-side — tool names must be stable |

---

## Notion MCP (reference implementation)

**URL:** `https://mcp.notion.com/mcp`  
**Docs:** [Notion MCP client guide](https://developers.notion.com/guides/mcp/build-mcp-client)

| Lesson for EdgeQuake | Notion approach |
|----------------------|-----------------|
| OAuth discovery | RFC 9728 → RFC 8414 two-step |
| Transport fallback | Streamable HTTP first, SSE `/sse` second |
| User consent | OAuth only for hosted — no bearer on hosted |
| Tool design | Optimized for token efficiency |

EdgeQuake **should support bearer** for headless agents (Notion hosted does not).

---

## xAI Grok / Grok Build

**Docs:** [Remote MCP Tools](https://docs.x.ai/developers/tools/remote-mcp)

| Parameter | Usage |
|-----------|-------|
| `server_url` | `https://api.edgequake.example/mcp` |
| `server_label` | `edgequake` |
| `authorization` | `Bearer {token}` |
| `allowed_tools` | `["edgequake_search","edgequake_fetch"]` |
| `extra_headers` | Workspace routing |

**Transports:** Streamable HTTP + SSE only (no stdio on Grok side).

**Example:** [grok.example.json](./grok.example.json)

**Edge cases:**
- EC-MCP-30: Grok injects all tools if `allowed_tools` empty — document recommended filter
- EC-MCP-31: Grok may skip initialize — server must be stateless

---

## Cross-Client Tool Naming

Stable names (do not rename without deprecation window):

| Tool | Purpose |
|------|---------|
| `edgequake_search` | Ranked summaries + retrieval_id |
| `edgequake_fetch` | Full ContextBundle |
| `edgequake_retrieve` | One-shot full retrieval |

Prefix pattern matches Grok's `server_label` tool prefixing expectations.

---

## Conformance Test Matrix (code-is-law 2026-06-28)

| Test | Cursor | Claude | Codex | Grok | Evidence |
|------|--------|--------|-------|------|----------|
| tools/list | ✅ | ✅ | ✅ | ✅ | `spec028_mcp_e2e` |
| tools/call search | ✅ | ✅ | ✅ | ✅ | `spec028_mcp_e2e` |
| search→fetch roundtrip | ✅ | ✅ | ✅ | ✅ | `spec028_mcp_e2e` |
| OAuth 401→flow | optional | ✅ | ✅ | N/A | `spec028_mcp_oauth_e2e` + wiremock OIDC |
| Bearer auth | ✅ | optional | ✅ | ✅ | JWT + API key e2e |
| 2026-07-28 headers | best-effort | ✅ | ✅ | ✅ | `spec028_mcp_transport` |
| SSE retrieve stream | emerging | ✅ | ✅ | ✅ | EC-MCP-09, `Mcp-Stream: true` |
| Registry manifest | — | — | — | — | `server.json` + well-known |
| Large payload truncation | ✅ | ✅ | ✅ | ✅ | granularity + max_results cap |

---

## Recommended Client Documentation Links

Ship in EdgeQuake docs portal:

1. "Connect from Cursor" → cursor.example.json
2. "Connect from Claude Cowork" → claude-cowork.example.md
3. "Connect from Codex" → codex.example.toml
4. "Connect from Grok" → grok.example.json + xAI API snippet
