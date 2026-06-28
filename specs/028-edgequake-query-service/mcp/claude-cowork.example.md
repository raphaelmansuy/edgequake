# Claude Cowork / Connector — OAuth Setup Checklist

**Cross-ref:** [002-oauth2-authorization-crossref.md](./002-oauth2-authorization-crossref.md) | [005-client-compatibility-matrix.md](./005-client-compatibility-matrix.md)

---

## Prerequisites

- EdgeQuake MCP gateway at `https://{host}/mcp` (Streamable HTTP)
- SPEC-027 OIDC authorization server on same host or trusted domain
- **Public HTTPS** for all metadata endpoints (never localhost in published metadata)

---

## Step 1 — Protected Resource Metadata

Verify:

```bash
curl -s "https://api.edgequake.example/.well-known/oauth-protected-resource" | jq .
```

Expected: `resource`, `authorization_servers`, `scopes_supported`.

---

## Step 2 — Authorization Server Metadata

```bash
curl -s "https://api.edgequake.example/.well-known/oauth-authorization-server" | jq .
```

**Critical:** `token_endpoint` and `issuer` MUST resolve to public HTTPS URLs reachable from Anthropic's Cowork sandbox.

---

## Step 3 — Register Redirect URIs

Allowlist at your authorization server:

| URI | Client |
|-----|--------|
| `https://claude.ai/api/mcp/auth_callback` | Claude.ai, Desktop, Cowork |
| `http://127.0.0.1:*/*` (port-agnostic) | Claude Code |
| `http://localhost:*/*` (port-agnostic) | Claude Code |

---

## Step 4 — Enable Dynamic Client Registration

Claude supports:
- `oauth_dcr` (RFC 7591) — **recommended**
- `oauth_cimd` (Client ID Metadata Document)

Ensure `registration_endpoint` is live and returns `client_id`.

---

## Step 5 — Add Custom Connector (Cowork)

1. Cowork → Settings → Connectors → Add custom connector
2. URL: `https://api.edgequake.example/mcp`
3. Complete Microsoft/OIDC sign-in when prompted
4. Verify tools: `edgequake_search`, `edgequake_fetch`, `edgequake_retrieve`

---

## Step 6 — Validate Tool Schemas

Every tool `inputSchema` MUST be:

```json
{ "type": "object", "properties": { ... } }
```

**Not** bare `true`, array root, or invalid `$ref` to external URLs.

---

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| OAuth UI OK, no tools | Invalid inputSchema | Validate tool-schemas.json |
| OAuth OK, tools fail on call | Workspace / scope | Pass workspace_id; check JWT scopes |
| Never POST /token in logs | token_endpoint is localhost | Fix AS metadata |
| Connected but 401 on tools | Token not bound to connector | Re-add connector; check AS issuer |
| Daily disconnect | No refresh token | Implement refresh (SEP-2207) |

---

## Test Command (after token obtained)

```bash
curl -X POST "https://api.edgequake.example/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "MCP-Protocol-Version: 2026-07-28" \
  -H "Mcp-Method: tools/call" \
  -H "Mcp-Name: edgequake_search" \
  -H "Authorization: Bearer ${ACCESS_TOKEN}" \
  -d '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"tools/call",
    "params":{
      "name":"edgequake_search",
      "arguments":{"query":"test","workspace_id":"default"},
      "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28"}
    }
  }'
```
