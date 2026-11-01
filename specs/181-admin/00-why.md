# 00 — WHY (five WHY chains)

Parent: [README](README.md) · Next: [01-first-principles](01-first-principles.md)

Each chain starts from a symptom an operator or a customer can see, drills to a single
root cause, and names the law that closes it. Nothing here is an opinion about taste;
every link is a file and a symbol that exists today.

---

## Chain A — "I created an organization and I am locked out of it"

| # | Question | Answer |
|---|----------|--------|
| 1 | Why can a platform admin create an organization and then see nothing in it? | `create_tenant` inserts the tenant row and, best-effort, a default workspace. It never inserts a membership. |
| 2 | Why is that fatal? | `require_tenant_access(..., TenantAccess::Manage)` resolves a role from `get_user_role`, which reads `memberships` only. No row → `denied()` → 403. |
| 3 | Why does the UI look fine? | The WebUI keeps the platform-admin JWT, and `is_platform_admin` short-circuits every tenant check, so the admin never notices. |
| 4 | Why does it break for the next person? | The organization has **zero** memberships, so `member_tenants` returns nothing for everyone who is not a platform admin, and `last_owner` is `None`. Nobody can invite, configure or delete it. |
| 5 | **Root cause** | **Organization creation is a data-plane insert, not a tenancy transaction. Ownership is never established.** |

```text
  POST /api/v1/tenants  (platform admin)
        |
        +--> INSERT tenants                         ok
        +--> create_workspace("Default Workspace")   best-effort, error swallowed
        +--> INSERT memberships (owner)             MISSING
        |
        v
  tenants row exists, memberships empty
        |
        +--> GET /tenants/{t}/members  -> 403 even for the org owner
        +--> members page shows "Members are managed by admins"
        +--> nobody can grant the first role: last_owner() == None
```

The handler's own comment says *"Continue anyway - tenant was created successfully"*
([tenants.rs:193-208](../../edgequake/crates/edgequake-api/src/handlers/workspaces/tenants.rs)),
and the swallow is `if let Err(e) = ... { tracing::warn! }`. A warn is not a rollback.

Cross-ref: [LAW-181-2](01-first-principles.md) · [F-181-01](04-findings.md) ·
[EC-181-01](12-edge-cases.md) · [W-06](09-ux-ui-spec.md#w-06-created-organization-confirmation)

---

## Chain B — "The button is there; the API refuses it"

| # | Question | Answer |
|---|----------|--------|
| 1 | Why does a Member see "New Organization" in the header switcher? | `ContextSelectorPopover` renders the create row for every caller; it takes no access prop. |
| 2 | Why is that a refusal rather than a silent no-op? | `POST /tenants` calls `require_platform_admin`, which returns 403 `Admin role required`. |
| 3 | Why does the same happen for workspaces? | `POST /tenants/{t}/workspaces` requires `TenantAccess::Manage`; a Member does not hold it. |
| 4 | Why is this expensive? | The operator fills an 8-step wizard — models, parsing, chunking, extract budget, entity types — and loses all of it at the last click. The wizard even persists a draft in `localStorage`, so the dead end is sticky. |
| 5 | **Root cause** | **The UI has no server-derived access model for creation. `me/settings-access` exists (SPEC-175) but no create surface reads it.** |

```text
  Member session
        |
        +--> header switcher: "New Organization"        rendered (no gate)
        +--> 8-step wizard, 14 form controls, draft persisted
        +--> POST /tenants
                 |
                 v
             403 Admin role required  -> toast "Failed to create tenant"
```

Cross-ref: [LAW-181-3](01-first-principles.md) · [F-181-02](04-findings.md) ·
[EC-181-06](12-edge-cases.md) · [W-01](09-ux-ui-spec.md#w-01-header-switcher)

---

## Chain C — "Sixteen steps to create an empty workspace"

| # | Question | Answer |
|---|----------|--------|
| 1 | Why does creating a workspace take two dialogs and sixteen steps? | `stepsForWizard('create-tenant')` yields `tenant-basics → models → document-parsing → workspace-basics → extraction → chunking → extract-budget → review`; `create-workspace` adds `retrieval`, giving 8. |
| 2 | Why are those steps mandatory? | The wizard renders every step the state machine produces; there is no "inherit everything" path that short-circuits to Review. |
| 3 | Why is that wrong for the common case? | The tenant already carries `default_llm_model`, `default_embedding_*`, `pdf_parser_backend`, `extraction_language`. A workspace that sets none of them inherits all of them, by design (LAW-123-8: omitted fields stay inherited). So the correct answer to 6 of the 8 steps is "inherit", the default. |
| 4 | Why does the operator not know that? | The inherited value is not printed on the step; `useInheritedModelDefaults` is only consulted by the eligibility gate. |
| 5 | **Root cause** | **The wizard models configuration as a required questionnaire, not as a short decision path with an explicit inherit default.** |

```text
  stepsForWizard('create-tenant')                       needed to create?
  --------------------------------------------------    ------------------
  1 tenant-basics      name, description                YES (name)
  2 models             LLM + embedding + vision         no   (tenant default)
  3 document-parsing   PDF backend + vision prompts     no   (tenant default)
  4 workspace-basics   name, slug                       YES (name)
  5 chunking           mode, size, overlap              no   (inherit)
  6 extract-budget     entities, records                no   (inherit)
  7 extraction         language + KG schema             no   (tenant default)
  8 review                                                -
```

`create-workspace` is the same eight with `retrieval` inserted after the name, so both flows are
eight screens regardless of how little the operator chooses.

Cross-ref: [LAW-181-4](01-first-principles.md) · [F-181-03](04-findings.md) ·
[EC-181-11](12-edge-cases.md) · [W-03](09-ux-ui-spec.md#w-03-create-workspace)

---

## Chain D — "Deleting an organization leaves the data behind"

| # | Question | Answer |
|---|----------|--------|
| 1 | Why does deleting an organization not reclaim storage? | `pg_delete_tenant` runs three `DELETE`s: `workspaces`, `memberships`, `tenants`. |
| 2 | Why is that not enough? | Each workspace owns a physical vector table (`drop_workspace_table`), an AGE graph, KV keys (`wsdoc:`, chunk prefixes), `pdf_documents` rows, `document_originals` (MinIO when `EDGEQUAKE_ORIGINAL_STORE=object`), the decision cache and tasks. `delete_workspace` handles all of them; `delete_tenant` calls none of it. |
| 3 | Why does the cascade not save us? | `ON DELETE CASCADE` removes rows, not objects in an object store, not `ag_catalog` graphs, not dynamically created vector tables. |
| 4 | Why does nobody notice? | The API returns 204 and the UI navigates away. The leak is only visible in `SELECT * FROM pg_tables WHERE tablename LIKE 'eq_%_vectors'` and in the bucket. |
| 5 | **Root cause** | **Two deletion paths exist for one concept (scope), and only the narrower one is complete.** SRP is not the issue; DRY is.** |

```text
  DELETE /api/v1/workspaces/{id}                DELETE /api/v1/tenants/{id}
  ------------------------------                ---------------------------
  tasks            cancelled                    (nothing)
  vector table     dropped                      rows only
  AGE graph        cleared                      rows only
  KV wsdoc:        deleted                      rows only
  pdf_documents    deleted                      rows only
  originals        deleted (object mode)        rows only
  decision cache   purged                       rows only
  workspace row    deleted                      rows deleted
```

Cross-ref: [LAW-181-7](01-first-principles.md) · [F-181-04](04-findings.md) ·
[EC-181-24](12-edge-cases.md) · [16-threat-model](16-threat-model.md#t-03-orphan-data)

---

## Chain E — "Two role axes are shipped; the third is invisible"

| # | Question | Answer |
|---|----------|--------|
| 1 | Why can nobody grant "Admin of Legal only"? | `memberships.workspace_id` expresses exactly that, `Membership::applies_to` honours it, and `me/settings-access` reports `administered_workspace_ids` from it. |
| 2 | Why is it unusable? | `POST /tenants/{t}/invitations` accepts `workspace_id`, but `InviteDialog` never sends it. `PATCH /tenants/{t}/members/{u}` accepts only `role`. There is no route that moves a grant between scopes. |
| 3 | Why does the UI show one role per person? | `MemberView` projects `primary_membership` — *the tenant-wide grant, else the highest workspace grant*. Two rows collapse into one cell. |
| 4 | Why does that block the whole SaaS shape? | Vercel, GitHub and Slack all separate organization membership from per-project/workspace access. Without it, the only way to give someone one workspace is to make them an organization Admin. |
| 5 | **Root cause** | **The workspace axis exists in storage and in enforcement, but has no write path and no read projection.** |

```text
  memberships (user, tenant, workspace, role)
  ------------------------------------------------
  (ada, acme, NULL,     member)   -> "Member"         primary_membership picks this
  (ada, acme, legal,    admin)    -> hidden in UI     second row, never rendered
  (ada, acme, finance,  readonly) -> hidden in UI     third row, never rendered
        |
        v
  UI shows:  Ada  |  Member  |  -        (the only role an admin can change)
  Reality:   Ada may administer Legal and read Finance
```

Cross-ref: [LAW-181-10](01-first-principles.md) · [F-181-05](04-findings.md) ·
[EC-181-19](12-edge-cases.md) · [W-09](09-ux-ui-spec.md#w-09-grant-editor)

---

## Chain F — "Quota is a fiction in three places"

| # | Question | Answer |
|---|----------|--------|
| 1 | Why does the dialog not say how many workspaces are left? | `CreateWorkspaceApiRequest` carries no count; `TenantResponse` exposes `max_workspaces` but not usage. |
| 2 | Why can two creations race past the limit? | The check is `SELECT COUNT(*)` then a separate `INSERT` in `pg_create_workspace`. No lock, no constraint. |
| 3 | Why is the number wrong on a fresh organization? | `Tenant::new` hard-codes `max_workspaces: 5, max_users: 10`, while `TenantPlan::Free` says `10` / `3`. `with_plan` overwrites it only when a plan is sent. |
| 4 | Why does the operator's configured default not apply? | `get_server_default_max_workspaces` is read by the admin UI and by nothing in the create path. |
| 5 | **Root cause** | **The quota has four sources of truth and no single enforcement point.** |

```text
  source                         value  used by
  ----------------------------   -----  --------------------------------------
  Tenant::new()                  5/10   every tenant created without a plan
  TenantPlan::Free               10/3   only after with_plan()
  metadata.max_workspaces        any    the INSERT check, the admin UI
  server default (config)        100    the admin GET, never the create path
```

Cross-ref: [LAW-181-6](01-first-principles.md) · [F-181-06](04-findings.md) ·
[EC-181-16](12-edge-cases.md) · [W-04](09-ux-ui-spec.md#w-04-quota-strip)

---

## Causal stack

```text
  LAW-181-1  Three axes, one derived catalog
       |
       +--> LAW-181-2  Creation is a tenancy transaction      --> closes A
       +--> LAW-181-3  Offer only what the server accepts     --> closes B
       +--> LAW-181-4  Short path, explicit inherit           --> closes C
       +--> LAW-181-7  One purge for every scope              --> closes D
       +--> LAW-181-10 Workspace grants are first class       --> closes E
       +--> LAW-181-6  One quota, one atomic check            --> closes F
       |
  LAW-181-5   Refusals are reasons, not 403s
  LAW-181-8   Create is not upsert
  LAW-181-9   Every scope change is audited
  LAW-181-11  Navigation is scope-shaped
  LAW-181-12  Empty is a state, not an error
  LAW-181-13  Inheritance is printed
  LAW-181-14  The wizard never loses work
  LAW-181-15  One entry point per intent
  LAW-181-16  CI is proof: one gate per EC
```
