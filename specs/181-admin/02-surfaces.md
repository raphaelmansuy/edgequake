# 02 — Surfaces (every file and symbol this pack touches)

Parent: [README](README.md) · Prev: [01-first-principles](01-first-principles.md) ·
Next: [03-product-spec](03-product-spec.md)

Paths are relative to the repository root. `R` = read and change, `N` = new file, `D` =
delete.

## Backend — creation and lifecycle

| # | Path | Symbols | Disposition |
|---|------|---------|-------------|
| 1 | `edgequake/crates/edgequake-api/src/handlers/workspaces/tenants.rs` | `create_tenant` (230 lines), `list_tenants`, `get_tenant`, `update_tenant`, `delete_tenant` | **R** — shrink `create_tenant` to parse + delegate; extract model-policy block into `services/tenancy/model_seed.rs` |
| 2 | `edgequake/crates/edgequake-api/src/handlers/workspaces/workspace_crud.rs` | `create_workspace` (200 lines), `delete_workspace` (200 lines), `update_workspace` | **R** — `create_workspace` delegates; `delete_workspace` becomes `ScopePurge::purge(Workspace)` |
| 3 | `edgequake/crates/edgequake-api/src/handlers/workspaces/tenant_access.rs` | `TenantAccess`, `require_platform_admin`, `require_tenant_access`, `member_tenants`, `membership_scoped` | **R** — add `TenantAccess::Create`, `TenantAccess::Delete`; replace string reasons with `capability_required:<id>` |
| 4 | `edgequake/crates/edgequake-api/src/handlers/workspaces/helpers.rs` | `generate_slug`, `verify_workspace_tenant_access`, `workspace_to_response_with_tenant` | **R** — `generate_slug` moves to `services/tenancy/slug.rs` and gains collision suffixing |
| 5 | `edgequake/crates/edgequake-api/src/handlers/workspaces_types/requests.rs` | `CreateTenantRequest`, `CreateWorkspaceApiRequest`, `UpdateTenantRequest` | **R** — `CreateTenantRequest` gains `owner` (email or existing user id), `slug`, `plan`; `CreateWorkspaceApiRequest` gains nothing (already complete) |
| 6 | `edgequake/crates/edgequake-api/src/handlers/workspaces_types/responses.rs` | `TenantResponse`, `WorkspaceResponse` | **R** — `TenantResponse` gains `workspaces_used`, `members_used`, `grants`, `my_role` |
| 7 | `N` `edgequake/crates/edgequake-api/src/services/tenancy/mod.rs` | module root | **N** |
| 8 | `N` `.../services/tenancy/create.rs` | `TenancyService::create_organization`, `create_workspace` | **N** — the one transaction (LAW-181-2) |
| 9 | `N` `.../services/tenancy/quota.rs` | `Quota`, `QuotaSource`, `resolve_quota`, `assert_workspace_capacity`, `assert_seat_capacity` | **N** — LAW-181-6 |
| 10 | `N` `.../services/tenancy/purge.rs` | `Scope`, `ScopePurge`, `PurgeReceipt`, `purge_workspace`, `purge_organization` | **N** — LAW-181-7, moves the body of today's `delete_workspace` |
| 11 | `N` `.../services/tenancy/slug.rs` | `slugify`, `unique_tenant_slug`, `unique_workspace_slug` | **N** — DRY: one slug rule for both scopes (today `helpers.rs` and `setup.rs` each have one) |
| 12 | `N` `.../services/tenancy/model_seed.rs` | `seed_tenant_models`, `seed_workspace_models` | **N** — extracted from `create_tenant` (SPEC-178 / SPEC-032 block) |
| 13 | `edgequake/crates/edgequake-api/src/services/role_catalog.rs` | `CAPABILITIES`, `build_role_catalog`, `ORGANIZATION_ROLES` | **R** — `CapabilityDef` gains `axis`; seven capabilities become fourteen; axis added to `CapabilityInfo` |
| 14 | `N` `edgequake/crates/edgequake-api/src/handlers/me_access.rs` | `get_me_access` | **N** — supersedes `me_settings_access.rs` with a generalized grant list + quota. See [08](08-api-contract.md#get-meaccess) |
| 15 | `edgequake/crates/edgequake-api/src/handlers/me_settings_access.rs` | `grants`, `scope_for`, `administered_workspace_ids`, `strongest_role` | **R** — keep the pure functions (SPEC-175 compatibility), re-export from `me_access` |
| 16 | `edgequake/crates/edgequake-api/src/handlers/members/list.rs` | `summarize`, `sort_rows`, `paginate`, `list_members` | **R** — accept `workspace_id` filter |
| 17 | `edgequake/crates/edgequake-api/src/handlers/members/view.rs` | `primary_membership`, `member_view`, `MemberFacts` | **R** — project **all** grants, not only the primary |
| 18 | `edgequake/crates/edgequake-api/src/handlers/members/directory.rs` | `load_people`, `grants_of`, `last_owner`, `actor_for` | **R** — `load_people` keeps every active grant |
| 19 | `edgequake/crates/edgequake-api/src/handlers/members/guards.rs` | `Actor`, `check_assign`, `check_remove`, `Denial` | **R** — rank becomes `(axis, role)` aware; `Denial` gains `CrossAxis` |
| 20 | `edgequake/crates/edgequake-api/src/handlers/members/update.rs` | `change_role`, `change_account`, `update_member` | **R** — accept `workspace_id` to target a specific grant |
| 21 | `edgequake/crates/edgequake-api/src/handlers/members/invitations.rs` | `create_invitation`, `invitation_view`, `assert_can_invite` | **R** — `workspace_id` becomes first-class and is validated against the target scope |
| 22 | `N` `.../handlers/members/grants.rs` | `post_grant`, `patch_grant`, `delete_grant` | **N** — explicit grant CRUD (LAW-181-10) |
| 23 | `edgequake/crates/edgequake-api/src/routes.rs` | `api_v1_routes` | **R** — adds `GET /me/access`, `POST /tenants/{t}/workspaces/{w}/grants`, `POST /tenants/{t}/archive` |
| 24 | `edgequake/crates/edgequake-api/src/handlers/roles.rs` | `get_role_catalog` | **R** — no shape change (the catalog behind it changes) |
| 25 | `edgequake/crates/edgequake-api/src/openapi.rs` | path list | **R** — register the new routes and schemas |
| 26 | `edgequake/crates/edgequake-api/src/error.rs` | `ApiError::forbidden_reason`, `diagnostic_details` | **R** — a `reason_code` field carried in `details` |

## Backend — storage and types

| # | Path | Symbols | Disposition |
|---|------|---------|-------------|
| 27 | `edgequake/crates/edgequake-core/src/workspace_service_impl/tenant_ops.rs` | `pg_create_tenant`, `pg_delete_tenant`, `pg_list_tenants` | **R** — `pg_create_tenant` loses the `ON CONFLICT` trick (LAW-181-8); `pg_delete_tenant` delegates to `ScopePurge` |
| 28 | `edgequake/crates/edgequake-core/src/workspace_service_impl/workspace_ops.rs` | `pg_create_workspace` (quota check at :51) | **R** — quota moves to `services/tenancy/quota.rs`, called **inside** the transaction |
| 29 | `edgequake/crates/edgequake-core/src/workspace_service_impl/quota_ops.rs` | `pg_update_tenant_quota`, `pg_get_server_default_max_workspaces` | **R** — writes the real columns, not `metadata` |
| 30 | `edgequake/crates/edgequake-core/src/workspace_service_impl/rows.rs` | `into_tenant` (metadata fallbacks) | **R** — read columns first, metadata only as a migration fallback |
| 31 | `edgequake/crates/edgequake-core/src/workspace_service_impl/mod.rs` | `build_tenant_metadata` | **R** — stop duplicating quota into metadata once the columns exist |
| 32 | `edgequake/crates/edgequake-core/src/types/multitenancy/tenant.rs` | `Tenant::new` (`max_workspaces: 5`), `TenantPlan` | **R** — `Tenant::new` takes a `Quota`; `TenantPlan` becomes a lookup into `subscription_plans` |
| 33 | `edgequake/crates/edgequake-core/src/types/multitenancy/membership.rs` | `Membership`, `MembershipRole`, `applies_to`, `can_*` | **R** — add `MembershipRole::rank_in(axis)`; predicates unchanged (they are the catalog's source) |
| 34 | `N` `edgequake/crates/edgequake-core/src/types/multitenancy/scope.rs` | `Scope`, `ScopeAxis`, `Grant` | **N** — the value objects the whole pack shares |
| 35 | `edgequake/crates/edgequake-auth/src/tenant.rs` | `TenantService`, `TenantContext`, `CreateTenantRequest` | **D** — dead: re-exported, never called by the API; a second `TenantService` in `edgequake-core` has a different meaning (LSP violation) |
| 36 | `edgequake/crates/edgequake-auth/src/rbac.rs` | `Permission`, `RbacService` | **R** — `role_permissions` stays the instance-axis source; the doc comment stops claiming a workspace matrix it does not have |

## Database

| # | Path | Disposition |
|---|------|-------------|
| 37 | `N` `edgequake/migrations/226_spec181_admin.sql` | **N** — see [07](07-data-and-db-contract.md) |
| 38 | `edgequake/migrations/manifest.toml` | **R** — append version 226, bump `compat_serve_max` |

## Frontend — the two dialogs

| # | Path | Symbols | Disposition |
|---|------|---------|-------------|
| 39 | `N` `edgequake_webui/src/components/admin/create-organization-dialog.tsx` | `CreateOrganizationDialog` | **N** — 3 decisions (LAW-181-4) |
| 40 | `N` `edgequake_webui/src/components/admin/create-workspace-dialog.tsx` | `CreateWorkspaceDialog` | **N** — 3 decisions |
| 41 | `N` `edgequake_webui/src/components/admin/advanced-drawer.tsx` | `AdvancedDrawer` | **N** — shared "Tune later" surface for both dialogs |
| 42 | `N` `edgequake_webui/src/components/admin/inherit-row.tsx` | `InheritRow` | **N** — the `Inherited from …: value` row (LAW-181-13) |
| 43 | `N` `edgequake_webui/src/components/admin/quota-strip.tsx` | `QuotaStrip` | **N** — `3 of 10 workspaces`, meter, reason when full |
| 44 | `N` `edgequake_webui/src/components/admin/owner-picker.tsx` | `OwnerPicker` | **N** — "you" or another member by email/id |
| 45 | `N` `edgequake_webui/src/hooks/use-create-scope.ts` | `useCreateOrganization`, `useCreateWorkspace` | **N** — the API sequence + cache invalidation + context switch, in one place |
| 46 | `edgequake_webui/src/components/onboarding/create-tenant-wizard.tsx` | `CreateTenantWizard` | **D** — replaced by #39; `onboarding/index.ts` drops the export |
| 47 | `edgequake_webui/src/components/onboarding/create-workspace-wizard.tsx` | `CreateWorkspaceWizard` | **D** — replaced by #40 |
| 48 | `edgequake_webui/src/components/onboarding/steps/*` | 14 step components | **R** — the reusable fields move into the Advanced drawer; the rest (`admin-credentials-step`, `review-*`) stay for `first-run-wizard` |
| 49 | `edgequake_webui/src/components/onboarding/first-run-wizard.tsx` | `FirstRunWizard` | **R** — keeps the long path: on an empty install the operator *is* choosing everything for the first time |
| 50 | `edgequake_webui/src/lib/onboarding/wizard-state.ts` | `stepsForWizard`, `WizardBlockerCode` | **R** — `create-tenant` / `create-workspace` arms become `['basics', 'review']`; blocker codes gain `quota_reached`, `capability_missing` |

## Frontend — navigation and access

| # | Path | Symbols | Disposition |
|---|------|---------|-------------|
| 51 | `N` `edgequake_webui/src/lib/access/use-access.ts` | `useAccess`, `NO_ACCESS`, `canCreateOrganization`, `canCreateWorkspace` | **N** — one hook over `GET /me/access` (LAW-181-1) |
| 52 | `N` `edgequake_webui/src/lib/access/capabilities.ts` | `Capability` union, `CAPABILITY_LABELS` | **N** — generated from the server catalog, not hand-written matrix |
| 53 | `edgequake_webui/src/components/layout/header-tenant-selector.tsx` | `HeaderTenantSelector` | **R** — create rows behind `useAccess`; quota shown; one dialog each |
| 54 | `edgequake_webui/src/components/layout/context-selector/context-selector-popover.tsx` | `ContextSelectorPopover` | **R** — takes `access` + `quota`; groups workspaces by the caller's grant |
| 55 | `edgequake_webui/src/components/layout/tenant-guard.tsx` | `TenantGuard` | **R** — empty states become actionable and role-aware (LAW-181-12) |
| 56 | `edgequake_webui/src/lib/keyboard/destinations.ts` | `DESTINATION_GROUPS` | **R** — every destination gains `requires: Capability`; `destinationsFor(access)` filters |
| 57 | `edgequake_webui/src/components/layout/sidebar-nav.tsx` | `SidebarNav` | **R** — group headings become scope names (`Organization`, `Deployment`) |
| 58 | `edgequake_webui/src/components/settings/shell/settings-nav.tsx` | `SettingsNav` | **R** — unchanged shape; the scopes behind it now include `Organization > Workspaces` |
| 59 | `edgequake_webui/src/lib/settings/registry.ts` | `SETTINGS_SECTIONS` | **R** — adds `organizations` (deployment) and `workspaces` (organization) sections |
| 60 | `N` `.../components/settings/sections/organizations-section.tsx` | `OrganizationsSection` | **N** — platform admin: every org, quota, archive |
| 61 | `N` `.../components/settings/sections/workspaces-section.tsx` | `WorkspacesSection` | **N** — org manager: every workspace, create, rename, delete |
| 62 | `N` `edgequake_webui/src/app/(dashboard)/organization/page.tsx` | — | **N** — the Organization scope landing page |
| 63 | `N` `edgequake_webui/src/app/(dashboard)/organization/members/page.tsx` | — | **N** — re-hosts `MembersPage` under the org scope |

## Frontend — grants and members

| # | Path | Symbols | Disposition |
|---|------|---------|-------------|
| 64 | `N` `.../components/members/grant-editor.tsx` | `GrantEditor`, `GrantRow` | **N** — one person, many grants (LAW-181-10) |
| 65 | `N` `.../components/members/scope-badge.tsx` | `ScopeBadge` | **N** — `Organization` / `Legal` |
| 66 | `edgequake_webui/src/components/members/member-row.tsx` | `MemberRow` | **R** — renders N grants, collapses to `+2` |
| 67 | `edgequake_webui/src/components/members/members-table.tsx` | `MembersTable` | **R** — a Scope column; filter by scope |
| 68 | `edgequake_webui/src/components/members/invite-dialog.tsx` | `InviteDialog` | **R** — scope selector (organization or one workspace) |
| 69 | `edgequake_webui/src/lib/api/edgequake/members.ts` | `MemberView`, `MemberViewer`, `createInvitation` | **R** — `grants: Grant[]`, `CreateGrantBody` |
| 70 | `edgequake_webui/src/lib/members/permissions.ts` | `roleChangeBlock`, `removeBlock`, `roleOptions` | **R** — per-grant decisions |
| 71 | `edgequake_webui/src/lib/members/role-meta.ts` | `ROLE_LEVEL`, `assignableRoles` | **R** — ranks come from `GET /roles` (`level` field), not a local constant |

## Frontend — state and API client

| # | Path | Symbols | Disposition |
|---|------|---------|-------------|
| 72 | `edgequake_webui/src/stores/use-tenant-store.ts` | `selectTenant`, `selectWorkspace`, `setTenants` | **R** — `selectTenant` no longer blanks `workspaces` before the new list arrives (a visible flash, [EC-181-33](12-edge-cases.md)) |
| 73 | `edgequake_webui/src/lib/onboarding/apply-created-workspace-context.ts` | `applyCreatedTenantContext`, `applyCreatedWorkspaceContext` | **R** — one `applyCreatedScope` handles both |
| 74 | `edgequake_webui/src/lib/api/edgequake/workspaces.ts` | `createTenant`, `createWorkspace`, `getTenants` | **R** — new response fields; `createTenant` sends `owner`, `plan`, `slug` |
| 75 | `N` `edgequake_webui/src/lib/api/edgequake/access.ts` | `fetchMeAccess` | **N** |

## Tests

| # | Path | Disposition |
|---|------|-------------|
| 76 | `N` `edgequake_webui/e2e/spec181/*.spec.ts` | **N** — six specs, see [14](14-e2e-test-matrix.md) |
| 77 | `N` `edgequake_webui/e2e/spec181/helpers/admin-mock.ts` | **N** — hermetic stateful mock of tenancy + access |
| 78 | `N` `edgequake/crates/edgequake-api/tests/e2e_spec181_tenancy.rs` | **N** — PostgreSQL tenancy, quota, purge |
| 79 | `N` `edgequake/crates/edgequake-api/tests/spec181_catalog_matches_enforcement.rs` | **N** — the property test behind LAW-181-1 |
| 80 | `edgequake/crates/edgequake-api/tests/e2e_tenant_authorization.rs` | **R** — extend with the new grant routes |

## Count

80 surfaces. 26 new files, 3 deletions, 51 edits. The deletions are all duplications
(`CreateTenantWizard`, `CreateWorkspaceWizard`, `edgequake_auth::tenant`).
