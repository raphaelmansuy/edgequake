# 08 — API contract

Parent: [README](README.md) · Prev: [07-data-and-db-contract](07-data-and-db-contract.md) ·
Next: [09-ux-ui-spec](09-ux-ui-spec.md)

All routes are under `/api/v1`. Every response is `application/json` unless stated. Denials
carry `{"error": {"kind":"forbidden","reason_code":"…"}}` — a stable code, per
[LAW-181-5](01-first-principles.md).

## Reason codes (the complete set this pack introduces)

| Code | Status | Meaning | UI action offered |
|------|--------|---------|-------------------|
| `capability_required:<id>` | 403 | The caller lacks a named capability | none — the control is normally absent |
| `platform_admin_required` | 403 | Deployment operation | none |
| `tenant_slug_taken` | 409 | Slug exists; body carries `existing_id` | "Open it" |
| `workspace_slug_taken` | 409 | Slug exists in this tenant | inline |
| `workspace_quota_reached` | 409 | `used >= max_workspaces` (live rows only) | "Archive a workspace" |
| `seat_quota_reached` | 409 | `used >= max_users` | "Ask your admin" |
| `last_owner` | 409 | Would orphan the organization | "Promote another owner first" |
| `last_platform_admin` | 409 | Would orphan the deployment | none |
| `grant_exists` | 409 | Active grant for (user, tenant, workspace) | "Change role" |
| `grant_scope_mismatch` | 400 | Workspace not in this tenant | none |
| `insufficient_role` | 403 | Requested role outranks the actor | select hides it |
| `self_suspend` | 409 | Suspending own account | none |
| `scope_inactive` | 409 | Target archived/disabled | "Restore" |
| `purge_in_progress` | 409 | Delete already running | "View progress" |
| `embedding_not_multimodal` | 422 | Image modality with a text-only model | model picker |
| `tenant_requires_owner` | 409 | DB trigger refused the write | internal |

---

## `GET /me/access`

The one call every surface reads. Replaces nothing (SPEC-175's `me/settings-access` stays for
compatibility) and supersedes it in the UI.

**Auth** any signed-in principal.

**Query** `tenant_id?`, `workspace_id?`. Absent ⇒ the caller's default organization.

```json
{
  "user_id": "1f0c…",
  "platform": { "role": "user", "admin": false },
  "organization": {
    "id": "9a2b…", "name": "Acme", "slug": "acme",
    "role": "member", "level": 2,
    "is_active": true,
    "capabilities": ["organization.read"]
  },
  "workspaces": [
    { "id": "4c1d…", "name": "Legal",   "slug": "legal",   "role": "admin",    "level": 3,
      "capabilities": ["workspace.read","workspace.content.write",
                       "workspace.settings.manage","workspace.members.manage"] },
    { "id": "7e9f…", "name": "Finance", "slug": "finance", "role": "readonly", "level": 1,
      "capabilities": ["workspace.read"] }
  ],
  "quota": {
    "workspaces": { "used": 3, "limit": 10 },
    "seats":      { "used": 9, "limit": 25 }
  },
  "can_create": { "organization": false, "workspace": false },
  "can_delete": { "organization": false },
  "reason": {
    "organization": null,
    "workspace": "capability_required:organization.workspaces.create"
  }
}
```

Rules:

- `organization` is `null` for a person whose only grants are workspace-scoped.
- `workspaces` lists only workspaces the caller can **read**; each row already includes the
  organization inheritance (an org Admin sees every workspace with `role:"admin"`).
- `can_create.workspace` is `false` when the capability is missing **or** the quota is full;
  `reason` distinguishes them so the UI can choose absent vs disabled.
- `quota.workspaces.limit` is the organization's `max_workspaces`; `used` counts
  `archived_at IS NULL`.

---

## `POST /tenants`

**Auth** platform admin. **Change** no longer an upsert ([LAW-181-8](01-first-principles.md)).

```json
{
  "name": "Globex",
  "slug": "globex",
  "description": "…",
  "plan": "free",
  "owner": { "kind": "self" },
  "default_llm_model": "gpt-5-nano",
  "default_llm_provider": "openai",
  "default_embedding_model": "text-embedding-3-small",
  "default_embedding_provider": "openai",
  "default_embedding_dimension": 1536,
  "default_vision_llm_model": null,
  "default_vision_llm_provider": null,
  "pdf_parser_backend": "vision",
  "primary_color": "#4b5bd6",
  "workspace": { "name": "Default Workspace", "slug": "default", "rag_profile": "graph" }
}
```

`owner` is a discriminated union:

| Shape | Meaning |
|-------|---------|
| `{"kind":"self"}` | the calling principal |
| `{"kind":"user","user_id":"…"}` | an existing user in any organization |
| `{"kind":"email","email":"…"}` | creates an invitation for that address and makes the creator the interim Owner |

`{"kind":"email"}` is the enterprise onboarding path ("I am the platform; the customer's IT
lead will be the owner"). It works because the org must have an owner immediately: the creator
holds Owner until the invitee accepts, and the accept promotes them and demotes the creator to
Admin — one transaction, recorded in the audit row.

**201 Created**

```json
{
  "tenant": { "id":"…", "name":"Globex", "slug":"globex", "plan":"free",
              "max_workspaces":10, "max_users":3,
              "workspaces_used":1, "members_used":1, "my_role":"owner" },
  "grants": [
    { "scope":"organization", "role":"owner", "source":"created" },
    { "scope":"workspace", "workspace_id":"…", "role":"owner", "source":"created" }
  ],
  "workspace": { "id":"…", "name":"Default Workspace", "slug":"default" },
  "invitation_token": null
}
```

`grants` is the proof of [LAW-181-2](01-first-principles.md): the response states what the
transaction created, so the UI can render a confirmation without a second round trip and a
test can assert it.

**Errors** 403 `platform_admin_required` · 409 `tenant_slug_taken` (+`existing_id`) ·
400 `invalid_slug` · 422 `embedding_not_multimodal` · 409 `tenant_requires_owner` (trigger).

---

## `POST /tenants/{tenant_id}/workspaces`

**Auth** `organization.workspaces.create` (org Admin/Owner) or platform admin.

Body is the existing `CreateWorkspaceApiRequest` (unchanged — it is already complete) plus
nothing. The **response** gains:

```json
{
  "…": "existing WorkspaceResponse fields",
  "grants": [],
  "quota": { "used": 4, "limit": 10 }
}
```

`grants` is empty when the caller already inherits the workspace (org manager); it carries a
`workspace/admin` grant when the caller is a **platform admin acting as support** and therefore
wants a durable row — which is the one case where a materialized grant is correct.

**Errors** 403 `capability_required:organization.workspaces.create` ·
409 `workspace_quota_reached` · 409 `workspace_slug_taken` · 404 org not found ·
422 `embedding_not_multimodal` · 422 `extraction_mode_not_supported` (SPEC-180).

---

## `GET /tenants` — changed semantics

| Caller | Returns |
|--------|---------|
| platform admin | every tenant, with `workspaces_used` / `members_used` |
| anyone else | only tenants where they hold an active membership of **any** kind |

The current implementation branches on `membership_scoped(&state)` — an *environment* flag
(`strict_tenant_bind` or SSO active). That means a single-tenant install silently lists every
tenant to every member. This pack makes the branch a function of the caller's rows, not of an
env var ([EC-181-29](12-edge-cases.md)). Legacy behaviour remains available under
`EDGEQUAKE_LEGACY_TENANT_LIST=1` for one release.

---

## `PUT /tenants/{tenant_id}` — changed

`name`, `description`, `primary_color`, `plan`, `is_active`, `pdf_parser_backend`, and the six
model defaults remain writable by an org Admin.

New rules:
- `plan` requires `organization.grant_owner` (Owner only) and rejects a change that would put
  `workspaces_used` above the new `max_workspaces` → 409 `workspace_quota_reached`.
- `max_workspaces` / `max_users` are **not** writable here. Only
  `PATCH /admin/tenants/{id}/quota` (platform) writes them, so an owner cannot raise their own
  limit.
- `slug` is immutable → 400 `slug_immutable`.

---

## `DELETE /tenants/{tenant_id}` — changed to async + receipt

**Auth** platform admin, or organization Owner.

**202 Accepted**

```json
{ "receipt_id": "…", "status": "running", "scope": { "kind":"organization", "id":"…" },
  "planes_total": 9, "planes_done": 0 }
```

**409** `purge_in_progress` (+`receipt_id`) when one is already running.

### `GET /tenants/{tenant_id}/purge-receipts/{receipt_id}`

```json
{ "receipt_id":"…", "status":"partial",
  "planes": { "tasks":{"status":"done"}, "vectors":{"status":"done","count":18},
              "graph":{"status":"done"}, "kv":{"status":"done"},
              "pdf_rows":{"status":"done"}, "originals":{"status":"failed","error":"S3 503"},
              "decision":{"status":"done"}, "vector_table":{"status":"done"},
              "row":{"status":"done"} } }
```

`status: partial` means the organization row is gone but one plane failed; a retry
(`POST …/purge-receipts/{id}/retry`) re-runs only the failed planes. This is why the receipt is
a table and not a log line.

---

## Grants

### `GET /tenants/{tenant_id}/grants?user_id=&workspace_id=`

```json
{ "grants": [
    { "membership_id":"…", "user_id":"…", "username":"aicha",
      "scope":{"kind":"organization"}, "role":"member", "level":2,
      "is_active":true, "granted_by":"…", "joined_at":"…" },
    { "membership_id":"…", "user_id":"…", "username":"aicha",
      "scope":{"kind":"workspace","workspace_id":"…","workspace_name":"Legal"},
      "role":"admin", "level":3, "is_active":true, "granted_by":"…", "joined_at":"…" }
  ] }
```

**Auth** `organization.members.manage`, or `workspace.members.manage` for grants in that
workspace only (a workspace Admin sees the rows for their workspace and no organization rows —
[EC-181-20](12-edge-cases.md)).

### `POST /tenants/{tenant_id}/grants`

```json
{ "user_id": "…", "workspace_id": null, "role": "admin" }
```

Creates a grant. `workspace_id: null` = organization grant.

**Auth**
- organization grant → `organization.members.manage`, and `role` must be ≤ actor level, and
  `role: "owner"` needs `organization.grant_owner`.
- workspace grant → `organization.members.manage` **or**
  `workspace.members.manage` on that workspace; `role` ≤ actor level in that scope.

**201** the created grant. **409** `grant_exists` · **400** `grant_scope_mismatch` ·
**403** `insufficient_role`.

### `PATCH /tenants/{tenant_id}/grants/{membership_id}`

```json
{ "role": "member" }
```

Same authorization as create, evaluated against **both** the current and the new role.
Demoting the last Owner → 409 `last_owner`. Editing an SSO-managed grant pins it
(`metadata.source = manual`, `previous_source = sso`) — the existing
`pg_assign_membership_role` already does this.

### `DELETE /tenants/{tenant_id}/grants/{membership_id}`

Removes one grant. Removing the last Owner grant → 409 `last_owner` (and the DB trigger agrees).
Removing your own non-owner grant is allowed (leaving a workspace).

---

## `POST /tenants/{tenant_id}/invitations` — `workspace_id` becomes first class

Unchanged shape; the field is now validated:

| Field | Rule |
|-------|------|
| `workspace_id` | when present, must belong to `tenant_id` → else 400 `grant_scope_mismatch` |
| `role` | ≤ actor level **in the invited scope** |
| seat check | `max_users` counts distinct active memberships; a workspace-only invite is a new person ⇒ counts |

The preview (`GET /auth/invitations/{token}`) gains `workspace_name` so the accept page can say
"Join **Legal** in Acme".

---

## `POST /workspaces/{workspace_id}/archive`

```json
{ "reason": "quota" }
```

**Auth** `workspace.settings.manage`. Sets `archived_at`, `archived_by`, `is_active=false`.
The workspace leaves the switcher and stops counting toward quota.
**200** the workspace. **409** `scope_inactive` when already archived.

`POST /workspaces/{workspace_id}/restore` reverses it, subject to quota (a restore that would
exceed the limit → 409 `workspace_quota_reached`).

---

## `GET /roles` — additive

`CapabilityInfo` gains two fields:

```json
{ "id": "organization.workspaces.create", "axis": "organization", "min_role": "admin",
  "group": "Organization", "label": "Create and configure workspaces",
  "roles": ["owner","admin"] }
```

`OrganizationRoleInfo` / `InstanceRoleInfo` keep their shape. The matrix the UI renders is
`capabilities × roles` filtered by `axis`, so a workspace capability is never drawn under the
organization columns.

---

## OpenAPI

- Register: `get_me_access`, `post_grant`, `patch_grant`, `delete_grant`, `list_grants`,
  `archive_workspace`, `restore_workspace`, `get_purge_receipt`, `retry_purge`.
- New schemas: `MeAccessResponse`, `GrantView`, `GrantBody`, `PurgeReceipt`,
  `CreateOrganizationOwner` (discriminated union).
- Regenerate the snapshot: `make codegen-openapi-refresh`, then
  `cargo test -p edgequake-api --test spec027_api_contract`.
- `TenantResponse` gains `workspaces_used`, `members_used`, `my_role`, `archived_at`; these are
  additive and `skip_serializing_if` is **not** used, so a snapshot diff is explicit.

---

## Backwards compatibility ledger

| Consumer | Break? | Mitigation |
|----------|--------|------------|
| WebUI's `getTenants` | no | new fields ignored |
| `POST /tenants` returning 200 on conflict | **yes** | old WebUI never relied on 200; only the wizard read `tenant.id` |
| `DELETE /tenants` 204 → 202 | **yes** | WebUI is updated in the same wave; SDKs get a note |
| `me/settings-access` | no | kept, implemented over `me/access` |
| `GET /tenants` visibility | **yes** for single-tenant installs | `EDGEQUAKE_LEGACY_TENANT_LIST=1` for one release |
| MCP tools (`eq_workspace_list`) | no | reads the same route; now correctly scoped |


---

## Implementation notes (found while building it)

Three things the contract above states had to be made true in code, and are recorded here so a
reviewer can see the difference between the design and the shipped behaviour.

### 1. Two quota checks must answer the same reason code

There are two gates: the application pre-check in `pg_create_workspace` (a specific message on
the common path) and the `trg_workspace_quota_guard` trigger in migration 226 (the authoritative,
serialized one). They originally disagreed — the pre-check returned
`400 Validation error: Tenant has reached maximum workspace limit`, the trigger returned a 409 —
so the UI could not tell "you lack the role" from "archive something".

Both now return `409 workspace_quota_reached`. Gate: `concurrent_workspace_creates_respect_the_quota`
fails if either regresses.

### 2. Both quota checks count live rows only

The pre-check counted every workspace; the trigger counts `archived_at IS NULL`. A caller who
archived a workspace to free a slot was therefore still refused. Both count live rows now, which
is also what `me/access.quota.workspaces.used` reports. Gate: `R-archive-frees-quota`.

### 3. The default workspace is read back, not assumed

`POST /tenants` creates the default workspace inside its transaction. The client reads
`GET /tenants/{id}/workspaces` to name it in the confirmation, rather than taking
`me/access.workspaces[0]` — which can be a stale sibling while the access cache settles
(produced `/w/legal` on a brand-new organization during round-09 of the visual loop).

### 4. Selection happens when the confirmation closes

`TenantGuard` renders the create dialog *inside* its "no organizations yet" branch. Invalidating
`['tenants']` while the dialog is open makes the guard see a non-empty list, drop the branch and
unmount the confirmation before it is read. Creation therefore settles the cache and switches
scope on **close**, not on response. Gates: `E-*` in
[14](14-e2e-test-matrix.md#create-organizationspec.ts).


---

## Shipped surface (what the routes actually are)

The sections above were the design. These are the routes that exist after implementation, plus
the three behaviours the tests forced us to change.

| Method | Path | Who | Notes |
|--------|------|-----|-------|
| `GET` | `/me/access` | any signed-in | capabilities + quota + `can_create` |
| `POST` | `/tenants` | platform admin | creates owner grant + default workspace, 409 on slug |
| `DELETE` | `/tenants/{id}` | platform admin | 202 + purge receipt |
| `GET` | `/tenants/{id}/purge-preview` | platform admin | counts for the confirmation |
| `GET` | `/tenants` | any signed-in | **row-scoped** (`EDGEQUAKE_LEGACY_TENANT_LIST=1` restores the old behaviour for one release) |
| `POST` | `/workspaces/{id}/archive` | `workspace.settings.manage` | frees a plan slot, keeps data |
| `POST` | `/workspaces/{id}/restore` | `workspace.settings.manage` | 409 when the plan is full |
| `DELETE` | `/workspaces/{id}` | workspace Owner or org manager | 202 + purge receipt |
| `GET` | `/workspaces/{id}/purge-preview` | workspace reader | counts |
| `GET` | `/tenants/{id}/grants` | org manager | `?user_id=`, `?workspace_id=` |
| `POST` | `/tenants/{id}/grants` | org manager | workspace grant is finally writable |
| `PATCH` | `/tenants/{id}/grants/{membership_id}` | org manager | rank + last-owner rules |
| `DELETE` | `/tenants/{id}/grants/{membership_id}` | org manager | rank + last-owner rules |

### Three behaviours the implementation had to change

1. **Archive/restore must reach an inactive workspace.** `require_active_scope` refused every
   request whose workspace was `is_active = false`, which made restore impossible: the only way
   to un-archive is to address the archived row. Those two paths now resolve the caller's grant
   without the active-workspace check and rely on `require_workspace_settings`, so a member
   still cannot touch them.
2. **`list_workspaces` keeps returning every workspace.** An earlier attempt filtered archived
   rows inside the store method, which silently broke organization purge (it enumerates
   workspaces to delete, archived ones included) and the purge preview count. The filter moved to
   the two read surfaces — `me/access` and the HTTP catalog — where "archived leaves the
   switcher" is the actual requirement.
3. **Grant mutations need real membership ids.** The directory's `grants` summary is a
   projection without ids, so the editor fetches `/tenants/{id}/grants?user_id=` before it can
   change anything. Synthesizing an id from scope + index would have silently PATCHed the wrong
   row.
