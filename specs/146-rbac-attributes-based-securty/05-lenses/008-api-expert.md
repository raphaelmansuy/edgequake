# Lens — API Expert (SPEC-146)

## Outcome

OpenAPI documents a clear 401/403/404 matrix; members/roles/attrs/policies are first-class; query/MCP never accept client ACL bypass; list PEP is application-side on KV.

## New / extended routes

```ascii
  Workspaces
    GET/POST   /api/v1/workspaces/{id}/members
    PATCH/DEL  /api/v1/workspaces/{id}/members/{principal_kind}/{principal_id}
    GET/PUT    .../attributes
    GET/POST   /api/v1/workspaces/{id}/roles
    PATCH/DEL  /api/v1/workspaces/{id}/roles/{role_id}
    GET/POST   /api/v1/workspaces/{id}/attribute-definitions
    GET/POST   /api/v1/workspaces/{id}/policies
    POST       /api/v1/workspaces/{id}/policies/{policy_id}/versions
    POST       /api/v1/workspaces/{id}/policies/dry-run

  Documents
    POST upload*          + security fields (JSON / multipart)
    PATCH /documents/{id}/security
    POST  /documents/{id}/security/retry-labels
    GET list/detail       existence-hiding; list = KV PEP
    status_counts         authorized-only when ABAC on

  Query / MCP
    document_filter ∩ allow-set (None forbidden when ABAC on)
    reject bypass_acl

  Parse
    GET /parse/jobs/{id}  bound to minting principal (IDOR fix)
```

## Status matrix

| Situation | Status |
|-----------|--------|
| Missing/invalid token | 401 |
| Authenticated, no capability (ingest/policy/members) | 403 |
| Authenticated, resource not in allow-set | **404** |
| Quarantine retry without edit right | 403 |
| Policy validate fail | 400 |
| ABAC flag off | pre-146 behavior |
| ABAC=1 + auth off | process refuse (not a request status) |

## DTO sketches

```ascii
  PrincipalRef { kind: user|api_key|master|worker|group, id: string }

  DocumentSecurity {
    classification, share_mode, project_id,
    export_control, pii, owner_org, retention_until,
    acl: [{ principal: PrincipalRef, permission }],
    security_status, policy_etag
  }

  AuthzContextHeader (debug/admin only):
    policy_version — never leak allow-set size to clients
```

## Headers

Keep `X-Tenant-ID` / `X-Workspace-ID`. When ABAC on, membership verified even if `STRICT_TENANT_BIND=false` (LAW-146-20). Recommend strict bind in ops docs.

## Codegen

`make codegen-openapi-refresh` after route land; WebUI `schema.d.ts` SSOT.

## Idempotency

- Policy version publish: immutable; duplicate POST → 409 or new version.  
- Retry-labels: idempotent if attrs unchanged.  
- Members POST: upsert or 409 on duplicate.

## Cross-refs

- Catalog → [../12-role-attribute-catalog.md](../12-role-attribute-catalog.md)  
- E2E → [../08-e2e-test-matrix.md](../08-e2e-test-matrix.md)  
- Honest → [../13-honest-assessment.md](../13-honest-assessment.md)  
- SPEC-027 security lens  
