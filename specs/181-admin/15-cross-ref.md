# 15 — Cross reference

Parent: [README](README.md) · Prev: [14-e2e-test-matrix](14-e2e-test-matrix.md) ·
Next: [16-threat-model](16-threat-model.md)

One table per axis. Every law has a finding, an edge case, a wave, a gate and a lens that owns
it. `scripts/validate-cross-ref.py` checks the parity.

---

## Law → everything

| Law | Title | Finding | ECs | Wave | Gates | Owner lens |
|-----|-------|---------|-----|------|-------|-----------|
| [LAW-181-1](01-first-principles.md#law-181-1--three-axes-one-derived-catalog) | Three axes, one derived catalog | F-181-05, F-181-12 | 19, 20, 27 | W0, W2, W7 | `catalog_equals_enforcement`, `R-two-grants`, `R-wsadmin-sibling-403` | Full Stack · UX |
| [LAW-181-2](01-first-principles.md#law-181-2--creating-a-scope-is-one-tenancy-transaction) | Creation is one tenancy transaction | F-181-01 | 01, 02, 03 | W1 | `R-create-rollback`, `R-interim-owner`, `R-create-two-requests` | Full Stack · DB |
| [LAW-181-3](01-first-principles.md#law-181-3--offer-only-what-the-server-will-accept) | Offer only what the server accepts | F-181-02 | 06, 07, 42 | W3 | `E-member-no-create`, `E-forced-create-member`, `E-forced-create-wsadmin`, `E-palette-refetch` | Front · UX |
| [LAW-181-4](01-first-principles.md#law-181-4--short-path-explicit-inherit-advanced-drawer) | Short path, explicit inherit, drawer | F-181-03 | 11, 12 | W5 | `E-create-ws-three-decisions`, `E-create-org-three-decisions` | Product · UX |
| [LAW-181-5](01-first-principles.md#law-181-5--refusals-are-reasons-not-403s) | Refusals are reasons | F-181-07 | 30 | W1 | `R-reason-codes` | Full Stack · Front |
| [LAW-181-6](01-first-principles.md#law-181-6--one-quota-one-atomic-check) | One quota, one atomic check | F-181-06 | 14, 15, 16, 17, 18 | W1, W4 | `R-quota-limit`, `R-quota-race`, `R-archive-frees-quota`, `R-migration-backfill`, `E-plan-overage-visible` | DB · Product |
| [LAW-181-7](01-first-principles.md#law-181-7--one-purge-for-every-scope) | One purge for every scope | F-181-04 | 31, 32, 33, 34, 35 | W4 | `R-purge-*`, `orphan_sweep_finds_zero_after_org_delete` | DB · Security |
| [LAW-181-8](01-first-principles.md#law-181-8--create-is-not-upsert) | Create is not upsert | F-181-08 | 05, 09 | W1 | `R-no-upsert`, `R-slug-race` | Full Stack |
| [LAW-181-9](01-first-principles.md#law-181-9--every-scope-change-is-audited) | Every scope change is audited | — | 31, 45 | W1, W4 | `R-create-rollback` (audit row count), `R-migration-ownerless` | Security |
| [LAW-181-10](01-first-principles.md#law-181-10--workspace-grants-are-first-class) | Workspace grants are first class | F-181-05 | 19, 20, 21, 22, 25, 26, 27 | W2, W7 | `R-two-grants`, `R-grant-exists`, `R-sso-pin`, `E-two-grant-rows` | Product · Front |
| [LAW-181-11](01-first-principles.md#law-181-11--navigation-is-scope-shaped) | Navigation is scope-shaped | F-181-09 | 30, 42, 43 | W3 | `E-sidebar-per-persona`, `E-palette-per-persona`, `E-palette-refetch` | UX · Front |
| [LAW-181-12](01-first-principles.md#law-181-12--empty-is-a-state-not-an-error) | Empty is a state, not an error | F-181-10 | 28, 40 | W3 | `E-no-workspace-wall`, `E-selection-heals` | UX |
| [LAW-181-13](01-first-principles.md#law-181-13--inheritance-is-printed) | Inheritance is printed | F-181-03 | 11, 27 | W5 | `E-drawer-prints-inherited`, `E-implicit-remains` | UX |
| [LAW-181-14](01-first-principles.md#law-181-14--the-wizard-never-loses-work) | The wizard never loses work | — | 08, 11, 12 | W5 | `E-refusal-keeps-fields`, `E-reload-restores-draft`, `E-success-clears-draft` | Front |
| [LAW-181-15](01-first-principles.md#law-181-15--one-entry-point-per-intent) | One entry point per intent | F-181-11 | 06, 07, 13 | W5 | `S-one-create-org`, `E-first-run-exclusive` | Full Stack |
| [LAW-181-16](01-first-principles.md#law-181-16--ci-is-proof-one-gate-per-ec) | CI is proof | — | all 48 | W8 | `validate-cross-ref.py` | every lens |

## Finding → disposition

| Finding | Severity | Closed by | Wave | Regression gate |
|---------|----------|-----------|------|-----------------|
| [F-181-01](04-findings.md#f-181-01--organization-creation-never-grants-ownership--critical) ownership never granted | Critical | LAW-181-2 | W1 | `R-create-rollback` |
| [F-181-02](04-findings.md#f-181-02--create-affordances-are-not-gated-by-access--high) ungated create | High | LAW-181-3, 181-11 | W3 | `E-member-no-create` |
| [F-181-03](04-findings.md#f-181-03--creation-is-a-sixteen-step-questionnaire--high) 16-step wizard | High | LAW-181-4, 181-13 | W5 | `E-create-ws-three-decisions` |
| [F-181-04](04-findings.md#f-181-04--organization-delete-does-not-purge-workspace-planes--critical) delete leaks | Critical | LAW-181-7 | W4 | `orphan_sweep_finds_zero_after_org_delete` |
| [F-181-05](04-findings.md#f-181-05--the-workspace-grant-axis-has-no-write-path-and-no-projection--high) no workspace grants | High | LAW-181-10 | W2 | `R-two-grants` |
| [F-181-06](04-findings.md#f-181-06--quota-has-four-sources-and-no-atomic-check--high) quota fiction | High | LAW-181-6 | W1, W4 | `R-quota-race` |
| [F-181-07](04-findings.md#f-181-07--refusals-carry-prose-not-codes--medium) prose refusals | Medium | LAW-181-5 | W1 | `R-reason-codes` |
| [F-181-08](04-findings.md#f-181-08--post-tenants-is-an-upsert-wearing-a-creates-clothes--medium) upsert | Medium | LAW-181-8 | W1 | `R-no-upsert` |
| [F-181-09](04-findings.md#f-181-09--navigation-does-not-know-about-rights--medium) navigation | Medium | LAW-181-11 | W3 | `E-sidebar-per-persona` |
| [F-181-10](04-findings.md#f-181-10--empty-states-are-not-role-aware--medium) empty states | Medium | LAW-181-12 | W3 | `E-no-workspace-wall` |
| [F-181-11](04-findings.md#f-181-11--two-creation-entry-points-disagree--medium) three create paths | Medium | LAW-181-15 | W5 | `S-one-create-org` |
| [F-181-12](04-findings.md#f-181-12--two-tenantservice-types-mean-different-things--low-but-structural) duplicate TenantService | Low | DRY ledger | W0 | compile |

## Journey → persona matrix

See [06 §journey matrix](06-user-journeys.md#journey-matrix--the-negative-space)

| Journey | Persona | Capability exercised | Gate |
|---------|---------|---------------------|------|
| J-01 | PA | `platform.organizations.create` | `E-create-org-three-decisions`, `R-create-rollback` |
| J-02 | OO | `organization.workspaces.create` | `E-create-ws-three-decisions` |
| J-03 | OM+WA | (refused) `organization.workspaces.create` | `E-forced-create-wsadmin` |
| J-04 | OO | `organization.members.manage` | `E-invite-scope-selector` |
| J-05 | OO | `organization.members.manage` → workspace grant | `R-two-grants`, `E-two-grant-rows` |
| J-06 | OM+WA | `workspace.members.manage` | `E-invite-scope-selector` |
| J-07 | PA | platform short-circuit + audit | `R-access-fail-closed` |
| J-08 | OO | `organization.settings.manage` | `E-plan-overage-visible` |
| J-09 | PA | `platform.organizations.delete` | `E-delete-requires-slug`, `E-delete-progress` |
| J-10 | OO | `organization.workspaces.create` + quota | `E-archive-frees-quota` |
| J-11 | OM, OV | `organization.read` only | `E-no-workspace-wall` |
| J-12 | OO | quota refusal | `E-quota-disabled` |

## Surface → wave

| Surface group | Files | Wave |
|---------------|-------|------|
| Cleanup, vocabulary | `edgequake-auth/tenant`, `tenant_manager`, `scope.rs`, `slug.rs`, `error.rs`, `role_catalog.rs` | W0 |
| Tenancy service | `services/tenancy/*`, `tenants.rs`, `workspace_crud.rs`, `tenant_ops.rs`, `quota_ops.rs`, `rows.rs` | W1 |
| Grants | `members/grants.rs`, `guards.rs`, `view.rs`, `directory.rs`, `update.rs`, `invitations.rs` | W2 |
| Access + nav | `me_access.rs`, `use-access.ts`, `capabilities.ts`, `destinations.ts`, `tenant-guard.tsx`, `use-tenant-store.ts` | W3 |
| Purge + schema | `services/tenancy/purge.rs`, `226_spec181_admin.sql`, `manifest.toml` | W4 |
| Dialogs | `components/admin/*`, `use-create-scope.ts`, delete 2 wizards | W5 |
| Org + deployment pages | `organization/*`, `settings/sections/{organizations,workspaces}-section.tsx` | W6 |
| Grants UI | `grant-editor.tsx`, `member-row.tsx`, `members-table.tsx`, `invite-dialog.tsx`, `roles-tab.tsx` | W7 |
| Polish | screenshots, locales, OpenAPI, docs | W8 |

## Lens index

| Lens | File | Answers |
|------|------|---------|
| Product Owner | [lenses/LENS-product-owner.md](lenses/LENS-product-owner.md) | why now, what ships first, how we know it worked |
| Full Stack | [lenses/LENS-full-stack.md](lenses/LENS-full-stack.md) | the transaction, the handler split, the typed boundary |
| Database | [lenses/LENS-database.md](lenses/LENS-database.md) | columns, triggers, indexes, migration safety |
| UX / UI | [lenses/LENS-ux-ui.md](lenses/LENS-ux-ui.md) | the twelve journeys as flows, the refusal copy |
| Front | [lenses/LENS-front.md](lenses/LENS-front.md) | the hook, the query keys, the component split |
| AI Engineer | [lenses/LENS-ai-engineer.md](lenses/LENS-ai-engineer.md) | model defaults at creation, MCP parity, no Acc drift |
| Design System | [lenses/LENS-design-system.md](lenses/LENS-design-system.md) | the gr8r-studio borrow, token by token |

## External references

| Spec | Relationship |
|------|--------------|
| [SPEC-101](../101-wizard-mode-tenant-workspace/) | The wizard engine. This pack shortens two of its four kinds and deletes two components. |
| [SPEC-123](../123-env-config-priority/) | Model ladder. LAW-123-8 (omitted field stays inherited) is why the short path is safe. |
| [SPEC-150](../150-reliable-migration-system/) | Migration manifest, lock classes, checksums. |
| [SPEC-158](../158-entreprise-grade-authentication/) | SSO provisioning, `(iss, sub)`, `strict_tenant_bind`, role re-sync. |
| [SPEC-168](../168-perfect-ui/) | UI bar, keyboard contract, CLS budget. |
| [SPEC-169](../169-perfect-user-management/) | Members, invitations, two role axes, last-owner guard. This pack adds the third axis's write path. |
| [SPEC-174](../174-document-security/) | Clearance ABAC. Composes after the three axes; never widened by them. |
| [SPEC-175](../175-perfect-settings/) | Settings scopes, `me/settings-access`, role labels, chip order. |
| [SPEC-178](../178-provider-and-model-choice/) | Model eligibility at creation (`ensure_tenant_choice`, `ensure_workspace_choice`). |
| [SPEC-180](../180-normal-rag/) | Immutable `rag_profile` / `embedding_modalities` / `fts_regconfig`. Retrieval type is therefore a create-time decision and is surfaced as one. |
| [SPEC-001](../001-benchmark/) | Acc benchmark. Untouched by this pack; gated in the definition of done. |

## Design references

| Product | What we took | File |
|---------|--------------|------|
| gr8r-studio | token semantics, `srow`/`sblock`, segmented control, meter, capability table density, 252/30 layout | [11](11-design-system.md) |
| GitHub Organizations | org role vs repo role as separate grants | [05 §8](05-authorization-model.md#8--why-this-matches-the-best-saas) |
| Vercel Teams | team owner vs project access; per-project member list | [05 §8](05-authorization-model.md#8--why-this-matches-the-best-saas) |
| Slack Enterprise Grid | org admin vs workspace admin naming | [05 §8](05-authorization-model.md#8--why-this-matches-the-best-saas) |
| Stripe | top role cannot orphan the account | [LAW-181-10](01-first-principles.md) · I-1 |
| Linear | search-first settings, one section per route | [SPEC-175](../175-perfect-settings/09-ux-ui-spec.md) |
