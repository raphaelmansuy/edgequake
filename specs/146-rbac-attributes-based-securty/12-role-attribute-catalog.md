# 12 — Role & Attribute Catalog (SPEC-146)

## Unify three vocabularies (LAW-146-15)

```ascii
  ┌──────────────────┬─────────────────────┬────────────────────────┐
  │ Global users.role│ Membership role     │ Principal kind         │
  ├──────────────────┼─────────────────────┼────────────────────────┤
  │ admin            │ owner / admin       │ human                  │
  │ user (operator)  │ member (editor)     │ api_key.query_agent    │
  │ readonly (viewer)│ readonly (viewer)   │ api_key.ingestion_svc  │
  │                  │                     │ break_glass (master)   │
  │                  │                     │ auditor (optional)     │
  └──────────────────┴─────────────────────┴────────────────────────┘

  WebUI map (M0):
    developer → user
    viewer    → readonly
    admin     → admin
```

Document access is **not** a fourth value on `users.role`. It is:

1. Capability permissions (RBAC), and  
2. ACL rows + resource attrs + Cedar (ABAC).

## Built-in workspace roles

| Role | Intent | Typical permissions |
|------|--------|---------------------|
| `viewer` | Read authorized docs + query + graph | `document:read`, `chunk:read`, `graph:read`, `query:execute`, `mcp:invoke` |
| `editor` | Upload/update/reprocess; delete own | viewer + `document:create|update|delete|set_labels`, `task:*` |
| `admin` | Workspace settings, members, policies | editor + `workspace:manage_members`, `policy:manage`, `user:read` (ws), `audit:read` |
| `ingestion_service` | Admit/convert/insert only | `document:create`, `task:create|read` — **no** `query:execute` / `graph:read` |
| `query_agent` | Query/chat/MCP; no ingest/admin | `document:read`, `query:execute`, `mcp:invoke`, `graph:read` |
| `auditor` | Read audit + metadata patterns | `audit:read`, limited `document:list_meta` (still existence-hiding for content) |
| `break_glass` | Emergency content access | All content read + **mandatory audit** + UI banner |

Membership `owner` maps to workspace `admin` capabilities + ownership transfer rules (EC-146-30).

## Permission extensions

Extend [`Permission`](../../edgequake/crates/edgequake-auth/src/rbac.rs) (do not fork):

| New / emphasize | String | Notes |
|-----------------|--------|-------|
| DocumentSetLabels | `document:set_labels` | Change classification/ACL |
| DocumentListMeta | `document:list_meta` | Authorized titles only |
| ChunkRead | `chunk:read` | Implied by document.read for authorized |
| GraphRead | `graph:read` | Authorized neighborhood |
| PolicyManage | `policy:manage` | PAP |
| AuditRead | `system:audit_log` | Already exists — wire it |
| McpInvoke | `mcp:invoke` | MCP tools |
| BreakGlass | `system:break_glass` | Master / explicit grant |

Wire `require_permission` at every PEP (F-146-15).

## Share modes

| Mode | Semantics |
|------|-----------|
| `workspace` | Any workspace member with `document:read` (legacy default) |
| `acl` | Explicit `document_acl` grants (+ owner) |
| `classified` | Cedar/attrs; missing required subject/resource attrs → deny / quarantine on mint |
| `owner_only` | Owner principal (+ break-glass) |

```ascii
  share_mode decision tree (AllowSet)
       |
       +-- quarantined? ──► exclude
       |
       +-- workspace ──► capability document:read?
       +-- owner_only ──► owner_principal == principal?
       +-- acl ──► ACL row OR owner?
       +-- classified ──► Cedar allow? (fail-closed on miss)
```

## Attribute classes

### Subject (principal_attributes / OIDC map)

| Name | Type | Example |
|------|------|---------|
| `clearance` | enum | `public`, `internal`, `confidential`, `secret` |
| `department` | string | `oncology` |
| `geo` | string | `US`, `EU` |
| `employment` | enum | `employee`, `contractor`, `partner` |
| `citizenship` | string | optional |
| `need_to_know` | string_set | `project-alpha` |
| `idp_groups` | string_set | from OIDC |

### Resource (document columns + security_attrs)

| Name | Column / bag | Notes |
|------|--------------|-------|
| `classification` | column | Align enum with clearance lattice |
| `project_id` | column | Need-to-know match |
| `export_control` | column | bool |
| `pii` | column | bool |
| `owner_org` | column | |
| `retention_until` | column | |
| `share_mode` | column | |
| extras | `security_attrs` JSONB | Workspace-defined |

### Environment (request context — optional v1)

| Name | Source |
|------|--------|
| `time_window` | server clock vs policy |
| `network_zone` | header / gateway claim (if configured) |
| `device_trust` | future |

Default workspace policies may ignore environment until configured.

## Clearance lattice (default template)

```ascii
  public < internal < confidential < secret

  principal.clearance must be >= resource.classification
  (unless need_to_know / project exception policy permits)
```

Exact Cedar templates ship in M0 spike; operators can replace via PAP.

## API key principal mapping

| Mint default | Kind | Caps |
|--------------|------|------|
| User-created key (scopes read/write) | `query_agent` + viewer/editor per scopes | Not Admin unless scope `admin` **and** break-glass policy |
| Ingest automation key | `ingestion_service` | No query |
| Master / env key | `break_glass` | Audited all-doc; never silent |

## Members API (new)

```ascii
  GET    /api/v1/workspaces/{id}/members
  POST   /api/v1/workspaces/{id}/members
  PATCH  /api/v1/workspaces/{id}/members/{principal_id}
  DELETE /api/v1/workspaces/{id}/members/{principal_id}

  GET/PUT /api/v1/workspaces/{id}/members/{principal_id}/attributes
  GET/POST /api/v1/workspaces/{id}/roles
  GET/PUT  /api/v1/workspaces/{id}/attribute-definitions
  GET/POST /api/v1/workspaces/{id}/policies
  POST     /api/v1/workspaces/{id}/policies/{id}/versions
```

OpenAPI matrix 401/403/404 in M5.

## Cross-refs

- Laws → [00-first-principles.md](00-first-principles.md)  
- Data model → [05-data-model.md](05-data-model.md)  
- UX → [06-ux-ui-spec.md](06-ux-ui-spec.md)  
- Security lens → [05-lenses/007-security-expert.md](05-lenses/007-security-expert.md)  
