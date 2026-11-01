# 14 — E2E test matrix

Parent: [README](README.md) · Prev: [13-implementation-plan](13-implementation-plan.md) ·
Next: [15-cross-ref](15-cross-ref.md)

Three layers, each with one job:

| Layer | Tool | Proves | Where |
|-------|------|--------|-------|
| **Policy** | `cargo test` unit | the decision is right, with no I/O | `edgequake-core/src/tenancy/*`, `members/guards.rs` |
| **Contract** | `cargo test --test` + PostgreSQL | the route, the transaction, the trigger | `edgequake/crates/edgequake-api/tests/` |
| **Experience** | Playwright, hermetic | the screen, the flow, the refusal, the screenshot | `edgequake_webui/e2e/spec181/` |

Every gate id from [12](12-edge-cases.md) appears exactly once in the tables below.
`R-` = Rust, `E-` = Playwright.

---

## Running

```bash
# Policy + contract (needs PostgreSQL: make postgres-start)
cargo test -p edgequake-core --lib tenancy
cargo test -p edgequake-api --test e2e_spec181_tenancy
cargo test -p edgequake-api --test spec181_catalog_matches_enforcement

# Experience (hermetic; no backend, no LLM)
cd edgequake_webui
SPEC181_ROUND=round-01 npx playwright test e2e/spec181 --project=mock-api
```

The Playwright suite follows the [SPEC-169](../../edgequake_webui/e2e/spec169/) pattern: a
stateful mock intercepts the tenancy/access routes so a create really creates, a 409 really
409s, and a screenshot is written per assertion group.

---

## Rust — policy (`edgequake-core`, no database)

| Gate | Test name | Asserts |
|------|-----------|---------|
| `R-quota-limit` | `quota::rejects_at_limit` | `assert_workspace_capacity(used == limit)` → `QuotaReached` |
| `R-quota-race` | `quota::reservation_is_serial` | two reservations on a fake store, one granted |
| `R-last-owner` | `guards::last_owner_cannot_be_demoted` | exists today, extended |
| `R-platform-cannot-orphan` | `guards::platform_admin_is_not_an_escape_hatch` | `check_assign(PLATFORM, Owner→Member, is_last)` → `LastOwner` |
| `R-wsadmin-cannot-delete` | `capabilities::workspace_delete_is_owner_only` | Admin holds `settings.manage`, not `delete` |
| `R-wsadmin-sibling-403` | `guards::cross_axis_is_denied` | workspace Admin on sibling → `CrossAxis` |
| `R-grant-scope-mismatch` | `grants::scope_must_belong_to_tenant` | |
| `R-sso-pin` | `membership::pin_manual_role` | exists today |
| `R-slug-sanitize-collision` | `slug::collision_gets_numeric_suffix` | `a-b` → `a-b-2` |
| `R-reason-codes` | `error::every_forbidden_has_reason` | route table walk |
| `R-idempotent-create` | `tenancy::same_key_returns_same_body` | |
| `R-access-fail-closed` | `access::no_rows_means_no_capabilities` | |

## Rust — contract (`edgequake-api`, PostgreSQL)

File: `crates/edgequake-api/tests/e2e_spec181_tenancy.rs`.

| Gate | Asserts |
|------|---------|
| `R-create-rollback` | injected `insert_workspace` failure ⇒ `SELECT count(*) FROM tenants` unchanged; no membership; no audit |
| `R-interim-owner` | create with `owner:{kind:email}` ⇒ creator is Owner, invitation row exists |
| `R-cross-org-owner` | invitee already has an account elsewhere ⇒ accept adds a membership, does not create a user |
| `R-slug-race` | two concurrent `POST /tenants` same slug ⇒ 201 + 409 with `existing_id` |
| `R-no-upsert` | `POST` twice, different name ⇒ 409, tenant count unchanged |
| `R-create-two-requests` | the created organization has exactly one workspace and one owner grant; no PATCH was issued |
| `R-quota-race` | two concurrent workspace creates at limit−1 ⇒ one 201, one 409 |
| `R-archive-frees-quota` | archive then create ⇒ 201 |
| `R-two-grants` | org grant + workspace grant ⇒ `GET /grants` returns 2 rows |
| `R-grant-exists` | second identical grant ⇒ 409 |
| `R-last-owner-trigger` | raw `DELETE FROM memberships` on the last owner ⇒ `organization_requires_owner` |
| `R-purge-during-ingest` | start ingest, delete org ⇒ task cancelled, purge completes, no orphan |
| `R-purge-partial-retry` | object-store failure ⇒ `partial`; retry ⇒ `done`; only failed planes re-run |
| `R-purge-in-progress` | second delete ⇒ 409 + `receipt_id` |
| `R-purge-idempotent` | delete after done ⇒ 404 |
| `R-purge-archived-ws` | archive a ws, delete org ⇒ that ws's vector table also gone |
| `R-tenant-list-scoped` | member of A calls `GET /tenants` ⇒ only A |
| `R-migration-backfill` | seed a pre-226 tenant with `metadata.max_workspaces = 42` ⇒ column is 42 |
| `R-migration-ownerless` | seed a tenant with no owner ⇒ migration adds the bootstrap admin, logs |
| `R-migration-order` | documented as a CI ordering assertion: W1 tests pass on schema 225 |
| `R-delete-202-legacy` | `DELETE` returns 202 and the body parses as `PurgeReceipt` |
| `R-settings-access-adapter` | `me/settings-access` and `me/access` agree on `tenant_manage` |
| `R-mcp-scope` | MCP `workspace_list` in a two-org account returns only bound scope |

### Orphan sweep (the important one)

```rust
async fn orphan_sweep(pool: &PgPool, tenant: Uuid) -> Vec<String> {
    let mut left = vec![];
    if row_exists(pool, "tenants", tenant).await      { left.push("tenants".into()); }
    if row_exists(pool, "workspaces", tenant).await   { left.push("workspaces".into()); }
    if row_exists(pool, "memberships", tenant).await  { left.push("memberships".into()); }
    for t in vector_tables_for_tenant(pool, tenant).await { left.push(t); }
    if kv_prefix_exists(pool, &format!("wsdoc:{tenant}")).await { left.push("kv".into()); }
    if age_graph_exists(pool, tenant).await           { left.push("age".into()); }
    left
}
```

`R-purge-during-ingest`, `R-purge-archived-ws` and `orphan_sweep_finds_zero_after_org_delete`
all end with `assert!(orphan_sweep(...).await.is_empty())`. That single assertion is the whole
of [LAW-181-7](01-first-principles.md).

## Rust — catalog

File: `crates/edgequake-api/tests/spec181_catalog_matches_enforcement.rs`.

| Gate | Asserts |
|------|---------|
| `catalog_equals_enforcement` | for each of 15 capabilities × 4 roles × 2 axes: `catalog.roles(cap).contains(role)` ⇔ the handler predicate returns `Ok` |
| `capability_ids_are_unique` | |
| `every_axis_is_drawn` | no capability omitted from the UI matrix |

This is the property test behind [LAW-181-1](01-first-principles.md). It fails the build the
moment someone adds a capability to the catalog that the handlers do not enforce, or tightens a
handler without the catalog.

---

## Playwright — experience

File: `edgequake_webui/e2e/spec181/`. Mock: `helpers/admin-mock.ts` (stateful; honours
`can_create`, quota, 409s).

### `create-organization.spec.ts`

| Gate | Asserts | Screenshot |
|------|---------|-----------|
| `E-create-org-three-decisions` | ≤ 3 labelled fields before Create; no stepper | `org-create` × light/dark/390 |
| `E-owner-already-member` | typing a member email swaps "Invite" for "Make owner" | `org-create-owner-existing` |
| `E-first-run-exclusive` | on an empty install the header has no create row | `first-run-only` |

### `create-workspace.spec.ts`

| Gate | Asserts | Screenshot |
|------|---------|-----------|
| `E-create-ws-three-decisions` | name + retrieval + models; Create reachable in 3 decisions | `ws-create` × light/dark/390 |
| `E-drawer-prints-inherited` | 6 rows, each with layer + value | `ws-create-drawer` |
| `E-drawer-edits-submitted` | editing 4 rows changes the POST body | `ws-create-drawer-edited` |
| `E-quota-disabled` | at limit: reason text present, Create inert, fields editable | `ws-create-quota-full` |
| `E-model-not-eligible` | 422 renders next to the model control | `ws-create-model-refused` |
| `E-refusal-keeps-fields` | 409 keeps all fields, focus on the alert | `ws-create-refused` |
| `E-reload-restores-draft` | reload → same values | — |
| `E-success-clears-draft` | after success, reopening is empty | — |
| `E-slug-taken-inline` | 409 slug shows "already exists · Open it" | `ws-create-slug-taken` |

### `access-gating.spec.ts`

| Gate | Asserts | Screenshot |
|------|---------|-----------|
| `E-forced-create-member` | `?create=organization` → refusal state, zero inputs | `refused-member` |
| `E-forced-create-wsadmin` | `?create=workspace` → refusal state | `refused-wsadmin` |
| `E-member-no-create` | switcher has no create rows; hint line present | `switcher-member` |
| `E-orgadmin-no-create-org` | workspace create present, org create absent | `switcher-orgadmin` |
| `E-sidebar-per-persona` | 4 personas, 4 sidebar snapshots | `sidebar-{pa,oo,wa,member}` |
| `E-palette-per-persona` | command list differs per persona | `palette-{pa,oo,wa,member}` |
| `E-palette-refetch` | losing a grant removes the command without reload | — |
| `E-grant-removed-live` | a grant removed mid-session drops the workspace from the switcher on the next `me-access` refetch, and an open page shows the no-access state | `grant-removed-live` |

### `grants.spec.ts`

| Gate | Asserts | Screenshot |
|------|---------|-----------|
| `E-two-grant-rows` | one person, two scope rows | `grants-editor` |
| `E-implicit-remains` | removing a ws grant leaves the org grant's implicit access, and says so | `grants-implicit` |
| `E-last-owner-locked` | owner select disabled + promote hint | `grants-last-owner` |
| `E-invite-scope-selector` | scope radio changes the role list and the seat warning | `invite-scope` |

### `navigation.spec.ts`

| Gate | Asserts | Screenshot |
|------|---------|-----------|
| `E-switch-no-blank` | org switch never renders the spinner state | — |
| `E-switch-keeps-section` | `/w/legal/documents` → `/w/finance/documents` | — |
| `E-selection-heals` | deleted workspace falls back with a toast naming it | `selection-healed` |
| `E-no-workspace-wall` | instructive state, no create button | `no-workspace-wall` |
| `E-org-deleted-under-user` | 404 clears the selection | `org-gone` |
| `E-deeplink-hidden` | `/w/unknown/documents` → not-found-with-switcher | `deeplink-hidden` |
| `E-archived-group` | archived workspace under its own group | `switcher-archived` |

### `destructive.spec.ts`

| Gate | Asserts | Screenshot |
|------|---------|-----------|
| `E-delete-requires-slug` | Delete inert until the slug is typed | `delete-confirm` |
| `E-delete-progress` | 9 plane rows, done/pending states | `delete-progress` |
| `E-archive-frees-quota` | quota strip goes 10/10 → 9/10 after archive | `archive-quota` |
| `E-plan-overage-visible` | 180 / 50 drawn amber | `org-overage` |

---

## Keyboard / a11y gates

| Gate | Asserts |
|------|---------|
| `A-dialog-focus` | first field focused, `select()` when prefilled |
| `A-dialog-trap` | Tab cycles inside; Esc asks to discard when dirty |
| `A-alert-announced` | refusal is in a `role="alert"` and receives focus |
| `A-quota-live` | quota updates announced by `aria-live="polite"` |
| `A-contrast` | axe: 0 critical, 0 serious on every new screen, both themes |
| `A-reduced-motion` | no transform when `prefers-reduced-motion: reduce` |
| `A-no-literal-color` | grep: no `#rrggbb` / `text-gray-` in `components/admin/**` |

## Structural gates

Cheap greps that prevent the DRY regressions from creeping back.

| Gate | Command | Fails when |
|------|---------|-----------|
| `S-one-create-org` | `rg -l "POST.*tenants[^/]*['\"\`]" src/lib/api` | more than one client function posts a tenant |
| `S-one-access-reader` | `rg -l "me/access" src` | more than one file reads the endpoint |
| `S-no-rank-math` | `rg "(level|role)\s*(>=|===)\s*[0-9]" src/components` | a component compares ranks |
| `S-no-second-matrix` | `rg "can_manage_workspaces|can_manage_users" src` | the client restates a server predicate |
| `S-wizard-single-use` | `rg -l "WizardShell" src` | more than the first-run and reconfigure wizards import it |

## Visual gates

Screenshots are reviewed, not just diffed. The checklist is
[11 §7](11-design-system.md#7--what-a-reviewer-checks). Output layout:

```text
specs/181-admin/e2e/screenshots/<SPEC181_ROUND>/
  org-create-{light,dark}-{390,768,1440}.png
  ws-create-{light,dark}-{390,768,1440}.png
  ws-create-drawer-{light,dark}-1440.png
  ws-create-quota-full-light-1440.png
  switcher-{pa,oo,wa,member}-light-1440.png
  sidebar-{pa,oo,wa,member}-light-1440.png
  grants-editor-light-1440.png
  invite-scope-light-1440.png
  delete-confirm-{light,dark}-1440.png
  delete-progress-light-1440.png
  no-workspace-wall-light-1440.png
```

## CI wiring

`make test-e2e-full` adds:

```make
test-e2e-spec181:
	cd edgequake_webui && SPEC181_ROUND=ci npx playwright test e2e/spec181 --project=mock-api
	cargo test -p edgequake-api --test e2e_spec181_tenancy
	cargo test -p edgequake-api --test spec181_catalog_matches_enforcement
	python3 specs/181-admin/scripts/validate-cross-ref.py specs/181-admin
```

The last line is the [LAW-181-16](01-first-principles.md) gate: 48 edge cases, 48 gates, or
the build fails.

## Coverage matrix — EC to gate

| EC | Gate(s) |
|----|---------|
| EC-181-01 | `R-create-rollback` |
| EC-181-02 | `R-interim-owner` |
| EC-181-03 | `R-cross-org-owner` |
| EC-181-04 | `E-owner-already-member` |
| EC-181-05 | `R-slug-race` |
| EC-181-06 | `E-forced-create-member` |
| EC-181-07 | `E-forced-create-wsadmin` |
| EC-181-08 | `R-idempotent-create` |
| EC-181-09 | `R-no-upsert` |
| EC-181-10 | `R-slug-sanitize-collision` |
| EC-181-11 | `E-drawer-edits-submitted` |
| EC-181-12 | `E-model-not-eligible` |
| EC-181-13 | `E-first-run-exclusive` |
| EC-181-14 | `R-quota-limit`, `E-quota-disabled` |
| EC-181-15 | `R-quota-race` |
| EC-181-16 | `R-archive-frees-quota`, `E-archive-frees-quota` |
| EC-181-17 | `R-migration-backfill` |
| EC-181-18 | `E-plan-overage-visible` |
| EC-181-19 | `R-two-grants`, `E-two-grant-rows` |
| EC-181-20 | `R-wsadmin-sibling-403` |
| EC-181-21 | `R-wsadmin-cannot-delete` |
| EC-181-22 | `R-last-owner`, `E-last-owner-locked` |
| EC-181-23 | `R-platform-cannot-orphan` |
| EC-181-24 | `R-grant-scope-mismatch` |
| EC-181-25 | `R-grant-exists` |
| EC-181-26 | `R-sso-pin` |
| EC-181-27 | `E-implicit-remains` |
| EC-181-28 | `R-access-fail-closed`, `E-grant-removed-live` |
| EC-181-29 | `R-tenant-list-scoped` |
| EC-181-30 | `R-reason-codes` |
| EC-181-31 | `R-purge-during-ingest` |
| EC-181-32 | `R-purge-partial-retry` |
| EC-181-33 | `R-purge-in-progress` |
| EC-181-34 | `R-purge-idempotent` |
| EC-181-35 | `R-purge-archived-ws` |
| EC-181-36 | `E-org-deleted-under-user` |
| EC-181-37 | `E-switch-no-blank` |
| EC-181-38 | `E-switch-keeps-section` |
| EC-181-39 | `E-selection-heals` |
| EC-181-40 | `E-no-workspace-wall` |
| EC-181-41 | `E-deeplink-hidden` |
| EC-181-42 | `E-palette-refetch` |
| EC-181-43 | `E-archived-group` |
| EC-181-44 | `R-migration-order` |
| EC-181-45 | `R-migration-ownerless` |
| EC-181-46 | `R-delete-202-legacy` |
| EC-181-47 | `R-settings-access-adapter` |
| EC-181-48 | `R-mcp-scope` |
