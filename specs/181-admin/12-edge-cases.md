# 12 — Edge cases (EC-181-01 … EC-181-48)

Parent: [README](README.md) · Prev: [11-design-system](11-design-system.md) ·
Next: [13-implementation-plan](13-implementation-plan.md)

Every row has: the scenario, the **designed** outcome, the mitigation, and the gate id used in
[14](14-e2e-test-matrix.md). No row says "handle gracefully" — either the behaviour is stated
or the case is out of scope and says so.

---

## A — Creation: ownership and transactionality

| Id | Scenario | Outcome | Mitigation | Gate |
|----|----------|---------|-----------|------|
| EC-181-01 | Platform admin creates an organization; the default-workspace insert fails | **Nothing is created.** 409 `workspace_creation_failed`, transaction rolled back, no orphan tenant | single transaction ([LAW-181-2](01-first-principles.md)) | `R-create-rollback` |
| EC-181-02 | Creator is the last platform admin and names a different owner | Creator still holds Owner until the invitee accepts; the org is never ownerless | interim owner grant + deferred DB trigger | `R-interim-owner` |
| EC-181-03 | Named owner email already has an account in another organization | Invitation is created; on accept the person gains a second organization grant, not a new user | `accept.rs` already links by email with a signed-in check | `R-cross-org-owner` |
| EC-181-04 | Named owner email is already an active member of **this** organization | 409 `grant_exists`; the UI offers "Make owner" instead of "Invite" | pre-check in the dialog (`GET /members?q=email`) | `E-owner-already-member` |
| EC-181-05 | Two platform admins create the same slug simultaneously | One 201, one 409 `tenant_slug_taken` with `existing_id` | unique index on `slug`; `ON CONFLICT` removed | `R-slug-race` |
| EC-181-06 | A member opens the create dialog by forcing `?create=organization` | Full-page refusal state, no form rendered | route guard + `can_create` check before render | `E-forced-create-member` |
| EC-181-07 | A workspace admin forces `?create=workspace` | Refusal state naming the missing capability | same | `E-forced-create-wsadmin` |
| EC-181-08 | Network drops after `POST /tenants` reaches the server | The client retries with the same `Idempotency-Key`; the server returns the same 201 body | `Idempotency-Key` honoured for 10 minutes on both create routes | `R-idempotent-create` |
| EC-181-09 | Caller posts an existing slug with a different display name | 409 `tenant_slug_taken`; **no** silent join into the foreign organization | create is not upsert ([LAW-181-8](01-first-principles.md)) | `R-no-upsert` |
| EC-181-10 | Slugs collide after sanitisation (`"A/B"` and `"A B"` both → `a-b`) | Second create gets 409 and the UI offers `a-b-2` | `unique_tenant_slug` probes with a numeric suffix | `R-slug-sanitize-collision` |
| EC-181-11 | Operator opens Tune later, changes 4 rows, then hits Create | All 4 are sent in the same body | drawer writes the same draft object the dialog submits | `E-drawer-edits-submitted` |
| EC-181-12 | Operator picks a model that the tenant ceiling disallows | 422 `models_not_eligible` next to the model control; nothing is created | SPEC-178 `ensure_workspace_choice` already enforces | `E-model-not-eligible` |
| EC-181-13 | `first-run-wizard` and the new dialog both mounted on an empty install | Only `first-run-wizard` mounts; the header switcher is hidden until a tenant exists | `TenantGuard` renders the empty state, not the header create row | `E-first-run-exclusive` |

## B — Quota

| Id | Scenario | Outcome | Mitigation | Gate |
|----|----------|---------|-----------|------|
| EC-181-14 | Workspace count equals the limit | Create disabled with the reason; API returns 409 `workspace_quota_reached` | check in the dialog + DB trigger | `R-quota-limit`, `E-quota-disabled` |
| EC-181-15 | Two creates race at limit−1 | Exactly one succeeds | `SELECT … FOR UPDATE` in the trigger | `R-quota-race` |
| EC-181-16 | Owner needs an 11th workspace at 10/10 | Archive flow frees a slot; restore is blocked while full | `archived_at` excluded from the count | `R-archive-frees-quota` |
| EC-181-17 | Tenant created before 226 has no `max_workspaces` column value | Migration backfills from metadata, then plan, then 5 | [07 §3.2](07-data-and-db-contract.md#32-real-columns-on-tenants) | `R-migration-backfill` |
| EC-181-18 | Plan lowered below current usage | Admin UI shows the overage in amber; no workspace is deleted | read-only display; explicit archive required | `E-plan-overage-visible` |

## C — Grants and roles

| Id | Scenario | Outcome | Mitigation | Gate |
|----|----------|---------|-----------|------|
| EC-181-19 | Two grants for one person, one org + one workspace | Editor shows both rows; `me/access` reports both | `MemberView.grants[]` | `R-two-grants`, `E-two-grant-rows` |
| EC-181-20 | Workspace admin requests grants for a sibling workspace | 403 `capability_required:workspace.members.manage`; sibling rows are not in the response at all | `grants` query filters by authorized scopes | `R-wsadmin-sibling-403` |
| EC-181-21 | Workspace admin tries `DELETE /workspaces/{w}` | 403 — delete now requires workspace **Owner** or org manager | capability 15 ([05 §3](05-authorization-model.md#3--the-capability-catalog)) | `R-wsadmin-cannot-delete` |
| EC-181-22 | Owner demotes themselves with no second owner | 409 `last_owner` + the promote hint | `check_remove` + deferred trigger | `R-last-owner`, `R-last-owner-trigger` |
| EC-181-23 | Platform admin demotes the last owner | Same 409; platform does **not** bypass the row | `check_assign` has no platform escape | `R-platform-cannot-orphan` |
| EC-181-24 | Grant targets a workspace of another tenant | 400 `grant_scope_mismatch` | FK + explicit scope check | `R-grant-scope-mismatch` |
| EC-181-25 | Grant already active for (user, tenant, workspace) | 409 `grant_exists`; UI offers Change role | unique index | `R-grant-exists` |
| EC-181-26 | SSO member's role is edited manually | Pinned (`source=manual`, `previous_source=sso`); next IdP login does not overwrite | `pg_assign_membership_role` already does this | `R-sso-pin` |
| EC-181-27 | Workspace-scoped grant is removed, person still has an org grant | Person keeps the implicit workspace access; the editor says so | `applies_to` inheritance, printed | `E-implicit-remains` |
| EC-181-28 | A member's only grant is removed while they are signed in | Next request 403 → the UI shows [W-02](09-ux-ui-spec.md#w-02-empty-state--no-organization) style "no access" and clears the selection | `me/access` returns `organization: null` | `E-grant-removed-live` |
| EC-181-29 | Single-tenant install (`strict_tenant_bind=false`) lists tenants to a member | **Fixed**: visibility is row-based, not env-based | `EDGEQUAKE_LEGACY_TENANT_LIST=1` opt-out for one release | `R-tenant-list-scoped` |
| EC-181-30 | Refusal message wording changes | Client still translates correctly | reason codes, not regexes ([LAW-181-5](01-first-principles.md)) | `R-reason-codes` |

## D — Deletion and purge

| Id | Scenario | Outcome | Mitigation | Gate |
|----|----------|---------|-----------|------|
| EC-181-31 | Org delete while an ingest is running | Task cancelled first; purge proceeds; no partial document | `purge_workspace_tasks` per workspace | `R-purge-during-ingest` |
| EC-181-32 | Org delete fails on the object store (S3 503) | `status: partial`; the org row is gone, the plane is retryable | receipt per plane | `R-purge-partial-retry` |
| EC-181-33 | Two deletes for the same org | Second → 409 `purge_in_progress` with the receipt | receipt uniqueness per scope | `R-purge-in-progress` |
| EC-181-34 | Delete retried after success | 404 (scope gone) — not a second receipt | receipt lookup by scope, `status=done` | `R-purge-idempotent` |
| EC-181-35 | Workspace archived then org deleted | Both the live and the archived workspace are purged | enumeration includes `archived_at IS NOT NULL` | `R-purge-archived-ws` |
| EC-181-36 | Org deleted while a member is browsing it | Their next request 404 → the UI clears the selection and shows "organization no longer exists" | `me/access` 404 handling | `E-org-deleted-under-user` |

## E — Navigation and state

| Id | Scenario | Outcome | Mitigation | Gate |
|----|----------|---------|-----------|------|
| EC-181-37 | Switch organization | Workspace list swaps without blanking; lands on the first workspace's dashboard | `workspacesStale` instead of `workspaces: []` | `E-switch-no-blank` |
| EC-181-38 | Switch workspace on `/settings/workspace/models` | Stays on Models for the new workspace | `sectionHref(section, slug)` already does it | `E-switch-keeps-section` |
| EC-181-39 | Selected workspace is deleted by someone else | Selection falls to the first visible workspace; toast names it | existing behaviour in `TenantGuard`, now with a reason | `E-selection-heals` |
| EC-181-40 | No workspace grants at all | [J-11](06-user-journeys.md#j-11-member-hits-a-wall-and-is-told-what-to-do) instructive state | role-aware guard | `E-no-workspace-wall` |
| EC-181-41 | Deep link `/w/{slug}/documents` to a workspace the caller cannot read | 404 (not 403) from the API; UI shows "not found or no access" with a switcher | existing 404-not-403 rule, extended to the guard | `E-deeplink-hidden` |
| EC-181-42 | Palette command for a capability the caller lost mid-session | Command absent after the `me-access` refetch | palette reads the same `destinationGroupsFor` | `E-palette-refetch` |
| EC-181-43 | 3 workspace grants, one archived | Archived section in the switcher, not mixed with live | `archived_at` split | `E-archived-group` |

## F — Migration and compatibility

| Id | Scenario | Outcome | Mitigation | Gate |
|----|----------|---------|-----------|------|
| EC-181-44 | Old binary runs against schema 226 | Works, except org creation, which the deferred owner trigger rejects | **W1 ships before 226** ([13 §ordering](13-implementation-plan.md#ordering)) | `R-migration-order` |
| EC-181-45 | Existing org with zero owners at migration time | Migration grants the bootstrap admin Owner and logs each repair | [07 §6](07-data-and-db-contract.md#6--rollback) | `R-migration-ownerless` |
| EC-181-46 | Client on the old WebUI builds calls `DELETE /tenants` expecting 204 | Gets 202; the old client ignores the body and navigates away — acceptable, purge continues server-side | documented in [08 §compat](08-api-contract.md#backwards-compatibility-ledger) | `R-delete-202-legacy` |
| EC-181-47 | `me/settings-access` still called by SPEC-175 shell | Returns the same shape, implemented over `me/access` | adapter, not a second query | `R-settings-access-adapter` |
| EC-181-48 | MCP `eq_workspace_list` in a multi-org account | Lists workspaces of the bound scope only | same route, now row-scoped | `R-mcp-scope` |

---

## Coverage check

| Group | Count | Critical | High | Medium |
|-------|-------|----------|------|--------|
| A creation | 13 | 3 | 6 | 4 |
| B quota | 5 | 1 | 3 | 1 |
| C grants | 12 | 2 | 6 | 4 |
| D deletion | 6 | 4 | 2 | 0 |
| E navigation | 7 | 0 | 3 | 4 |
| F migration | 5 | 2 | 2 | 1 |
| **Total** | **48** | **12** | **22** | **14** |

48 edge cases, 48 gates. [LAW-181-16](01-first-principles.md) makes the count a build gate:
`scripts/validate-cross-ref.py` fails when the `EC-181-*` set and the gate set differ.

---

## Deliberately out of scope

| Case | Why |
|------|-----|
| Two platform admins creating the same org name in different slugs | Allowed; names are not unique |
| Moving a workspace between organizations | Requires re-keying every vector, graph node and KV key. A separate spec if demanded. |
| Restoring a hard-deleted organization | The purge is deliberate. Only the receipt survives, for audit. |
| Role expiry / temporary grants | No product request; would need a scheduler. |
| Per-workspace plan quotas | The plan is per organization by design. |
