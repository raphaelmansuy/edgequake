# SPEC-181 — Admin: creating organizations and workspaces, and who may do what

> **Status:** Proposed. Nothing implemented yet.
> **Product pin:** EdgeQuake v0.38.3. Migration file head is
> [225_spec180_normal_rag.sql](../../edgequake/migrations/225_spec180_normal_rag.sql).
> This pack reserves migration **226**.
> **Scope:** Two creation flows (a Tenant, called *organization* in the UI; a Workspace),
> the navigation that reaches them, and the RBAC × ABAC model that decides who may run
> them. Every screen is re-cut to the calm, dense, warm-paper language of
> `/Users/raphaelmansuy/Github/03-working/gr8r-studio`.
> **Inherits:** [SPEC-101](../101-wizard-mode-tenant-workspace/) wizard engine ·
> [SPEC-168](../168-perfect-ui/) UI bar and keyboard contract ·
> [SPEC-169](../169-perfect-user-management/) members, invitations, two role axes ·
> [SPEC-175](../175-perfect-settings/) settings scopes and `/me/settings-access` ·
> [SPEC-174](../174-document-security/) clearance ABAC (object axis) ·
> [SPEC-158](../158-entreprise-grade-authentication/) SSO provisioning ·
> [SPEC-150](../150-reliable-migration-system/) migration manifest ·
> [SPEC-180](../180-normal-rag/) immutable workspace profile ·
> [SPEC-178](../178-provider-and-model-choice/) model eligibility.
> **Peers:** [SPEC-174](../174-document-security/) owns *which document* a reader sees.
> This pack owns *who may create, configure and destroy a scope*, and never re-implements
> the clearance algebra.

## The one-sentence job

> A platform admin, an organization owner and a workspace admin each open the product,
> see exactly the two create actions their rights allow, finish the one they started in
> under a minute, and end with a working scope whose ownership, quota and navigation are
> correct — or a clear, actionable refusal before they type anything.

## Start here

1. [00-why.md](00-why.md) — Five WHY chains, each with the causal ASCII
2. [01-first-principles.md](01-first-principles.md) — LAW-181-1…16, DRY / SOLID ledger
3. [02-surfaces.md](02-surfaces.md) — Every file and symbol this pack touches
4. [03-product-spec.md](03-product-spec.md) — Personas, stories, acceptance criteria
5. [04-findings.md](04-findings.md) — F-181-01…12, with file and symbol
6. [05-authorization-model.md](05-authorization-model.md) — The three axes, the algebra, the capability catalog
7. [06-user-journeys.md](06-user-journeys.md) — J-01…J-12, all twelve workflows
8. [07-data-and-db-contract.md](07-data-and-db-contract.md) — Migration 226, ER diagram
9. [08-api-contract.md](08-api-contract.md) — Routes, bodies, reason codes, OpenAPI delta
10. [09-ux-ui-spec.md](09-ux-ui-spec.md) — ASCII wireframes W-01…W-14 and navigation trees
11. [10-frontend-architecture.md](10-frontend-architecture.md) — One access hook, one wizard engine
12. [11-design-system.md](11-design-system.md) — What we take from gr8r-studio, token by token
13. [12-edge-cases.md](12-edge-cases.md) — EC-181-01…48 register
14. [13-implementation-plan.md](13-implementation-plan.md) — Waves W0…W8, DRY/SOLID gates
15. [14-e2e-test-matrix.md](14-e2e-test-matrix.md) — One gate per EC, hermetic and live
16. [15-cross-ref.md](15-cross-ref.md) — Law × finding × EC × wave × test × lens
17. [16-threat-model.md](16-threat-model.md) — Escalation, quota bypass, orphan data
18. Lenses → [`lenses/`](lenses/)
    - [Product Owner](lenses/LENS-product-owner.md)
    - [Full Stack](lenses/LENS-full-stack.md)
    - [Database](lenses/LENS-database.md)
    - [UX / UI](lenses/LENS-ux-ui.md)
    - [Front](lenses/LENS-front.md)
    - [AI Engineer](lenses/LENS-ai-engineer.md)
    - [Design System](lenses/LENS-design-system.md)
19. Validation → [`scripts/validate-cross-ref.py`](scripts/validate-cross-ref.py)
20. Test harness → [`e2e/README.md`](e2e/README.md)

## Locked decisions (Wave 0)

1. **Three axes, never conflated.** `Platform` (instance JWT `Role`), `Organization`
   (`memberships` with `workspace_id IS NULL`), `Workspace` (`memberships` with a
   `workspace_id`). A capability names one axis and a minimum role. The catalog is
   derived from the same predicates the handlers call, so it cannot drift.
   [LAW-181-1](01-first-principles.md).
2. **The creator owns what they create.** Organization creation inserts the tenant, the
   creator's tenant-wide `owner` membership and the default workspace in **one**
   transaction. Today it inserts neither the membership nor a reliable workspace.
   [F-181-01](04-findings.md).
3. **A workspace-scoped grant becomes reachable.** Memberships already carry
   `workspace_id`; the UI and the API cannot currently create or edit one. A person may
   hold *organization Member + workspace Admin* at once, as two rows. This is the
   Slack/Vercel model. [LAW-181-10](01-first-principles.md).
4. **Creation is short.** Organization: 3 decisions (name, owner, defaults) plus an
   optional Advanced drawer. Workspace: 3 decisions (name, retrieval type, models).
   Everything the 8-step wizard asks today moves behind "Tune later" with the inherited
   value printed. [LAW-181-4](01-first-principles.md).
5. **Offer only what the server will accept.** A create action renders iff
   `me/access` grants the capability **and** quota remains. Otherwise it is absent, with
   the reason carried by the surface that would have hosted it.
   [LAW-181-3](01-first-principles.md).
6. **Quota is visible and atomic.** `used / limit` sits in the creation dialog and in the
   switcher. The limit is checked inside the insert transaction, not by a
   read-then-write in a different handler. [LAW-181-6](01-first-principles.md).
7. **Create is not upsert.** `POST /tenants` returns 409 `tenant_slug_taken` with the
   existing id. Get-or-create stays only in `POST /setup/initialize`.
   [LAW-181-8](01-first-principles.md).
8. **Delete purges every plane.** The same `ScopePurge` service deletes a workspace and an
   organization: relational rows, vector table, AGE graph, KV keys, PDF rows, object
   originals, tasks, caches — with a receipt row so a retry is idempotent.
   [LAW-181-7](01-first-principles.md).
9. **Visual language is gr8r-studio's**, mapped onto the existing shadcn tokens: warm
   paper background, hairline borders, 13.5px body, 220px scope nav, label/description/
   control rows, segmented controls, pill badges. No new component library.
   [11-design-system](11-design-system.md).
10. **One entry point per intent.** Four surfaces (header switcher, settings, empty state,
    command palette) call the same two dialogs. [LAW-181-15](01-first-principles.md).

## Job in one screen

```text
   WHO                     WHERE THEY START              WHAT THEY MAY CREATE
   ----------------------  ----------------------------  ------------------------------
   Platform Admin          Deployment > Organizations    Organization (any), Workspace
   (JWT Role::Admin)       or header switcher            (any org), delete either

   Super Admin             Organization > People         Workspace (this org)
   (tenant-wide owner)     or header switcher            members, grants, brand
   (tenant-wide admin)                                   NOT an organization

   Workspace Admin         Workspace > Members           members of that workspace
   (workspace owner/admin)                               NOT a workspace, NOT a sibling

   Member / Viewer         no create affordance          nothing
   ----------------------  ----------------------------  ------------------------------
```

```text
  header switcher ─┐
  empty state ─────┼──> <CreateOrganizationDialog/>  ──> POST /tenants ──> tenant
  palette ─────────┤                                         │            + owner grant
  settings ────────┘                                         │            + default ws
                                                             v
                                              <CreateWorkspaceDialog/> ──> POST /tenants/{t}/workspaces
```

## What this pack will **not** do

- Re-open the clearance algebra of [SPEC-174](../174-document-security/). Tags and
  classification stay the object axis; this pack only points at them from the grant
  editor.
- Ship a SCIM server. Provisioning stays [SPEC-158](../158-entreprise-grade-authentication/)
  JIT + role re-sync.
- Add billing. `plan` becomes a real column with a quota, not a payment integration.
- Replace [SPEC-175](../175-perfect-settings/)'s settings shell. This pack adds two
  scopes' landing pages into it and one new Organization section.
