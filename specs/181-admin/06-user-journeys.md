# 06 — User journeys (J-01 … J-12)

Parent: [README](README.md) · Prev: [05-authorization-model](05-authorization-model.md) ·
Next: [07-data-and-db-contract](07-data-and-db-contract.md)

Twelve end-to-end workflows. Each names the actor, the precondition (the grants that exist
*before* the journey starts), the steps, the exact rows written, and the refusal paths. Nothing
appears here that the code cannot do after this pack lands.

Legend: `PA` = platform admin · `OO` = organization owner · `OA` = organization admin ·
`OM` = organization member · `OV` = organization viewer · `WA/WM/WV` = workspace admin /
member / viewer.

---

## J-01 Platform admin creates an organization

**Actor** Dana (`PA`). **Precondition** none.

```text
 1  Dana opens the header switcher                        GET /tenants, /me/access
 2  "New organization" is present (can_create.organization = true)
 3  Dialog: Name "Globex"
              Owner  (•) me   ( ) another person by email
              Defaults (•) server defaults   ( ) copy from… (list)
              [Tune later ▾]  0 changes
 4  Create                                                 POST /tenants
 5  Confirmation card: org, owner, default workspace, quota 3/10, [Invite people]
```

**Rows written (one transaction, [LAW-181-2](01-first-principles.md))**

| Table | Row |
|-------|-----|
| `tenants` | `{id, name:'Globex', slug:'globex', plan:'free', max_workspaces:10, max_users:3}` |
| `memberships` | `{user: dana, tenant: globex, workspace: NULL, role: owner}` |
| `workspaces` | `{name:'Default Workspace', slug:'default', rag_profile:'graph'}` |
| `memberships` | `{user: dana, tenant: globex, workspace: default, role: owner}` |
| `audit_logs` | `TenantAccess / create_organization / actor_kind=platform_admin` |

**Refusals**

| Cause | Response | UI |
|-------|----------|----|
| Not platform admin | 403 `capability_required:platform.organizations.create` | dialog never opens |
| Slug exists | 409 `tenant_slug_taken` + `existing_id` | inline "Globex already exists — **Open it**" |

**After** Dana's switcher lists Globex; her `me/access` for Globex reports
`organization.role=owner`.

---

## J-02 Organization owner creates a workspace

**Actor** Marc (`OO` of Acme; owns 3 workspaces of 10).

```text
 1  Switcher → Acme → "New workspace"                      present (org admin)
 2  Dialog header: "3 of 10 workspaces used"
 3  Name        [Legal            ]
    Retrieval   (o) Graph — entities and relations     ( ) Normal — text only
    Models      Inherited from Acme: gpt-5-nano / text-embedding-3-small   [Change]
    [Tune later ▾]  Parsing · Chunking · Budget · Entity types · Language · Decision
 4  Create                                                  POST /tenants/acme/workspaces
 5  Navigates to /w/legal ; toast "Legal created · Upload documents / Invite people"
```

**Rows written**

| Table | Row |
|-------|-----|
| `workspaces` | `{name:'Legal', slug:'legal', rag_profile:'graph', embedding_modalities:'text'}` |
| (no membership — Marc is already an implicit member via his org grant) | — |
| `audit_logs` | `WorkspaceAccess / create_workspace` |

**Why no membership row:** `Membership::applies_to(tenant, Some(legal))` is true for a
tenant-wide row. Writing a duplicate workspace grant would be noise, and
`memberships_user_tenant_workspace_uidx` would allow it but the editor would show two rows for
one effective role. The rule: **an implicit grant is never materialized.**

**Refusals**

| Cause | Response |
|-------|----------|
| Org Member | 403 `capability_required:organization.workspaces.create` (button absent) |
| At limit | 409 `workspace_quota_reached` (button disabled + reason) |
| Slug taken in this org | 409 `workspace_slug_taken` |
| Image embedding chosen, model cannot read images | 422 `embedding_not_multimodal` (next to the control) |

---

## J-03 Workspace admin creates a workspace — **and is refused**

**Actor** Aïcha (`OM` + `WA` of Legal). This journey exists to prove the negative.

```text
 1  Switcher → Acme → Legal
 2  "New workspace" ABSENT (no organization.workspaces.create)
 3  She opens /settings/organization/workspaces by URL
 4  Section renders "Only organization owners and admins create workspaces."
    with the list of workspaces she can administer (Legal) — no create button
```

**Why this matters:** today the button is present and the wizard runs for eight steps before
`require_tenant_access(..., Manage)` returns 403 ([F-181-02](04-findings.md)).

---

## J-04 Organization owner invites an organization admin

**Actor** Marc (`OO`).

```text
 1  Organization → People  →  [Invite people]
 2  Email  [ines@acme.test]
    Scope  (o) Whole organization     ( ) One workspace ▾
    Role   [Admin ▾]      (Owner is not in the list: only an Owner may grant Owner)
 3  Invite                                                  POST /tenants/acme/invitations
 4  Single-use link shown once, copy affordance, expiry
```

**Rows written**

| Table | Row |
|-------|-----|
| `tenant_invitations` | `{tenant, workspace_id: NULL, email, role: admin, token_hash, expires_at}` |
| (`audit_logs` on accept, not on invite) | — |

**On accept** (`POST /auth/invitations/{token}/accept`): `users` (if new) +
`memberships{tenant, NULL, admin}`.

**Refusal:** `role=owner` when the inviter is an Admin → 403 `insufficient_role`
(the select does not offer it).

---

## J-05 Organization owner grants one workspace's admin

**Actor** Marc (`OO`). **Target** Aïcha (`OM`, currently no workspace grant).

```text
 1  Organization → People → Aïcha → Grant editor
 2  Existing:  Organization · Member
 3  [+ Add a workspace grant]
    Workspace [Legal ▾]     Role [Admin ▾]
 4  Save                                                     POST /tenants/acme/workspaces/legal/grants
 5  Row appears:  Legal · Admin
```

**Rows written**

| Table | Row |
|-------|-----|
| `memberships` | `{user: aicha, tenant: acme, workspace: legal, role: admin}` |
| `audit_logs` | `Authorization / grant_create / detail:"legal:admin"` |

**Effect on the next request.** `me/access` now returns `workspaces[legal].role=admin`
with `workspace.settings.manage` and `workspace.members.manage`.

**Refusals**

| Cause | Response |
|-------|----------|
| Workspace belongs to another tenant | 400 `grant_scope_mismatch` |
| Actor is workspace Admin, not org manager | 403 `capability_required:workspace.members.manage` only for that workspace — and cannot target a sibling |
| Requested role above actor's level in that scope | 403 `insufficient_role` |
| A grant already exists for (user, tenant, workspace) and is active | 409 `grant_exists` → offer **Change** |

---

## J-06 Workspace admin invites to their own workspace

**Actor** Aïcha (`WA` Legal).

```text
 1  Legal → Members
 2  Banner: "Organization membership is managed by Acme's owners.
             You can add people to Legal."
 3  Directory lists only people with a Legal grant
 4  [Invite to Legal]
    Email [tom@acme.test]   Role [Member ▾]   (Owner not offered)
 5  Invite                                                  POST /tenants/acme/invitations
                                                              {email, role:'member', workspace_id:legal}
```

**On accept** → `memberships{tenant:acme, workspace:legal, role:member}` — **no** org-wide row.
Tom can read Legal and nothing else; his `me/access.organization.role` is `null`.

**Refusal:** Aïcha opens Finance → Members. `403` → the page shows the restricted state and no
invite button ([EC-181-20](12-edge-cases.md)).

---

## J-07 Platform admin gives support access to a customer organization

**Actor** Dana (`PA`). **Target** Globex, owned by a customer.

```text
 1  Settings → Deployment → Organizations → Globex
 2  Full organization page renders, header chip "Platform admin" (not "Owner")
 3  Every write is available and every write is audited with actor_kind=platform_admin
 4  A persistent banner: "You are viewing Globex as a platform administrator.
                         Your changes are recorded in the audit log."
```

**Why a banner:** support access is the highest-risk capability in the product. The chip
already distinguishes the axes ([LAW-175-17](../175-perfect-settings/15-roles.md)); the banner
makes the *act* visible, so a screenshot in a support ticket is unambiguous.

**Note:** the platform admin is **not** written as a membership. `require_tenant_access`
short-circuits before the lookup, and `last_owner` must not count them.

---

## J-08 Organization owner renames and re-brands

**Actor** Marc (`OO`).

```text
 1  Settings → Organization → General
 2  Name [Acme Research]   Slug acme (link icon, warning "changing breaks links")
    Description …
    Brand color  [■] #4b5bd6     preview light/dark, contrast AA readout
 3  Save                                                     PUT /tenants/acme
```

**Rows written:** `tenants.name`, `tenants.metadata.primary_color`, `audit_logs`.

**Slug is immutable** in this pack. Changing it would invalidate `/w/{slug}` links and every
SSO binding keyed on `tenants.slug`. The field is read-only with a "contact support" hint.

---

## J-09 Platform admin deletes a trial organization

**Actor** Dana (`PA`).

```text
 1  Organizations → globex → Delete
 2  Confirm dialog: type the slug; the dialog lists:
       3 workspaces · 412 documents · 1.8 GB vectors · 340 MB originals
 3  Delete                                                   DELETE /tenants/{id}
 4  202 + receipt id; the row shows a progress strip per plane
 5  When every plane reports done: the row leaves the list, audit row written
```

**Rows written**

| Table | Change |
|-------|--------|
| `scope_purge_receipts` | one row, `planes` jsonb with 9 outcomes |
| every plane | deleted ([LAW-181-7](01-first-principles.md)) |
| `audit_logs` | `TenantAccess / delete_organization / detail:"globex"` |

**Refusals:** not platform admin → 403; already purging → 409 `purge_in_progress` (offers the
receipt).

---

## J-10 Organization owner archives a workspace instead of deleting

**Actor** Marc (`OO`). **Motivation** the plan is at 10/10 and he needs an 11th.

```text
 1  Legal → Settings → Danger zone → [Archive workspace]
 2  Copy: "Archived workspaces stop counting toward your plan. Documents and
           vectors are kept. You can restore for 30 days."
 3  Archive                                                  POST /workspaces/{w}/archive
 4  Workspace disappears from the switcher; Settings → Workspaces shows it under Archived
```

**Why this journey exists:** without archive, the only way to free a quota slot destroys data.
Every mature SaaS has this ([EC-181-16](12-edge-cases.md)).

**Rows written:** `workspaces.archived_at`, `workspaces.is_active=false`,
`audit_logs`. Quota counts only `archived_at IS NULL`.

---

## J-11 Member hits a wall and is told what to do

**Actor** Tom (`OM`, no workspace grant — his Legal grant was removed).

```text
 1  Header switcher: Acme (1 workspace visible? no — Legal was his only grant)
 2  TenantGuard: "You don't have access to a workspace in Acme yet."
                 "Ask an owner of Acme to add you to a workspace."
                 [Copy request] (copies a prefilled message)
 3  No create button. No spinner. No error boundary.
```

**Why:** today this is a "Create your first tenant" card whose button 403s
([F-181-10](04-findings.md)).

---

## J-12 A refusal keeps the work

**Actor** Marc (`OO`), at 10/10 workspaces, fills the workspace dialog anyway.

```text
 1  Header shows "10 of 10 workspaces used"
 2  Create is disabled; the reason is inline:
       "Your plan allows 10 workspaces. Archive one, or ask your platform admin
        to raise the limit."
 3  He archives an unused workspace in another tab
 4  Returning focus to the dialog: the quota strip now reads "9 of 10" and Create enables
 5  The name he typed is still there
```

The draft, the refusal and the recovery all live in [LAW-181-14](01-first-principles.md).

---

## Journey matrix — the negative space

The rows are the twelve journeys; the columns are the five personas. A cell says what that
persona sees at the same entry point. Every "refused" cell is a *designed* state with a
reason, not a 403 toast.

| Journey | PA | OO | OA | OM + WA | OM | OV |
|---------|----|----|----|---------|----|----|
| J-01 create org | **creates** | absent | absent | absent | absent | absent |
| J-02 create ws | creates | **creates** | **creates** | absent | absent | absent |
| J-03 ws admin tries | creates | creates | creates | **refused: copy** | absent | absent |
| J-04 invite org admin | can, as support | **invites** | invites ≤ admin | absent | absent | absent |
| J-05 grant ws admin | can | **grants** | grants ≤ admin | absent | absent | absent |
| J-06 invite to own ws | can | can | can | **invites** | absent | absent |
| J-07 support view | **enters** | n/a | n/a | n/a | n/a | n/a |
| J-08 rename org | can | **renames** | renames | absent | absent | absent |
| J-09 delete org | **deletes** | deletes (owner only) | refused: owner only | absent | absent | absent |
| J-10 archive ws | can | **archives** | archives | archives (own)? see I-15 | absent | absent |
| J-11 no-workspace wall | n/a | n/a | n/a | n/a | **instructed** | **instructed** |
| J-12 quota refusal | n/a | **disabled + reason** | disabled + reason | absent | absent | absent |

J-10 for a workspace Admin: archiving a workspace is `workspace.settings.manage` (Admin), not
`workspace.delete` (Owner). That asymmetry is deliberate — an admin can stop the workspace
being used, only the owner (or an org manager) can end it permanently.
