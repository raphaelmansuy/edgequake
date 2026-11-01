# 01 — First principles (LAW-181)

Parent: [README](README.md) · Prev: [00-why](00-why.md) · Next: [02-surfaces](02-surfaces.md)

These sixteen laws are **non-negotiable**. A wave in the
[implementation plan](13-implementation-plan.md) may defer a feature. It may not ship code
that violates a law. Each law states its closure, its test, and the DRY/SOLID principle it
serves.

---

## The first principles, stated plainly

1. **A scope is a set of rows with an owner.** A tenant and a workspace are not records; they
   are *authorization scopes*. Creating one creates its grants, its quota accounting and its
   storage namespace together, or it creates nothing.
2. **A capability is a predicate, not a label.** "Can create a workspace" must be the same
   boolean the handler evaluates. Any second copy of the rule drifts.
3. **A refusal happens before the first keystroke, or it is a bug.** A caller must never
   invest work that the server will reject.
4. **Every default must be visible at the point of decision.** Inheriting is a choice; the
   UI must print what is inherited.
5. **Deleting a scope deletes every plane the scope wrote to.** Rows, vectors, graphs, keys,
   objects, caches. One implementation.
6. **Navigation is generated from the same catalog as the buttons.**
7. **Work is never lost silently.** A draft survives a reload; a failed submit preserves
   input; a destructive action is reversible for its retention window or is not offered.

---

## DRY / SOLID ledger

| Principle | Where it is violated today | What this pack does |
|-----------|---------------------------|---------------------|
| **DRY** | Two delete implementations (workspace complete, tenant incomplete) — [F-181-04](04-findings.md). | One `ScopePurge` trait with `purge_workspace` / `purge_organization`; the tenant path iterates workspaces through the same code. |
| **DRY** | Four quota sources (`Tenant::new`, `TenantPlan`, `metadata`, server default) — [F-181-06](04-findings.md). | `Quota` is a value object resolved **once** at tenant creation and stored in a column. |
| **DRY** | Role ranks in Rust (`guards.rs`), in TS (`role-meta.ts`), in SQL (`CASE role WHEN`). | `GET /roles` already publishes the matrix; `GET /me/access` publishes the grants. The UI reads both. No rank constant in TS. |
| **SRP** | `tenants.rs::create_tenant` does validation, model-policy checks, insert, workspace creation, logging. 230 lines. | `services/tenancy/create.rs` owns the transaction; the handler parses and maps. |
| **SRP** | `CreateTenantWizard` owns draft state, validation, two API calls, context switching, cache invalidation and toasts. | A `useCreateScope` mutation owns the API sequence; the dialog owns fields only. |
| **OCP** | A new plan means editing `TenantPlan` **and** the CHECK constraint **and** the admin UI. | Plans are rows in `subscription_plans`; the enum becomes a lookup. |
| **LSP** | `TenantService` exists twice: `edgequake_auth::tenant::TenantService` (unused, in-memory-ish, no caller) and `edgequake_core::tenant_manager::TenantService` (RAG tenant manager) with different meanings. | Neither is used by the API. The auth one is deleted; the core one is renamed `TenantRagRouter` to say what it does. |
| **ISP** | `WorkspaceService` is a ~40-method trait; the creation path needs 6 of them. | A narrow `TenancyStore` trait is what the creation service depends on. |
| **DIP** | The handler calls `state.workspace_service` directly, so tenancy policy is untestable without a database. | `TenancyService<S: TenancyStore>`; policy is unit-tested with an in-memory store. |

---

## LAW-181-1 — Three axes, one derived catalog

Every authorization decision names **exactly one** axis:

| Axis | Stored as | Wire id | Decides |
|------|-----------|---------|---------|
| `platform` | `users.role` (JWT `Role`) | `admin`, `user`, `readonly` | Deployment: connections, fleet, every organization |
| `organization` | `memberships.workspace_id IS NULL` | `owner`, `admin`, `member`, `readonly` | The org: brand, members, workspace creation |
| `workspace` | `memberships.workspace_id = X` | `owner`, `admin`, `member`, `readonly` | One workspace: content, models, labels |

A **capability** is `(id, axis, min_role, group)`. The catalog is built by evaluating the
same predicate functions the handlers call
(`MembershipRole::can_manage_workspaces`, `can_manage_users`, `can_delete_tenant`, and the
platform check). The UI renders the matrix from `GET /roles` plus `GET /me/access`; it
hard-codes no table.

- **Closes:** WHY-E · [F-181-05](04-findings.md)
- **Tests:** `catalog_matches_enforcement.rs` asserts every capability's holders equal the
  set the handler accepts (property test over the four roles × two axes).

## LAW-181-2 — Creating a scope is one tenancy transaction

`POST /tenants` runs, in a single transaction:

1. insert `tenants` (plan → quota resolved here, LAW-181-6),
2. insert `memberships(creator, tenant, NULL, owner)`,
3. insert the default workspace **with its own admin grant** for the creator,
4. insert the `audit_logs` row,

and commits. If any step fails, nothing exists. `POST /tenants/{t}/workspaces` inserts the
workspace plus, when the caller is not already an org manager, a `workspace/admin` grant for
the creator. The response body states the grants it created.

- **Closes:** WHY-A · [F-181-01](04-findings.md)
- **Tests:** `create_tenant_grants_owner`, `create_tenant_rolls_back_on_workspace_failure`,
  `create_workspace_grants_admin_to_non_manager`.

## LAW-181-3 — Offer only what the server will accept

A create action renders iff `access.can(capability)` and `quota.remaining > 0`. It is
**absent**, not disabled, when the caller lacks the role; it is **disabled with the reason
inline** when quota blocks it (the caller may act on quota; they cannot act on their role).

- **Closes:** WHY-B · [F-181-02](04-findings.md)
- **Tests:** `header_has_no_create_for_member`, `palette_omits_tenant_create_for_org_admin`,
  `create_workspace_disabled_at_quota_with_reason`.

## LAW-181-4 — Short path, explicit inherit, advanced drawer

Organization creation asks for **name** (required), **first owner** (default: you), and
**default models** (default: server defaults). Workspace creation asks for **name**,
**retrieval type**, and **models**. Every other decision is reachable from a single
"Tune later" drawer, and each drawer row prints the inherited value.

Maximum: 3 visible decisions in each dialog's main column.

- **Closes:** WHY-C · [F-181-03](04-findings.md)
- **Tests:** `create_workspace_three_decisions_to_finish`,
  `advanced_drawer_prints_inherited_value`.

## LAW-181-5 — Refusals are reasons, not 403s

A denial that the UI could have predicted is a bug. A denial the server must make returns
a **stable machine reason** the UI translates:

| Situation | Status | Reason code |
|-----------|--------|-------------|
| Caller lacks the capability | 403 | `capability_required:<id>` |
| Workspace quota reached | 409 | `workspace_quota_reached` |
| Seat quota reached | 409 | `seat_quota_reached` |
| Slug taken | 409 | `tenant_slug_taken` / `workspace_slug_taken` |
| Last organization owner | 409 | `last_owner` |
| Target scope inactive | 409 | `scope_inactive` |

- **Closes:** [F-181-07](04-findings.md)
- **Tests:** `every_refusal_has_a_reason_code` (route table walk).

## LAW-181-6 — One quota, one atomic check

`subscription_plans` is a table. `tenants.max_workspaces` / `tenants.max_users` are real
columns with `NOT NULL` defaults, resolved once from the plan at creation. The insert of a
workspace takes `SELECT ... FOR UPDATE` on the tenant row before counting. `TenantResponse`
carries `workspaces_used` and `members_used`.

- **Closes:** WHY-F · [F-181-06](04-findings.md)
- **Tests:** `concurrent_creates_respect_quota` (two tasks, one 409),
  `new_tenant_quota_matches_plan`.

## LAW-181-7 — One purge for every scope

`ScopePurge::purge(Scope::Workspace(id))` and `purge(Scope::Organization(id))` share one
implementation. Organization purge enumerates workspaces and calls the workspace purge for
each, then removes org-level rows. A `scope_purge_receipts` row records each plane's outcome,
so a retry re-runs only unfinished planes.

- **Closes:** WHY-D · [F-181-04](04-findings.md)
- **Tests:** `orphan_sweep_finds_zero_after_org_delete`,
  `purge_is_idempotent`, `purge_receipt_lists_every_plane`.

## LAW-181-8 — Create is not upsert

`POST /tenants` returns 201, or 409 `tenant_slug_taken` with the existing `tenant_id` in the
body so the UI can offer "Open it instead". The current `ON CONFLICT (slug) DO UPDATE SET
name = tenants.name` trick is removed. Get-or-create survives only in
`POST /setup/initialize`, where it is the product behaviour.

- **Closes:** [F-181-08](04-findings.md)
- **Tests:** `create_tenant_twice_conflicts_with_existing_id`.

## LAW-181-9 — Every scope change is audited

Create, rename, archive, restore, plan change, grant change, ownership transfer and delete
write an `audit_logs` row with `event_type ∈ {TenantAccess, WorkspaceAccess}` and a
`metadata.detail` naming the capability exercised. The delete receipt is the audit row.

- **Closes:** [16-threat-model](16-threat-model.md#t-05-unattributable-change)
- **Tests:** `grant_change_writes_audit_row`, `delete_writes_receipt_and_audit`.

## LAW-181-10 — Workspace grants are first class

A person may hold an organization grant and any number of workspace grants. The UI renders
them as rows in one grant editor, with the scope named. `MemberView` gains `grants: Grant[]`
in addition to the existing `role` (kept for compatibility as the *primary* grant).

Rules that stay from [SPEC-169](../169-perfect-user-management/):
- a manager may only assign at or below their own rank in that scope,
- the last organization Owner cannot be demoted or removed,
- an SSO-managed role is re-synced unless pinned.

New rule:
- an organization Admin may grant at or below `admin` **in a workspace of that organization**,
  even when that workspace grant is `admin` and their organization role is only `admin`.

- **Closes:** WHY-E · [F-181-05](04-findings.md)
- **Tests:** `org_admin_grants_workspace_admin`, `workspace_admin_cannot_escalate`,
  `two_rows_one_person_render_as_two_grants`.

## LAW-181-11 — Navigation is scope-shaped

The sidebar sections, the switcher groups and the command palette are all produced from one
`access` object. Deployment sections exist iff platform. Organization sections exist iff org
manager. Workspace sections exist iff a workspace is selected and the caller can read it.

- **Closes:** [F-181-09](04-findings.md)
- **Tests:** `sidebar_for_each_of_four_personas` (four snapshots).

## LAW-181-12 — Empty is a state, not an error

"No organizations yet", "no workspace selected", "you administer none" are distinct, drawn,
actionable states. None of them is an error boundary, a spinner that never resolves, or a
redirect loop.

- **Closes:** [F-181-10](04-findings.md)
- **Tests:** `empty_states_render_for_five_inputs`.

## LAW-181-13 — Inheritance is printed

Wherever a value can be inherited, the control shows `Inherited from <layer>: <value>` and a
link to that layer. Absence of a value is stated as `None`.

- **Closes:** WHY-C
- **Tests:** `every_inheritable_row_shows_its_layer`.

## LAW-181-14 — The wizard never loses work

Draft persists per `(kind, tenantId)` in `localStorage` (SPEC-101 already does this); on a
refused submit the dialog stays open with the server reason inline and the fields intact; on
a successful submit the draft is cleared; on close with a dirty draft the discard confirm
appears.

- **Closes:** WHY-B
- **Tests:** `refused_submit_keeps_fields`, `reload_restores_draft`, `success_clears_draft`.

## LAW-181-15 — One entry point per intent

Exactly one component creates an organization (`<CreateOrganizationDialog>`), one creates a
workspace (`<CreateWorkspaceDialog>`). The header switcher, the organization page, the
workspace page, the empty states and the command palette all open those two. No surface keeps
a private form.

- **Closes:** [F-181-11](04-findings.md)
- **Tests:** `create_organization_mounts_once` (grep + runtime assertion).

## LAW-181-16 — CI is proof: one gate per EC

Every `EC-181-*` in [12-edge-cases](12-edge-cases.md) names a test in
[14-e2e-test-matrix](14-e2e-test-matrix.md). A wave is not done until its gates are green in
`make test-e2e-full` and `cargo test -p edgequake-api`.

- **Closes:** the whole pack
- **Tests:** [`scripts/validate-cross-ref.py`](scripts/validate-cross-ref.py) fails the build
  when an EC has no gate.

---

## What is deliberately **not** a law

| Tempting rule | Why we refuse it |
|---------------|------------------|
| "Every scope must have at least one owner at all times" | The last-owner guard achieves it for live scopes; a soft-deleted org has none by definition, and forcing one would block deletion. |
| "Workspace roles inherit down to documents" | That is [SPEC-174](../174-document-security/): clearance is the object axis. Workspace role decides *what you may do*, clearance decides *what you may see*. |
| "The UI mirrors the whole permission matrix" | It mirrors `GET /roles`; a hard-coded matrix in TS is the drift we are removing. |
| "Creation must be idempotent" | Retry-on-network-error is a client concern (`Idempotency-Key` header), not a semantic change to POST. |
