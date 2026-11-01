# 03 — Product spec

Parent: [README](README.md) · Prev: [02-surfaces](02-surfaces.md) ·
Next: [04-findings](04-findings.md)

## Personas

| Id | Persona | Role axes | What they need from this pack |
|----|---------|-----------|-------------------------------|
| P-01 | **Dana, platform operator.** Runs EdgeQuake for six customers. | Platform Admin | Create an organization for a customer in under a minute, be sure it starts clean, and delete a trial without leaking storage. |
| P-02 | **Marc, organization owner.** Buys EdgeQuake for his company, 40 people, three departments. | Org Owner | Invite the department leads as workspace admins — not as org admins. Rename the org, set the brand color. |
| P-03 | **Aïcha, workspace admin.** Runs the Legal knowledge base. | Org Member + Workspace Admin (Legal) | Invite the two paralegals to Legal only. She must not see Finance, and must not be able to create a workspace. |
| P-04 | **Tom, member.** Uploads documents, asks questions. | Org Member | Never see a create button he cannot use. Never see an organization setting. |
| P-05 | **Ruth, viewer.** Reads the knowledge base (external auditor, read-only seat). | Org Viewer | See the org and the workspaces she was granted, read only. |

The four axes are exactly the four in [01-first-principles § LAW-181-1](01-first-principles.md#law-181-1--three-axes-one-derived-catalog).

## Jobs to be done

```text
  WHEN                                        I WANT TO                                    SO THAT
  ------------------------------------------  -------------------------------------------  -----------------------------------
  a customer signs                          create their organization with me as owner   they can invite their team today
  I run a new department                     give its lead admin rights to that workspace only  nobody gains access to another dept
  I am a workspace admin                     invite a colleague to my workspace            they can work without org-wide access
  I hit the plan limit                       be told before I fill the form                I don't waste five minutes
  I created a trial by mistake               delete it                                     no vectors, no objects, no rows remain
  I have five workspaces                     switch between them without losing context    I keep my place in the product
```

## Stories, with acceptance criteria

### US-181-01 — Platform admin creates an organization (P-01)

> As a platform admin I create an organization and immediately hold its Owner grant.

- The header switcher, the deployment Organizations list and the command palette all open
  **one** dialog.
- The dialog asks for **Name** and shows **Owner: you** with an option to pick another member
  or invite by email.
- Submitting returns 201 and the dialog is replaced by a confirmation card naming the
  organization, the owner, the default workspace, and the next action ("Invite people" /
  "Upload documents").
- The creator's `memberships` row exists before the response is sent.
- **Gate:** `spec181/create-organization.spec.ts › platform admin creates organization`
  · Rust `create_tenant_grants_owner`.

### US-181-02 — Organization owner creates a workspace (P-02)

> As an organization owner I create a workspace for a new department.

- The dialog asks **Name**, **Retrieval type** (Graph / Normal) and **Models**
  (default: inherit tenant defaults, printed).
- A "Tune later" drawer holds parsing, chunking, extract budget, entity types, languages and
  the decision engine — each row printing its inherited value.
- Finish creates the workspace and selects it, and the URL becomes `/w/{slug}`.
- The workspace appears in the switcher without a full reload.
- **Gate:** `create-workspace.spec.ts › owner creates a graph workspace in three decisions`.

### US-181-03 — Workspace admin invites to their workspace only (P-03)

> As the admin of Legal I invite a colleague to Legal.

- Aïcha opens **Legal → Members**. The page lists only people with a grant on Legal, plus a
  banner explaining that organization-wide membership is managed by the org owner.
- "Invite" sends `workspace_id`; the invitation, when accepted, produces a
  `memberships(…, workspace_id=legal, role=member)` row — not an org-wide grant.
- Aïcha cannot pick a role above `admin`, cannot invite to Finance, and does not see
  `Create workspace`.
- **Gate:** `grants.spec.ts › workspace admin invites to own workspace`.

### US-181-04 — Organization owner grants Legal-Admin without org-admin (P-02)

> As the org owner I give Aïcha admin on Legal only.

- The grant editor lists Aïcha's current grants: `Organization · Member`.
- Adding a grant offers scope = a workspace of this org, role ≤ the owner's own rank.
- Two rows now exist for one person; the members table shows `Organization · Member` and
  `Legal · Admin` (collapsed as `+1`).
- Aïcha's next session reports `administered_workspace_ids = [legal]`.
- **Gate:** `grants.spec.ts › two grants render as two rows`,
  Rust `org_admin_grants_workspace_admin`.

### US-181-05 — A member is never offered an action the API refuses (P-04)

> As a member I see no create affordance.

- Header switcher: no "New organization", no "New workspace"; instead a quiet line
  `Ask an admin to create a workspace`.
- Command palette: the two commands are absent.
- `/organization` and `/settings/deployment/*` redirect to the caller's landing page.
- Forcing the dialog open by URL (`?create=organization`) renders a refusal state, not a form.
- **Gate:** `access-gating.spec.ts › member sees no create affordance`.

### US-181-06 — Quota is visible and honest (P-02)

> As an owner I know how many workspaces I have left before I start.

- The create dialog header shows `3 of 10 workspaces used`.
- At the limit, the primary button is disabled with the reason
  `Your plan allows 10 workspaces. Archive one or ask your platform admin for more.`
- Two simultaneous creations produce one 201 and one 409 `workspace_quota_reached`.
- **Gate:** `create-workspace.spec.ts › quota blocks with reason`,
  Rust `concurrent_creates_respect_quota`.

### US-181-07 — Deleting an organization leaves nothing (P-01)

> As a platform admin I delete a trial and the storage is reclaimed.

- The confirm dialog requires typing the slug and lists what will be destroyed (workspace
  count, document count).
- The response is 202 with a receipts URL; the page polls until every plane reports `done`.
- After completion no row references the tenant, no `eq_*_ws_*` vector table remains, no
  object prefix remains, and the AGE graph is gone.
- **Gate:** Rust `orphan_sweep_finds_zero_after_org_delete`,
  `purge_is_idempotent`.

### US-181-08 — Switching context keeps my place (P-02)

> As an owner I switch from Legal to Finance and land on Finance's documents.

- Switching a workspace keeps the current section (`/w/legal/documents` →
  `/w/finance/documents`).
- Switching an organization lands on its first workspace's dashboard and never on a blank
  screen; the previous selection is not blanked before the new one arrives.
- **Gate:** `navigation.spec.ts › workspace switch keeps section`,
  `organization switch never blanks`.

### US-181-09 — A refused submit keeps my work (P-03, P-02)

> Whoever fills a form does not lose it.

- A refusal (quota, slug, capability) renders inline in the dialog, keeps every field, and
  offers the fixing action.
- Reloading the page restores the draft.
- Success clears the draft.
- **Gate:** `forms.spec.ts › refusal keeps fields`, `reload restores draft`.

### US-181-10 — The permission matrix cannot lie (every persona)

> The Roles & permissions tab is generated from enforcement.

- `GET /roles` publishes capabilities with `axis` and `min_role`.
- A property test asserts, for each capability, that the set of roles the catalog says hold
  it equals the set of roles the handler accepts.
- **Gate:** Rust `spec181_catalog_matches_enforcement`.

## Non-goals

- Billing, invoices, proration. `plan` selects a quota row and nothing else.
- SCIM. Provisioning is [SPEC-158](../158-entreprise-grade-authentication/) JIT.
- Per-document permission UI. That is [SPEC-174](../174-document-security/) and it is
  already shipped; this pack links to it from the grant editor.
- A new component library. The dialogs are built from `components/ui/*`.

## Success measures

| Measure | Today (estimated) | Target |
|---------|-------------------|--------|
| Clicks from "New workspace" to created workspace (inherit everything) | 22 | ≤ 6 |
| Time to first workspace for a new organization | 3–5 min | < 45 s |
| Create actions visible to a caller who will get 403 | 2 (org + workspace) | 0 |
| Organizations that exist with zero Owner grants | every one created via API | 0 |
| Planes checked by organization delete | 3 of 9 | 9 of 9 |
| Matrix rows that drift from enforcement | unknown (hand-written) | 0 (generated + property test) |
