# 13 — Implementation plan (waves W0 … W8)

Parent: [README](README.md) · Prev: [12-edge-cases](12-edge-cases.md) ·
Next: [14-e2e-test-matrix](14-e2e-test-matrix.md)

Nine waves. Each is independently shippable and independently revertable. Each ends with a
green gate set, not with a demo. Ordering is a hard constraint in two places, both called out.

---

## Ordering (non-negotiable)

```text
  W0  cleanup          ─┐
  W1  ownership+quota  ─┼─> MUST land before migration 226 is applied
  W2  grants API       ─┘
  W3  me/access + nav
  W4  purge + migration 226
  W5  dialogs (frontend)
  W6  organization/deployment pages
  W7  grants UI
  W8  polish + gates
```

**Why W1 before 226.** Migration 226 installs a deferred trigger that refuses a live
organization with no owner. The current `create_tenant` never writes an owner. Applying 226
before W1 would break organization creation outright ([EC-181-44](12-edge-cases.md)).
The migration contains its own backfill for pre-existing ownerless organizations, so the pair
is safe in either order *for old data* — but not for *new* writes.

**Why W4 after W3.** The purge UI polls a receipt; the receipt route is registered in W4, the
polling surface in W6. W3 introduces `me/access` first because every later wave's UI reads it.

---

## W0 — Cleanup and the shared vocabulary (0.5 day)

| Task | File | Done when |
|------|------|-----------|
| Delete `edgequake-auth::tenant::TenantService` and its module | `edgequake-auth/src/{lib,tenant}.rs` | `cargo build --workspace` clean; no reference remains ([F-181-12](04-findings.md)) |
| Rename `edgequake-core::tenant_manager::TenantService` → `TenantRagRouter` | `tenant_manager.rs` + 6 call sites | names say what they do |
| Add `Scope`, `ScopeAxis`, `Grant` value objects | `edgequake-core/src/types/multitenancy/scope.rs` | unit-tested, `Copy` where possible |
| Move `generate_slug` out of two places into `services/tenancy/slug.rs` | `handlers/workspaces/helpers.rs`, `handlers/setup.rs` | one function, one test file |
| Add `reason_code` to `ApiError::Forbidden` serialization | `edgequake-api/src/error.rs` | `diagnostic_details` carries it; existing strings unchanged |
| Extend `GET /roles` with `axis` on every capability | `services/role_catalog.rs` | additive; snapshot diff is explicit |

**Gates:** `cargo test --workspace --lib` · `cargo clippy --workspace --all-targets -- -D warnings`.

---

## W1 — Ownership and quota (2 days)

Closes [F-181-01](04-findings.md), [F-181-06](04-findings.md), [F-181-07](04-findings.md),
[F-181-08](04-findings.md).

### W1.1 — `TenancyService` skeleton

```rust
// services/tenancy/mod.rs
pub trait TenancyStore: Send + Sync {
    async fn insert_tenant(&self, t: Tenant) -> Result<Tenant>;
    async fn insert_workspace(&self, t: Uuid, r: CreateWorkspaceRequest) -> Result<Workspace>;
    async fn insert_grant(&self, g: Grant) -> Result<Grant>;
    async fn tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>>;
    async fn workspace_count(&self, t: Uuid) -> Result<usize>;
    async fn member_count(&self, t: Uuid) -> Result<usize>;
}
```

`TenancyService<S: TenancyStore>` holds the policy; `PgTenancyStore` holds the SQL;
`InMemoryTenancyStore` makes the policy unit-testable without a database (DIP, SOLID ledger).

### W1.2 — Create organization, one transaction

```rust
pub async fn create_organization(&self, actor: Actor, body: CreateOrganization) -> Result<Created> {
    let quota = Quota::resolve(body.plan.as_deref(), self.plan_catalog()).await?;
    self.store.transaction(|tx| async {
        let tenant = tx.insert_tenant(Tenant::new(&body.name, slug).with_quota(quota))?;
        let owner  = tx.insert_grant(Grant::org(owner_id, tenant.id, Owner))?;
        let ws     = tx.insert_workspace(tenant.id, body.workspace)?;
        tx.insert_grant(Grant::workspace(owner_id, tenant.id, ws.id, Owner))?;
        tx.audit(...)?;
        Ok(Created { tenant, owner, ws })
    }).await
}
```

### W1.3 — Quota

`Quota::resolve` reads `subscription_plans` once. `assert_workspace_capacity` runs inside the
transaction. `TenantResponse` gains `workspaces_used` / `members_used` / `my_role`.

### W1.4 — Create is not upsert

Remove `ON CONFLICT (slug) DO UPDATE SET name = tenants.name` from `pg_create_tenant`. Map the
unique violation to `tenant_slug_taken` with `existing_id`.

### W1.5 — Reason codes

Every refusal in `tenant_access.rs`, `tenants.rs`, `workspace_crud.rs` returns
`capability_required:<id>` (or the specific code from [08](08-api-contract.md#reason-codes-the-complete-set-this-pack-introduces)).

**Gates**

```bash
cargo test -p edgequake-api --test e2e_spec181_tenancy
cargo test -p edgequake-core --lib tenancy
```

| Gate | Asserts |
|------|---------|
| `create_tenant_grants_owner` | `memberships` row exists, role owner, before the response |
| `create_tenant_rolls_back_on_workspace_failure` | injected store error → zero rows |
| `create_tenant_quota_matches_plan` | free → 10/3, pro → 500/50 |
| `create_tenant_twice_conflicts_with_existing_id` | 409 + `existing_id` |
| `concurrent_creates_respect_quota` | two tasks, one success |
| `every_refusal_has_a_reason_code` | route table walk |

---

## W2 — Grants (2.5 days)

Closes [F-181-05](04-findings.md).

| Task | Notes |
|------|-------|
| `handlers/members/grants.rs` — `list`, `post`, `patch`, `delete` | pure decisions in `guards.rs` |
| `guards.rs` becomes axis-aware: `Actor::{rank_in(scope), max_assignable(scope)}` | `Denial` gains `CrossAxis` |
| `MemberView.grants: Vec<GrantView>` | `primary_membership` kept for the `role` field's compatibility |
| `GET /members?workspace_id=` filter | one `WHERE` clause |
| Invitations validate `workspace_id` | `grant_scope_mismatch` |
| Accept writes the workspace-scoped row when the invitation has one | `accept.rs` already branches on `workspace_id` |

**Gates**

| Gate | Asserts |
|------|---------|
| `org_admin_grants_workspace_admin` | 201 + row + `me/access` reflects it |
| `workspace_admin_cannot_escalate` | 403 on Owner, on org scope, on sibling |
| `two_rows_one_person_two_grants` | projection lists both |
| `last_owner_trigger_refuses` | direct SQL `DELETE` raises `organization_requires_owner` |
| `sso_role_edit_pins` | metadata `source=manual` |

---

## W3 — `me/access` and navigation (2 days)

Closes [F-181-02](04-findings.md), [F-181-09](04-findings.md), [F-181-10](04-findings.md).

| Task | Notes |
|------|-------|
| `handlers/me_access.rs` | composes `scope_for` + `administered_workspace_ids` + quota; re-exports the SPEC-175 pure fns |
| `me/settings-access` becomes an adapter | same shape, same numbers |
| `lib/access/use-access.ts` + `capabilities.ts` | one hook, one query key, fail-closed |
| `destinations.ts` gains `scope` + `requires` | `destinationGroupsFor(access, caps)` |
| `header-tenant-selector.tsx` + `context-selector-popover.tsx` gated | create rows absent/disabled per [05 §6](05-authorization-model.md#6--right-attribution-in-the-ui) |
| `tenant-guard.tsx` five role-aware states | [W-02](09-ux-ui-spec.md#w-02-empty-state--no-organization) |
| `RequireOrganization` / `RequireOrganizationManager` / `RequirePlatform` | in `app/(dashboard)/layout.tsx` |
| `use-tenant-store.selectTenant` keeps the old workspace list | `workspacesStale` |

**Gates**

| Gate | Asserts |
|------|---------|
| `E-member-no-create` | member sees no create rows anywhere |
| `E-orgadmin-no-create-org` | org admin sees workspace create, not org create |
| `E-forced-create-member` | refusal state, no form |
| `E-sidebar-per-persona` | 4 snapshots |
| `E-palette-per-persona` | 4 lists |
| `R-access-fail-closed` | before the query resolves, no gated element exists |

---

## W4 — Purge and migration 226 (2.5 days)

Closes [F-181-04](04-findings.md).

| Task | Notes |
|------|-------|
| `services/tenancy/purge.rs` | `ScopePurge` trait; `purge_workspace` is today's `delete_workspace` body moved, not rewritten |
| `purge_organization` | enumerates workspaces (live + archived) → `purge_workspace` each → org rows |
| `scope_purge_receipts` writes with per-plane status | idempotent retry |
| `DELETE /tenants/{id}` → 202 + receipt | `GET …/purge-receipts/{id}`, `POST …/retry` |
| `POST /workspaces/{id}/archive` + `/restore` | `archived_at`, quota interaction |
| migration `226_spec181_admin.sql` | [07](07-data-and-db-contract.md) verbatim |
| manifest + `compat_serve_max = 226` | |

**Gates**

| Gate | Asserts |
|------|---------|
| `orphan_sweep_finds_zero_after_org_delete` | no `eq_*_ws_*` table, no KV prefix, no AGE graph, no object prefix |
| `purge_is_idempotent` | second call → 404, one receipt |
| `purge_partial_then_retry` | injected S3 failure → `partial` → retry → `done` |
| `R-migration-backfill` | pre-226 tenant keeps its effective limit |
| `R-migration-ownerless` | ownerless org repaired, logged |

---

## W5 — The two dialogs (3 days)

Closes [F-181-03](04-findings.md), [F-181-11](04-findings.md).

| Task | Notes |
|------|-------|
| `components/admin/scope-dialog.tsx` | a11y from `wizard-shell` without the stepper |
| `create-organization-dialog.tsx` | name + owner + models + Advanced |
| `create-workspace-dialog.tsx` | name + retrieval + models + Tune later |
| `advanced-drawer.tsx` | reuses `steps/*` field components |
| `inherit-row.tsx`, `quota-strip.tsx`, `owner-picker.tsx` | |
| `hooks/use-create-scope.ts` | one mutation per intent; `applyCreatedScope` |
| delete `create-{tenant,workspace}-wizard.tsx` | −567 lines |
| `wizard-state.ts`: create kinds → `['basics','review']` | first-run and reconfigure unchanged |

**Gates**

| Gate | Asserts |
|------|---------|
| `E-create-ws-three-decisions` | ≤ 3 labelled decisions before Create |
| `E-create-org-three-decisions` | same |
| `E-drawer-prints-inherited` | each row shows layer + value |
| `E-refusal-keeps-fields` | server 409 → fields intact, alert focused |
| `E-reload-restores-draft` | |
| `E-success-clears-draft` | |
| `R-create-two-requests` | exactly one POST per create (no PATCH follow-up) |

---

## W6 — Organization and deployment pages (2.5 days)

| Task | Notes |
|------|-------|
| `app/(dashboard)/organization/page.tsx` | [W-06](09-ux-ui-spec.md#w-06-organization-page-organization-owner) |
| `app/(dashboard)/organization/members/page.tsx` | re-hosts `MembersPage` |
| `settings/sections/organizations-section.tsx` | [W-07](09-ux-ui-spec.md#w-07-platform-admin--deployment--organizations), replaces `AdminQuotaSection` |
| `settings/sections/workspaces-section.tsx` | org-scoped workspace list |
| `delete-scope-dialog.tsx` + progress strip | [W-08](09-ux-ui-spec.md#w-08-delete-organization--typed-confirmation) |
| registry entries `organizations`, `workspaces` | scope + gate |

---

## W7 — Grants UI (2 days)

| Task | Notes |
|------|-------|
| `grant-editor.tsx`, `scope-badge.tsx` | [W-09](09-ux-ui-spec.md#w-09-grant-editor) |
| `member-row.tsx` renders n grants + `+n more` | [W-11](09-ux-ui-spec.md#w-11-members--one-person-many-grants) |
| `members-table.tsx` SCOPE column + filter | |
| `invite-dialog.tsx` scope selector | [W-10](09-ux-ui-spec.md#w-10-invite--scope-first) |
| `roles-tab.tsx` axis-aware matrix | `GET /roles` `axis` |
| delete `ROLE_LEVEL` from `role-meta.ts` | levels come from the server |

---

## W8 — Polish and the full gate set (1.5 days)

| Task | Notes |
|------|-------|
| `prefers-reduced-motion`, focus rings, `aria-live` regions | [09 §interaction](09-ux-ui-spec.md#interaction-contract) |
| CLS 0 on switcher + dialogs | measured |
| Screenshots light/dark × 390/768/1440 per screen | [14 §visual](14-e2e-test-matrix.md#visual-gates) |
| `locales/{en,fr,zh}.json` parity | all new keys |
| OpenAPI refresh + `spec027_api_contract` | |
| `scripts/validate-cross-ref.py` in CI | EC ↔ gate parity |
| docs: `docs/operations/admin-and-roles.md` | operator-facing, one page |

---

## Estimate and risk

| Wave | Days | Risk | Mitigation |
|------|------|------|-----------|
| W0 | 0.5 | low | mechanical |
| W1 | 2.0 | **medium** — touches the hottest handler | policy in a service, unit-tested with a fake store |
| W2 | 2.5 | **medium** — guard semantics | pure functions first, HTTP after |
| W3 | 2.0 | low | additive |
| W4 | 2.5 | **high** — destructive | receipt + idempotency + a live orphan sweep test |
| W5 | 3.0 | medium | reuses existing field components |
| W6 | 2.5 | low | new pages |
| W7 | 2.0 | medium | per-grant decisions |
| W8 | 1.5 | low | |
| **Total** | **18.5** | | |

### The three risks worth naming

1. **W4 can destroy customer data.** Mitigation: the purge is the *existing* workspace purge,
   called per workspace, with a receipt; nothing new deletes anything. The only new code
   enumerates scopes. `orphan_sweep_finds_zero_after_org_delete` runs against a real PostgreSQL
   in CI, not a mock.
2. **W2 changes who can do what.** `workspace.delete` narrowing to Owner is a behaviour change
   ([EC-181-21](12-edge-cases.md)). Mitigation: announce in CHANGELOG, and the API returns a
   reason code the old UI can still display.
3. **W5 deletes two wizards other specs' tests drive.** `e2e/spec169`, `spec180` and the
   first-run specs reference the wizard test ids. Mitigation: W5 keeps `first-run-wizard`
   untouched and updates the two specs that target the create dialogs in the same PR.

---

## Definition of done

```text
[ ] cargo fmt --check
[ ] cargo clippy --workspace --all-targets -- -D warnings
[ ] cargo test --workspace --lib
[ ] cargo test -p edgequake-api --test e2e_spec181_tenancy
[ ] cargo test -p edgequake-api --test spec181_catalog_matches_enforcement
[ ] cargo test -p edgequake-api --test spec027_api_contract
[ ] make bench001-doctor && make bench        (Acc unchanged: no extraction path touched)
[ ] cd edgequake_webui && npx playwright test e2e/spec181 --project=mock-api
[ ] scripts/validate-cross-ref.py specs/181-admin
[ ] screenshots reviewed at 390 / 768 / 1440, light and dark
[ ] CHANGELOG entry names the four behaviour changes:
      POST /tenants no longer upserts
      DELETE /tenants returns 202 + receipt
      GET /tenants visibility is row-scoped
      workspace delete requires Owner
```

The Acc benchmark is in the list because [SPEC-001](../001-benchmark/) pins the release score,
and this pack touches ingest-adjacent code (`create_workspace` defaults). It should not move
the number; the gate proves it.
