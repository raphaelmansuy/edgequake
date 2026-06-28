# 002 — OAuth 2.0 / OIDC Authorization Cross-Reference

**Cross-ref:** [005-client-compatibility-matrix.md](./005-client-compatibility-matrix.md) | SPEC-027 OIDC  
**Sources:**
- [MCP Authorization 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization)
- [Authorization tutorial](https://modelcontextprotocol.io/docs/tutorials/security/authorization)
- [Claude Connector auth](https://claude.com/docs/connectors/building/authentication)
- [Notion MCP client guide](https://developers.notion.com/guides/mcp/build-mcp-client)

---

## Normative Requirements (MCP 2025-11-25 + 2026 hardening)

| # | Requirement | RFC/SEP | EdgeQuake |
|---|-------------|---------|-----------|
| A1 | MCP server acts as **OAuth 2.1 resource server** | OAuth 2.1 | Map to SPEC-027 JWT validation |
| A2 | Publish **Protected Resource Metadata** (PRM) | RFC 9728 | `/.well-known/oauth-protected-resource` |
| A3 | PRM includes `authorization_servers[]` | RFC 9728 §3 | Point to EdgeQuake OIDC AS |
| A4 | PRM SHOULD include `scopes_supported` | MCP auth | `edgequake:read`, `edgequake:query` |
| A5 | Clients use **Resource Indicators** | RFC 8707 | `resource=` = MCP endpoint URL |
| A6 | Unauthorized → **401 + WWW-Authenticate** | RFC 6750 | Include `resource_metadata` URL |
| A7 | Support **Authorization Code + PKCE (S256)** | RFC 7636 | Via SPEC-027 OIDC handlers |
| A8 | AS SHOULD support **DCR** (RFC 7591) | SEP-991 optional CIMD | Required for Codex/Claude Code |
| A9 | Validate **`iss`** on auth responses | RFC 9207 / SEP-2468 | AS configuration |
| A10 | Token in `Authorization: Bearer` | OAuth 2.1 §5 | Reuse API middleware |
| A11 | **`token_endpoint` MUST be public HTTPS** | Cowork bug class | Never localhost in published metadata |
| A12 | Refresh token guidance | SEP-2207 | Implement refresh proxy if needed |

---

## Discovery Flow (Cross-Client SSOT)

```
  Client                          EdgeQuake MCP Gateway              Auth Server (SPEC-027)
    │                                      │                                │
    │  POST /mcp (no token)                │                                │
    │ ───────────────────────────────────► │                                │
    │ ◄─────────────────────────────────── │ 401 WWW-Authenticate: Bearer   │
    │         resource_metadata=…/well-known/oauth-protected-resource       │
    │                                      │                                │
    │  GET /.well-known/oauth-protected-resource                             │
    │ ───────────────────────────────────► │                                │
    │ ◄── { authorization_servers: [AS] }  │                                │
    │                                      │                                │
    │  GET AS /.well-known/oauth-authorization-server                      │
    │ ─────────────────────────────────────────────────────────────────────►│
    │ ◄── authorize, token, registration endpoints                          │
    │                                      │                                │
    │  [DCR POST /register] ──────────────────────────────────────────────►│
    │  [Browser PKCE /authorize] ─────────────────────────────────────────►│
    │  [POST /token code+verifier] ───────────────────────────────────────►│
    │                                      │                                │
    │  POST /mcp  Authorization: Bearer …  │                                │
    │ ───────────────────────────────────► │ validate JWT ─────────────────►│
    │ ◄── tools/list / tools/call          │                                │
```

---

## Protected Resource Metadata (EdgeQuake template)

**Path:** `GET https://{host}/.well-known/oauth-protected-resource`  
**Alt path (MCP scoped):** `GET https://{host}/.well-known/oauth-protected-resource/mcp`

```json
{
  "resource": "https://api.edgequake.example/mcp",
  "authorization_servers": [
    "https://api.edgequake.example/api/v1/auth/oidc"
  ],
  "scopes_supported": [
    "edgequake:read",
    "edgequake:query",
    "openid",
    "profile"
  ],
  "bearer_methods_supported": ["header"],
  "resource_documentation": "https://docs.edgequake.example/mcp"
}
```

### WWW-Authenticate (401 template)

```http
HTTP/1.1 401 Unauthorized
WWW-Authenticate: Bearer realm="edgequake-mcp",
  resource_metadata="https://api.edgequake.example/.well-known/oauth-protected-resource"
```

**EC-MCP-28:** Use title-case `WWW-Authenticate` — some Claude proxies ignore lowercase variants.

---

## Client Registration Matrix

| Client | Registration mode | Redirect URI |
|--------|-------------------|--------------|
| Claude.ai / Cowork | DCR or CIMD | `https://claude.ai/api/mcp/auth_callback` |
| Claude Code | CIMD + loopback | `http://127.0.0.1:{port}/callback`, `http://localhost:{port}/callback` (port-agnostic match) |
| OpenAI Codex | DCR | Configurable via `mcp_oauth_callback_url` |
| Cursor (remote) | DCR or API key | Ephemeral localhost or custom |
| Grok remote MCP | **Bearer only** | No OAuth — pass `authorization: Bearer …` in tool config |
| Notion (reference) | OAuth only | Hosted at `https://mcp.notion.com/mcp` |

---

## Auth Modes for EdgeQuake (tiered)

| Tier | Mode | Use case | Implementation |
|------|------|----------|----------------|
| **T0 Dev** | No auth / API key | Local Cursor, tests | `EDGEQUAKE_DEV_MODE`, `X-API-Key` |
| **T1 Agent** | Static bearer | Grok, CI, headless | `Authorization: Bearer` + API keys table |
| **T2 Interactive** | OAuth 2.1 + PKCE | Claude, Codex, ChatGPT | SPEC-027 OIDC + PRM |
| **T3 Enterprise** | OAuth + mTLS gateway | Bank deployments | Future SEP-990 IdP policies |

**Invariant FP-AUTH-01:** MCP OAuth reuses SPEC-027 identity — **no parallel user store**.

---

## Scope Design

| Scope | Grants |
|-------|--------|
| `edgequake:read` | tools/list, edgequake_fetch |
| `edgequake:query` | edgequake_search, edgequake_retrieve |
| `edgequake:admin` | Denied on MCP surface |

Principle of least privilege: ChatGPT Apps should request `edgequake:read edgequake:query` only.

---

## Token Validation Checklist

- [ ] Verify JWT signature against AS JWKS
- [ ] Validate `iss`, `aud` (resource URL or client_id)
- [ ] Check `exp` / `nbf`; clock skew ≤ 60s
- [ ] Map `sub` → tenant user; enforce workspace RLS
- [ ] Rate-limit per `sub` + client_id
- [ ] Audit log: tool name, retrieval_id, workspace_id (no chunk content)

---

## SPEC-027 Integration Points

| SPEC-027 artifact | MCP use |
|-------------------|---------|
| `handlers/auth/oidc.rs` | Authorization + token endpoints |
| `middleware.rs` TenantContext | Extract from JWT claims |
| API keys handler | Bearer fallback for Grok/Codex CI |
| RLS envelope | workspace_id from claim or tool param |

---

## Anti-Patterns (observed in production)

| Anti-pattern | Symptom | Fix |
|--------------|---------|-----|
| `token_endpoint` in metadata points to localhost | Cowork: OAuth succeeds, zero POST /token | Publish public HTTPS URLs only |
| Tool `inputSchema` serializes to non-object | Claude: "tools fetch failed" | Validate every schema is `{ "type": "object", … }` |
| No refresh token path | Daily manual reconnect | Token proxy or AS refresh support |
| Query params for secrets (`?apiKey=`) | Rejected by Claude spec | Headers only |
| Missing PRM on 401 | Client cannot start OAuth | Always include `resource_metadata` |

---

## Implementation Files (planned)

```
edgequake-api/src/mcp/
  auth/
    protected_resource_metadata.rs   # RFC 9728
    www_authenticate.rs            # 401 envelope
  gateway/
    streamable_http.rs             # Transport compliance
    oauth_middleware.rs            # Bearer validation
```
