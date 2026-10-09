---
title: API keys and MCP
description: How programmatic access works next to SSO: API keys, MCP OAuth, and what logout and membership changes revoke.
---

SSO covers people at a browser. Programs use API keys or the MCP OAuth flow (SPEC-154). An SSO user creates an API key with the normal endpoint and the session token, so the same rules apply: tenant binding, scopes and audience.

```bash
curl -X POST http://localhost:8080/api/v1/api-keys \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"ci","scopes":["edgequake:read","edgequake:query"],"expires_in_days":90}'
```

The response shows the key once. New keys get the scopes `edgequake:read` and `edgequake:query` unless you ask for `edgequake:write`. See the [security guide](../best-practices.md#credential-types) for all credential types.

What revokes what:

| Event | Effect |
|-------|--------|
| Back-channel logout from the IdP | Revokes the SSO session (refresh family and federated session). It does not revoke API keys that were already issued. |
| Revoke a key (`DELETE /api/v1/api-keys/{key_id}`) | The key stops working. |
| Deactivate the user | The user's JWTs and keys stop working, because the account is checked on each request. |
| Remove a membership or suspend a tenant | Session refresh fails at once with `membership_revoked` or `tenant_suspended`. |

MCP OAuth discovery endpoints stay public. The MCP endpoint (`POST /mcp`) does its own authentication.
