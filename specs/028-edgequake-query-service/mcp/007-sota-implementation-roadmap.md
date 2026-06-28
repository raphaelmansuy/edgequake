# 007 — SOTA Implementation Roadmap

**Cross-ref:** [013-implementation-assessment.md](../013-implementation-assessment.md) | [000-index.md](./000-index.md) | [012-code-is-law-verdict.md](../012-code-is-law-verdict.md)  
**Date:** 2026-06-28  
**Verdict:** **Code is law** — MCP SOTA Phases **MCP-A through MCP-E complete** (prod OAuth smoke deferred).

---

## Code-is-Law Summary (2026-06-28)

| Phase | Status | Evidence |
|-------|--------|----------|
| **MCP-A** Transport | ✅ Done | `gateway/{validate,body,meta,json_rpc}.rs`, `tests/spec028_mcp_transport.rs` (21) |
| **MCP-B** OAuth RS | ✅ Done | PRM + JWT + OIDC wiremock e2e (`spec028_mcp_oauth_e2e.rs`, 15) |
| **MCP-C** Tool hardening | ✅ Done | `tool_validation.rs`, `workspace_policy.rs`, EC-21..27,29,30,34,38..40,44..46 |
| **MCP-D** Client configs | ✅ Done | Example configs + `server.json` registry artifact |
| **MCP-E** Production | ✅ Done | SSE retrieve stream, rate limits, registry publish-ready, traceparent |

**Tests (all passing):**

| Suite | Count |
|-------|-------|
| `spec028_api_contract` | 18 |
| `spec028_context_e2e` | 9 |
| `spec028_mcp_e2e` | 19 |
| `spec028_mcp_transport` | 21 |
| `spec028_mcp_oauth_e2e` | 15 |
| `spec028_mcp_registry` | 3 |
| `spec027_e2e` regression | 35 |
| **SPEC-028 MCP total** | **58** |

**DRY / SOLID:**

| Concern | Module |
|---------|--------|
| Thin HTTP adapter | `handlers/mcp/mod.rs` (Bytes → gateway; JSON + SSE outcomes) |
| Body parse SSOT | `gateway/body.rs` (batch, notification, 1MB) |
| Transport validation | `gateway/validate.rs` |
| SSE streaming (MCP-E) | `gateway/sse.rs` (`Mcp-Stream` / `_meta.stream`) |
| Workspace claim policy | `gateway/workspace_policy.rs` (FP-MCP-04, EC-30) |
| Tool arg validation | `gateway/tool_validation.rs` |
| Tool execution SSOT | `gateway/dispatch.rs::execute_tool_call` |
| Tool catalog | `gateway/tools.rs` |
| Registry manifest SSOT | `mcp/registry.rs` + `server.json` |
| Query execution | `QueryContextService` |
| JSON-RPC semantics | `json_rpc.rs::json_rpc_http_status` |
| Auth credential order | `handlers/auth/mod.rs::authenticate_request_async` (EC-14) |
| E2E harness | `tests/common/spec028_mcp.rs` |
| OIDC wiremock | `tests/common/oidc_wiremock.rs` |

---

## Gap Analysis: Code vs SOTA

| Layer | Implemented | Status |
|-------|-------------|--------|
| Transport + headers | `validate.rs`, `body.rs` | ✅ |
| Legacy protocol default (EC-03) | `meta.rs::normalize_protocol_version` | ✅ |
| Batch / notification / 1MB | `body.rs` | ✅ |
| OAuth PRM + JWT + OIDC wiremock | `auth/*` + e2e | ✅ |
| Bearer > API key (EC-14) | `authenticate_request_async` | ✅ |
| Debug granularity admin (EC-29) | `enforce_debug_granularity` | ✅ |
| Workspace claim fail-closed (EC-30) | `workspace_policy.rs` | ✅ |
| Prompt injection as data (EC-39) | retrieve/search only | ✅ |
| SSE retrieve stream (EC-09) | `sse.rs` + `Mcp-Stream: true` | ✅ |
| Per-tenant rate limit (EC-35) | `tenant_rate_limit_from_state` | ✅ |
| MCP Registry manifest + publish CI | `server.json`, workflow | ✅ publish-ready |
| Trace propagation | `propagation_from_meta` | ✅ |
| Keycloak prod OAuth smoke (EC-13..20) | wiremock covers path | 🟡 deferred live |

---

## Definition of Done — MCP SOTA

- [x] `POST /mcp` Streamable HTTP header validation
- [x] OAuth: 401 + PRM + JWT + OIDC e2e
- [x] Tool errors: HTTP 200 + JSON-RPC (-32602 / -32004); 403 stays HTTP 403
- [x] EC-MCP-01..08,10,11,12,14,16,21..27,29,30,34,35,38..40,41..44,45,46,09,39
- [x] No `Mcp-Session-Id` support (legacy ignored; modern rejected)
- [x] Role-aware debug `content_granularity`
- [x] Per-tenant rate limits on `POST /mcp`
- [x] MCP Registry `server.json` + `GET /.well-known/mcp/server.json`
- [x] Registry publish workflow (`mcp-publisher validate` + release CI)
- [x] SSE streaming for `edgequake_retrieve` (`Mcp-Stream: true`)
- [ ] EC-MCP-13,15,17..20 live Keycloak prod smoke (wiremock sufficient for CI)
- [ ] EC-MCP-28,31..33,36..37,47,48 (future / operational)

---

## MCP-E: SSE Streaming

Opt-in via `Mcp-Stream: true` header or `_meta.stream: true` on `tools/call` + `edgequake_retrieve`.

| Event | Content |
|-------|---------|
| Progress | `notifications/progress` (10 → 50 → 90) |
| Final | JSON-RPC response with `result` or `error` |
| Headers | `Content-Type: text/event-stream`, `X-Accel-Buffering: no` |
| Cancel (EC-09) | Client disconnect → `AbortHandle` on worker task |

---

## MCP Registry Publish

**Server name:** `io.github.raphaelmansuy/edgequake`

| Step | Command |
|------|---------|
| Validate | `make mcp-registry-validate` |
| Publish | `mcp-publisher login github && make mcp-registry-publish` |
| CI | GitHub Release → `.github/workflows/mcp-registry-publish.yml` |

---

## Architecture (as implemented)

```
edgequake-api/src/mcp/
  registry.rs
  gateway/
    sse.rs               # MCP-E SSE retrieve stream
    workspace_policy.rs  # EC-30
    body.rs, validate.rs, meta.rs, json_rpc.rs
    dispatch.rs          # execute_tool_call SSOT
    tools.rs, tool_validation.rs
  auth/
handlers/mcp/mod.rs      # JSON + SSE response adapter
routes.rs                # /mcp + rate limit + well-known endpoints
```

---

## References

| Date | Action |
|------|--------|
| 2026-06-28 | MCP-E SSE + EC-30/39; MCP total 58 tests; phases A–E complete |
| 2026-07-28 | Re-diff MCP final spec against transport modules |
| Quarterly | Refresh `005-client-compatibility-matrix.md` |
