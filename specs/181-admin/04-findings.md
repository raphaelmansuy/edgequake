# 04 — Findings (F-181-01 … F-181-12)

Parent: [README](README.md) · Prev: [03-product-spec](03-product-spec.md) ·
Next: [05-authorization-model](05-authorization-model.md)

Every finding names the file and the symbol, quotes the offending line where useful, and
states the consequence. All of these were read from the tree at v0.38.3; none is inferred
from a comment.

---

## F-181-01 — Organization creation never grants ownership · **Critical**

**Where** `edgequake/crates/edgequake-api/src/handlers/workspaces/tenants.rs`,
`create_tenant` (lines 40–228); `edgequake-core/src/workspace_service_impl/tenant_ops.rs`,
`pg_create_tenant` (lines 19–67).

```rust
// tenants.rs — after create_tenant(...) succeeds
if created_new {
    if let Err(e) = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, default_workspace_request)
        .await
    {
        tracing::warn!(tenant_id = %created_tenant.tenant_id, error = %e,
            "Failed to auto-create default workspace");
        // Continue anyway - tenant was created successfully
    }
}
```

There is no `add_membership` call anywhere in the handler or in `pg_create_tenant`.

**Consequence.** The organization has zero memberships. `require_tenant_access` resolves the
role through `get_user_role`, which filters `workspace_id IS NULL`; nothing matches, so
`denied()`. `member_tenants` returns nothing for every non-platform user. `last_owner` is
`None`, so the last-owner guard protects nobody and the first invite can be issued by an
Admin whose own grant does not exist. The organization is permanently unadministrable except
by a platform admin — which is exactly the support ticket this pack exists to prevent.

**Secondary.** The default workspace creation failure is swallowed, so an organization can
also exist with zero workspaces. `TenantGuard` then shows the "create your first workspace"
empty state to the creator — who is not permitted to create one, because they hold no
membership.

- **Closes:** [LAW-181-2](01-first-principles.md) · [EC-181-01](12-edge-cases.md)
- **Journey:** [J-01](06-user-journeys.md#j-01-platform-admin-creates-an-organization)

---

## F-181-02 — Create affordances are not gated by access · **High**

**Where** `edgequake_webui/src/components/layout/context-selector/context-selector-popover.tsx`
(lines 108–119 and 185–193); `edgequake_webui/src/components/layout/header-tenant-selector.tsx`
(lines 265–299).

```tsx
<CommandItem value="create-organization" onSelect={onCreateTenant}
  data-testid="header-create-tenant">
  <Plus className="mr-2 h-4 w-4 text-muted-foreground" aria-hidden="true" />
  {t('tenant.createNew', 'New Organization')}
</CommandItem>
```

The component takes no `access` prop. `HeaderTenantSelector` renders both wizards
unconditionally and never reads `me/settings-access` — which exists and returns exactly the
flags needed (`instance_admin`, `tenant_manage`, `workspace_manage`,
`administered_workspace_ids`).

**Consequence.** A Member or Viewer can open an 8-step wizard, fill every control, and lose
everything on submit to `403 Admin role required` or `403 Workspace administration requires
owner/admin membership`. The draft is persisted, so the dead end survives a reload
([`use-wizard-draft-persistence`](../../edgequake_webui/src/lib/onboarding/use-wizard-draft-persistence.ts)).

**Also.** The same gap exists in the command palette: `DESTINATION_GROUPS` has no capability
field, so `Members` and `Settings → deployment` are offered to everyone.

- **Closes:** [LAW-181-3](01-first-principles.md), [LAW-181-11](01-first-principles.md)
- **EC:** [EC-181-06](12-edge-cases.md), [EC-181-07](12-edge-cases.md)

---

## F-181-03 — Creation is a sixteen-step questionnaire · **High**

**Where** `edgequake_webui/src/lib/onboarding/wizard-state.ts`, `stepsForWizard` (lines
205–244); `edgequake_webui/src/components/onboarding/step-meta.ts`.

```ts
} else if (kind === 'create-tenant') {
  steps = withIngestTuningBeforeExtraction(
    ['tenant-basics', 'models', 'workspace-basics', 'extraction', 'review'], true);
}
```

`withIngestTuningBeforeExtraction` inserts `chunking` and `extract-budget` before
`extraction`; `withDocumentParsingAfterModels` inserts `document-parsing` after `models`. Net:
8 steps for a tenant, 8 for a workspace.

Six of the eight ask for values that have a working inherited default
(`Tenant.default_llm_model`, `default_embedding_*`, `pdf_parser_backend`,
`extraction_language`, `chunking_mode`, `extract_budget_mode`), and
[LAW-123-8](../../edgequake/crates/edgequake-api/src/handlers/workspaces/workspace_crud.rs)
guarantees that omitting a field keeps it inherited. The wizard therefore asks the operator to
re-state defaults they cannot see.

**Consequence.** Eight steps and 14 form controls to create an empty workspace. `create-tenant` also does something
subtle and undocumented in the UI: it creates the tenant, then **PATCHes** the auto-created
`Default Workspace` (`updateWorkspace`) because the tenant endpoint already created one. Two
API calls, two failure modes, one dialog.

- **Closes:** [LAW-181-4](01-first-principles.md), [LAW-181-13](01-first-principles.md)
- **EC:** [EC-181-11](12-edge-cases.md), [EC-181-12](12-edge-cases.md)

---

## F-181-04 — Organization delete does not purge workspace planes · **Critical**

**Where** `edgequake-core/src/workspace_service_impl/tenant_ops.rs`, `pg_delete_tenant`
(lines 128–152).

```rust
sqlx::query("DELETE FROM workspaces WHERE tenant_id = $1")...
sqlx::query("DELETE FROM memberships WHERE tenant_id = $1")...
sqlx::query("DELETE FROM tenants WHERE tenant_id = $1")...
```

Compare `delete_workspace` in `handlers/workspaces/workspace_crud.rs` (lines 609–793), which
does nine things: `purge_workspace_tasks`, `vector_storage.clear_workspace`,
`graph_storage.clear_workspace`, `plan_workspace_document_kv_deletion`,
`purge_decision_workspace`, PDF row deletion, `vector_registry.evict`,
`vector_registry.drop_workspace_table`, then the row delete.

**Consequence.** An organization delete leaves:

| Plane | Left behind |
|-------|-------------|
| Per-workspace vector table | `eq_*_ws_<uuid>_vectors` tables stay on disk |
| AGE graph | nodes/edges remain in `ag_catalog` |
| KV | `wsdoc:*`, chunk keys, staging keys remain |
| `pdf_documents` | rows cascade by FK, but their objects do not |
| Object originals | MinIO/S3 prefixes remain when `EDGEQUAKE_ORIGINAL_STORE=object` |
| Decision cache | rows remain |
| Tasks | rows remain |

`delete_tenant` is also not wrapped in a transaction, so a mid-way failure leaves a partially
deleted organization.

- **Closes:** [LAW-181-7](01-first-principles.md)
- **EC:** [EC-181-22](12-edge-cases.md), [EC-181-23](12-edge-cases.md)
- **Threat:** [T-03](16-threat-model.md#t-03-orphan-data)

---

## F-181-05 — The workspace grant axis has no write path and no projection · **High**

**Where**

- Storage and enforcement: `memberships.workspace_id`
  ([008_add_multi_tenancy_tables.sql](../../edgequake/migrations/008_add_multi_tenancy_tables.sql))
  and `Membership::applies_to` / `can_access_workspace`
  ([membership.rs](../../edgequake/crates/edgequake-core/src/types/multitenancy/membership.rs)).
- Read path: `me_settings_access.rs::administered_workspace_ids` already derives from it.
- **Write path: absent.** `InviteDialog` calls `createInvitation(tenantId, {email, role})`
  with no `workspace_id`, although `CreateInvitationRequest` accepts one.
  `PATCH /members/{u}` takes `{role}` only.
- **Projection: collapsed.** `view.rs::primary_membership` returns *one* membership per
  person (`find(|m| m.workspace_id.is_none()).or_else(|| max_by_key(level))`), so a second
  grant is invisible.

**Consequence.** The Slack/Vercel model — organization member, admin of one workspace — cannot
be expressed through any UI or route. The only way to let Aïcha administer Legal is to make
her an organization Admin, which grants her Finance too. This is the difference between an
acceptable enterprise product and an unacceptable one for P-02 and P-03.

- **Closes:** [LAW-181-10](01-first-principles.md)
- **Journeys:** [J-05](06-user-journeys.md#j-05-org-owner-grants-one-workspace-admin),
  [J-06](06-user-journeys.md#j-06-workspace-admin-invites-to-own-workspace)
- **EC:** [EC-181-18](12-edge-cases.md), [EC-181-19](12-edge-cases.md), [EC-181-20](12-edge-cases.md)

---

## F-181-06 — Quota has four sources and no atomic check · **High**

**Where**

| Source | Value | File |
|--------|-------|------|
| `Tenant::new` | `max_workspaces: 5`, `max_users: 10` | `edgequake-core/src/types/multitenancy/tenant.rs:111-112` |
| `TenantPlan::Free` | `10` / `3` | `edgequake-auth/src/tenant.rs:69-80` |
| `metadata.max_workspaces` | any | written by `build_tenant_metadata`, read by `into_tenant` |
| server default | `100` | `AdminQuotaSection` + `pg_get_server_default_max_workspaces` |

`pg_create_workspace` checks `COUNT(*)` then inserts, with no lock:

```rust
let current_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspaces WHERE tenant_id = $1")...
if current_count as usize >= tenant.max_workspaces { return Err(Error::validation(...)); }
```

**Consequence.** A tenant created through `POST /tenants` without a `plan` gets **5** workspaces
(the struct default), not the **10** the plan says and not the **100** the operator configured.
`TenantPlan::default_max_workspaces` is only applied by `with_plan`, which runs only when the
request carries `plan`. The admin screen shows a server default that the create path never
reads. Two concurrent creates can both pass the count check. And a quota refusal arrives as
`400 validation`, not a conflict the UI can translate.

- **Closes:** [LAW-181-6](01-first-principles.md), [LAW-181-5](01-first-principles.md)
- **EC:** [EC-181-14](12-edge-cases.md), [EC-181-15](12-edge-cases.md), [EC-181-16](12-edge-cases.md)

---

## F-181-07 — Refusals carry prose, not codes · **Medium**

**Where** `tenant_access.rs` (`ApiError::forbidden_reason("Admin role required")`,
`"tenant_access_denied"`, `"Workspace administration requires owner/admin membership"`);
`workspace_ops.rs` (`Error::validation("Tenant has reached maximum workspace limit")`).

**Consequence.** `lib/members/errors.ts` matches refusals with regexes over the message text
(`[/seat_limit/i, "seatLimit"]`, …). A wording change silently breaks every translated error.
Creation refusals are not in that table at all, so the user sees a raw API sentence.

- **Closes:** [LAW-181-5](01-first-principles.md)
- **EC:** [EC-181-30](12-edge-cases.md)

---

## F-181-08 — `POST /tenants` is an upsert wearing a create's clothes · **Medium**

**Where** `pg_create_tenant`:

```rust
INSERT INTO tenants (...) VALUES (...)
ON CONFLICT (slug) DO UPDATE SET name = tenants.name
RETURNING tenant_id, ...
```

**Consequence.** The handler documents 200 "Tenant already existed (idempotent get-or-create by
slug)" and 409 "Tenant with this slug already exists" — both, for the same route. The 409 only
fires when the display name also differs. A client cannot distinguish "I created it" from
"it already existed" without comparing ids, which the contract does not mention. More
seriously, a caller who mistypes a slug silently lands inside **someone else's organization**.

- **Closes:** [LAW-181-8](01-first-principles.md)
- **EC:** [EC-181-09](12-edge-cases.md)
- **Threat:** [T-02](16-threat-model.md#t-02-slug-collision-into-a-foreign-org)

---

## F-181-09 — Navigation does not know about rights · **Medium**

**Where** `edgequake_webui/src/lib/keyboard/destinations.ts`, `DESTINATION_GROUPS`.

The three groups are static; only `needsGraph` filters. `Members` (`/members`) and the whole
`Settings → Deployment` scope are offered to every signed-in caller. `settings/registry.ts`
*does* gate by scope — but the sidebar and palette do not, so the two disagree.

**Consequence.** A member clicks Members, sees `MembersRestricted` ("Members are managed by
admins"). That is a correct empty state reached through an incorrect door. The sidebar also
has no concept of "Organization" vs "Deployment", so a platform admin cannot reach the
organization list at all except through the switcher.

- **Closes:** [LAW-181-11](01-first-principles.md)
- **EC:** [EC-181-27](12-edge-cases.md), [EC-181-28](12-edge-cases.md)

---

## F-181-10 — Empty states are not role-aware · **Medium**

**Where** `components/layout/tenant-guard.tsx` (lines 207–303).

```tsx
if (tenantsData && tenantsData.length === 0) {
  // "Welcome to EdgeQuake" + Create Tenant button, for everyone
}
```

**Consequence.** A member of an organization that was archived, or a person whose only
organization grant was removed, sees a "Create your first tenant" card with a button that
returns 403. The card also claims the product is empty when it is full — the person simply has
no grant.

- **Closes:** [LAW-181-12](01-first-principles.md)
- **EC:** [EC-181-26](12-edge-cases.md)

---

## F-181-11 — Two creation entry points disagree · **Medium**

**Where** `CreateTenantWizard` (header + guard) vs `useTenantContext().createTenantMutation`
(a plain `{name, description}` call used by nothing today but exported and documented as the
tenant creation hook) vs `POST /setup/initialize` (first-run, its own 200-line body).

**Consequence.** Three code paths create an organization with three different post-conditions:
the wizard PATCHes the default workspace, the mutation does not, the setup route configures
everything. Only one of them is exercised in tests.

- **Closes:** [LAW-181-15](01-first-principles.md)
- **EC:** [EC-181-13](12-edge-cases.md)

---

## F-181-12 — Two `TenantService` types mean different things · **Low, but structural**

**Where** `edgequake-auth/src/tenant.rs::TenantService` (create/list tenants, plan limits,
`check_workspace_limit` — **no caller in the API**) and
`edgequake-core/src/tenant_manager.rs::TenantService` (routes a tenant to a RAG instance).

**Consequence.** `check_workspace_limit` and `check_user_limit` exist, are unit-tested, and are
never called — so the *tested* quota rule is not the *enforced* one (F-181-06). Two public
types with one name violate LSP and mislead every reader.

- **Closes:** DRY ledger in [01](01-first-principles.md#dry--solid-ledger)
- **EC:** [EC-181-17](12-edge-cases.md)

---

## Severity summary

| Finding | Severity | Law | Wave |
|---------|----------|-----|------|
| F-181-01 ownership never granted | Critical | 181-2 | W1 |
| F-181-04 delete leaks every plane | Critical | 181-7 | W4 |
| F-181-02 ungated create | High | 181-3 | W3 |
| F-181-03 16-step questionnaire | High | 181-4 | W5 |
| F-181-05 no workspace grants | High | 181-10 | W2 |
| F-181-06 quota fiction | High | 181-6 | W1 |
| F-181-07 prose refusals | Medium | 181-5 | W1 |
| F-181-08 upsert | Medium | 181-8 | W1 |
| F-181-09 navigation | Medium | 181-11 | W3 |
| F-181-10 empty states | Medium | 181-12 | W3 |
| F-181-11 three create paths | Medium | 181-15 | W5 |
| F-181-12 duplicate TenantService | Low | DRY | W0 |
