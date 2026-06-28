# 006 — MCP Edge Cases & Invariants

**Cross-ref:** [009-edge-cases-invariants.md](../009-edge-cases-invariants.md) | [005-client-compatibility-matrix.md](./005-client-compatibility-matrix.md)

---

## Invariants (FP-MCP)

| ID | Invariant |
|----|-----------|
| FP-MCP-01 | No protocol-level session (`Mcp-Session-Id` forbidden) |
| FP-MCP-02 | No MCP Sampling — host owns LLM |
| FP-MCP-03 | `retrieval_id` is the only cross-call state handle |
| FP-MCP-04 | Auth claims beat tool-supplied tenant/workspace |
| FP-MCP-05 | Tool schemas are always JSON Schema 2020-12 objects |
| FP-MCP-06 | Search results MUST include non-empty citable `url` |
| FP-MCP-07 | Bypass retrieval mode rejected on all MCP tools |
| FP-MCP-08 | 401 responses MUST include PRM pointer |

---

## Edge Case Catalog

### Transport & Protocol

| ID | Case | Expected behavior | Clients affected |
|----|------|-------------------|------------------|
| EC-MCP-01 | Missing `Accept` header | 406 + message listing required types | Gemini CLI |
| EC-MCP-02 | `Mcp-Method` ≠ body `method` | 400 HeaderMismatch | All HTTP |
| EC-MCP-03 | Missing `MCP-Protocol-Version` | 400 or default 2025-03-26 if compat mode | xAI docs server |
| EC-MCP-04 | Unsupported protocol version | 400 UnsupportedProtocolVersionError + supported list | Forward-compat |
| EC-MCP-05 | Legacy `initialize` on 2026 server | Return capabilities shim, no session ID | Older Claude |
| EC-MCP-06 | Client sends `Mcp-Session-Id` | Ignore header (do not error) | 2025 clients |
| EC-MCP-07 | JSON-RPC batch array body | 400 — single object only | Malformed |
| EC-MCP-08 | Notification POST | 202 Accepted | Progress cancel |
| EC-MCP-09 | SSE stream closed mid-request | Treat as cancellation | Streaming retrieve |
| EC-MCP-10 | Request body > 1MB | 413 Payload Too Large | Large filters |

### Authentication & OAuth

| ID | Case | Expected behavior |
|----|------|-------------------|
| EC-MCP-11 | No Authorization on prod server | 401 + WWW-Authenticate + PRM URL |
| EC-MCP-12 | Expired JWT | 401 invalid_token |
| EC-MCP-13 | Valid token, wrong audience | 403 |
| EC-MCP-14 | API key + Bearer both sent | Prefer Bearer; log warning |
| EC-MCP-15 | PRM `token_endpoint` is localhost | **Misconfiguration** — Cowork fails token exchange |
| EC-MCP-16 | lowercase `www-authenticate` | Use title case WWW-Authenticate |
| EC-MCP-17 | OAuth scope insufficient | 403 + required scope in error |
| EC-MCP-18 | DCR registration missing redirect | 400 from AS — document allowed URIs |
| EC-MCP-19 | Claude Code loopback redirect port varies | AS must match port-agnostic |
| EC-MCP-20 | Refresh token expired | 401; client must re-login |

### Tool Execution — Retrieval

| ID | Case | Expected behavior |
|----|------|-------------------|
| EC-MCP-21 | Empty `query` string | -32602 Invalid params |
| EC-MCP-22 | `mode: bypass` | -32602 "bypass not allowed" |
| EC-MCP-23 | Invalid `mode` string | -32602 |
| EC-MCP-24 | Unknown tool name | -32602 or -32601 per method |
| EC-MCP-25 | `retrieval_id` wrong prefix | -32602 |
| EC-MCP-26 | Unknown `retrieval_id` | -32004 Not found |
| EC-MCP-27 | Expired `retrieval_id` (>TTL) | -32004 "Retrieval expired — re-run search" |
| EC-MCP-28 | fetch before search | 404/410 — agent must re-search |
| EC-MCP-29 | `content_granularity: debug` without admin scope | 403 |
| EC-MCP-30 | Workspace in tool ≠ claim workspace | 403 fail-closed |
| EC-MCP-31 | Missing workspace_id, multi-tenant | -32602 or MRTR elicitation |
| EC-MCP-32 | Document filter matches zero docs | Empty results + coverage_score=0 |
| EC-MCP-33 | Document filter SQL error | -32603 internal (no leak) |
| EC-MCP-34 | Concurrent fetch same retrieval_id | Idempotent — same bundle |
| EC-MCP-35 | search storm (rate limit) | 429 + Retry-After |

### Payload & Token Limits

| ID | Case | Expected behavior |
|----|------|-------------------|
| EC-MCP-36 | ContextBundle > client limit | Recommend citation granularity first |
| EC-MCP-37 | 500+ chunk results | Truncate + truncation metadata |
| EC-MCP-38 | Unicode / emoji in query | UTF-8 validate |
| EC-MCP-39 | Prompt injection in query | Retrieve as data; no tool side-effects |
| EC-MCP-40 | `max_results` > 50 | Clamp to 50 |

### Deprecated Features

| ID | Case | Expected behavior |
|----|------|-------------------|
| EC-MCP-41 | `sampling/createMessage` | -32601 Method not found |
| EC-MCP-42 | `roots/list` | -32601 |
| EC-MCP-43 | `logging/setLevel` | -32601 |

### Multi-Client Interop

| ID | Case | Expected behavior |
|----|------|-------------------|
| EC-MCP-44 | Grok skips initialize | tools/call works immediately |
| EC-MCP-45 | Codex concurrent read-only tools | Safe — all tools read-only |
| EC-MCP-46 | ChatGPT requires HTTPS url field | edgequake:// or https redirect |
| EC-MCP-47 | Notion-style OAuth-only client | OAuth path must work (no bearer fallback assumed) |
| EC-MCP-48 | Cursor stdio bridge timeout | Return within 120s or stream progress SSE |

---

## Error Mapping (complete)

| Condition | HTTP | JSON-RPC | MCP message |
|-----------|------|----------|-------------|
| Invalid params | 200* | -32602 | Human-readable |
| Unauthorized | 401 | — | WWW-Authenticate |
| Forbidden | 403 | -32003 | Scope/workspace |
| Not found | 200* | -32004 | retrieval_id |
| Expired retrieval | 200* | -32004 | Re-run search |
| Rate limit | 429 | -32603 | Retry-After |
| Internal | 200* | -32603 | Generic |

\*Streamable HTTP may return JSON-RPC error in 200 body per JSON-RPC convention; align with client expectations in tests.

---

## Test Coverage Map

| EC IDs | Test file |
|--------|-----------|
| EC-MCP-21..27,34,38,40,44,45,46 | `spec028_mcp_e2e.rs` ✅ |
| EC-MCP-01..08,10,03,09,35,41..43 + registry | `spec028_mcp_transport.rs` ✅ |
| EC-MCP-11,12,14,16,29,30,39 + PRM + JWT + OIDC | `spec028_mcp_oauth_e2e.rs` ✅ |
| MCP Registry server.json SSOT | `spec028_mcp_registry.rs` ✅ |
| EC-MCP-13,15,17..20 | deferred (Keycloak prod smoke; wiremock in CI) |
| EC-MCP-28,31..33,36..37,47,48 | future / operational |

---

## Agent Loop Edge Cases (Agentic Search)

```
  Normal loop:
    search → fetch → synthesize (host LLM)

  Expired mid-loop:
    search → fetch OK → … delay … → fetch 410
    → MUST search again (same or refined query)

  Empty corpus:
    search → results=[] → coverage_score≈0
    → agent should broaden query or ingest docs

  Wrong workspace:
    search with workspace A → empty
    → agent should elicit workspace or list workspaces (future tool)
```
