# 16 — Threat model

Parent: [README](README.md) · Prev: [15-cross-ref](15-cross-ref.md) ·
Next: [lenses](lenses/LENS-product-owner.md)

The assets are *scopes* (organizations and workspaces) and the data inside them. The threats
are the ways a person ends up with rights they were not granted, or a scope is left in a state
nobody can govern.

---

## T-01 Privilege escalation through the grant editor

**Attack.** A workspace Admin of Legal adds themselves as organization Owner, or grants
themselves `admin` on Finance.

**Path today.** Not possible through the UI (no grant editor exists), but
`POST /tenants/{t}/invitations` accepts any `role` and `workspace_id`; `createInvitation` calls
`check_assign(&actor, Readonly, role, false)` — the *current* role is faked as `Readonly`, so
only the requested role is checked against the actor's rank. An Admin therefore passes
`Admin` (equal rank). It does not currently pass `Owner` (rank 4 > 3), which is correct by
accident of the level comparison.

**After.** `check_assign` is called with the real current role **and** the target scope:
`check_assign(actor, current_in_scope, new_role, is_last_owner)`, plus an explicit
`CrossAxis` denial when the target scope is not one the actor manages. Gate:
`R-wsadmin-sibling-403`, `workspace_admin_cannot_escalate`.

---

## T-02 Slug collision into a foreign organization

**Attack.** A platform admin (or, with `allow_self_serve_org`, any user) posts a tenant with a
slug that already exists. Today `ON CONFLICT (slug) DO UPDATE SET name = tenants.name` returns
the **existing** tenant.

**Impact.** The caller believes they created an organization; they were handed someone else's.
A subsequent `POST /tenants/{id}/workspaces` from a platform admin writes into the customer's
organization.

**After.** Create is not upsert ([LAW-181-8](01-first-principles.md)); 409 with `existing_id`
only, and "Open it" is an explicit user action. Gate: `R-no-upsert`.

---

## T-03 Orphan data

**Attack surface.** Not an attacker in the classic sense: a deletion that claims success while
the data remains. It is a confidentiality risk (a later tenant can collide with a stale vector
table name and read another corpus) and a cost risk.

**Path today.** `DELETE /tenants` removes three tables and leaves vectors, graphs, KV, objects
([F-181-04](04-findings.md)).

**After.** One purge, nine planes, a receipt, and a sweep assertion in CI. The sweep is the
control: `orphan_sweep` runs against real PostgreSQL in
`orphan_sweep_finds_zero_after_org_delete`. Gate: `R-purge-*`.

**Residual.** A plane that is permanently failing (a bucket deleted out from under us) leaves
`status: partial` forever. The receipt makes it visible; the retry route makes it fixable. It
is never silently `done`.

---

## T-04 Quota bypass

**Attack.** Two concurrent creates exceed the plan; or a tenant is created with the struct
default (5) instead of the plan value (10), so the customer gets less than they paid for; or an
owner raises their own limit.

**After.**
- Race: `SELECT … FOR UPDATE` inside a `BEFORE INSERT` trigger.
- Wrong default: quota resolved once from `subscription_plans` and stored in `NOT NULL` columns.
- Self-raise: `max_workspaces` is not writable from `PUT /tenants`; only
  `PATCH /admin/tenants/{id}/quota`.

Gates: `R-quota-race`, `create_tenant_quota_matches_plan`, `R-tenant-list-scoped`.

---

## T-05 Unattributable change

**Attack.** A platform admin changes a customer's plan, deletes a workspace, or edits a grant,
and nothing records who did it.

**After.** Every scope mutation writes `audit_logs` with `actor_kind ∈ {member, platform_admin}`
and a `detail` naming the capability ([LAW-181-9](01-first-principles.md)). The support-access
banner ([W-07](09-ux-ui-spec.md#w-07-platform-admin--deployment--organizations)) makes the act
visible while it happens, not only afterwards. Gate: audit row assertions in
`e2e_spec181_tenancy.rs`.

---

## T-06 Enumeration across organizations

**Attack.** A member of Acme probes for Globex by id: `GET /tenants/{globex_id}`,
`GET /tenants/{globex_id}/members`, `GET /workspaces/{ws_id}`.

**After.**
- `GET /tenants/{id}` → `require_tenant_access(Read)` → 403 with `capability_required:organization.read`
  (a *named* capability, not "denied" — the caller already knows the id exists or they would
  not have sent one).
- `GET /tenants` is row-scoped ([EC-181-29](12-edge-cases.md)).
- A workspace id from another organization → 404 from `verify_workspace_tenant_access`
  (existing 404-not-403 rule; it must not confirm existence).
- `me/access` reports `organization: null` rather than 403, so a stale tab degrades calmly.

Gate: `R-tenant-list-scoped`, `E-deeplink-hidden`.

---

## T-07 Orphaned organization (governance denial of service)

**Attack.** The last owner leaves, is suspended, or is deleted; or an organization is created
with no owner (today, always). Nobody can then invite, configure, transfer or delete it. Only a
platform admin can recover, and only if they know the tenant id.

**After.** Four layers:
1. Transactional owner grant on create.
2. `check_assign` / `check_remove` `LastOwner` denial (existing).
3. Deferred DB trigger `organization_requires_owner` — bypassing the API does not help.
4. Migration backfill for organizations that already have no owner, logged.

Gates: `R-create-rollback`, `R-last-owner-trigger`, `R-migration-ownerless`.

---

## T-08 Cross-axis confusion

**Attack.** A workspace Admin of Legal sends `PATCH /tenants/{acme}/members/{self}` with
`{role:"admin"}` expecting to become an organization Admin.

**Why it fails today.** `require_tenant_access(Manage)` reads `get_user_role`, which filters
`workspace_id IS NULL`. Legal's Admin has no organization row → 403. Correct behaviour reached
through an opaque error.

**After.** The same denial, with `capability_required:organization.members.manage` and a UI
that never offered the control ([05 §6](05-authorization-model.md#6--right-attribution-in-the-ui)).
Gate: `R-wsadmin-sibling-403`.

---

## T-09 SSO role re-sync overwriting a deliberate grant

**Attack / failure.** An administrator demotes an SSO user to Viewer; the next IdP login
restores their mapped role.

**Before this pack.** Handled by `pin_manual_role` writing `metadata.source = manual`.
**After.** Preserved, and the grant editor shows `Managed by Keycloak · pinned on 3 Feb` so the
behaviour is visible. Unpinning is an explicit action. Gate: `R-sso-pin`.

---

## T-10 Model-policy bypass at creation

**Attack.** A caller sets a workspace model that the organization ceiling does not allow, then
reads other tenants' embeddings through a provider key they should not reach.

**Before this pack.** `ensure_workspace_choice` (SPEC-178) already judges every pair before
storage. Preserved verbatim — the tenancy service calls it, it does not re-implement it.

**After.** Same enforcement, plus the refusal reaches the dialog as `422 models_not_eligible`
next to the control instead of a toast. Gate: `E-model-not-eligible`.

---

## T-11 Data retention against the operator's wishes

**Scenario.** A customer asks for deletion. The operator runs `DELETE /tenants`. Today the
answer is 204 while 340 MB of originals and 18 vector tables remain — a GDPR exposure.

**After.** The receipt is the evidence: it names each plane and its count, is retained for 30
days, and is itself the audit row. A `partial` status is a support ticket, not silence.

---

## Risk register

| # | Threat | Likelihood | Impact | Residual after this pack |
|---|--------|-----------|--------|--------------------------|
| T-01 | escalation via grants | medium | high | **low** — axis + rank + property test |
| T-02 | slug into foreign org | medium (today: silent) | critical | **low** — 409 only |
| T-03 | orphan data | high (today: certain) | high | **low** — receipt + sweep test |
| T-04 | quota bypass | medium | medium | **low** — lock + NOT NULL |
| T-05 | unattributable change | high | medium | **low** — audit + banner |
| T-06 | enumeration | medium | medium | **low** — row-scoped + 404 rule |
| T-07 | ownerless org | certain (today) | high | **very low** — 4 layers |
| T-08 | axis confusion | medium | low | **low** — named capability |
| T-09 | SSO overwrite | medium | medium | **low** — pin + visible |
| T-10 | model-policy bypass | low | high | **unchanged/low** — SPEC-178 owns it |
| T-11 | retention failure | high (today) | critical | **low** — receipt as evidence |

The two that were *certain* (T-03, T-07) are the reason this pack exists as a correctness pack
before it is a UX pack.
