# Lens — Full Stack Developer

Parent: [README](../README.md) · Spec: [02](../02-surfaces.md) · [08](../08-api-contract.md) · [13](../13-implementation-plan.md)

## The shape I am building

```text
  handler (thin)          services/tenancy (policy)        store (SQL)
  ─────────────           ──────────────────────────        ──────────
  parse body              authorize(actor, capability)      transaction {
  call service            resolve quota                       insert tenant
  map Result → HTTP       build entities                      insert grants
                          call store                          insert workspace
                                                              audit
                                                            }
```

Today `create_tenant` is 230 lines doing all four jobs in one function, including a
model-policy check (`ensure_tenant_choice`), a best-effort workspace insert with a swallowed
error, and three provider heuristics. That is the function to split.

## Boundaries I will not cross

| Rule | Why |
|------|-----|
| A handler never opens a transaction | `TenancyService` owns it; a handler that writes two rows is the bug we are fixing |
| Policy is a pure function over `Actor` + `Scope` | unit-testable with `InMemoryTenancyStore`, no PG needed |
| The store trait has 6 methods, not 40 | `WorkspaceService` is a 40-method god trait; the tenancy path needs six. ISP. |
| A refusal is an enum variant, not a string | `Denial::{LastOwner, CrossAxis, Rank, QuotaReached}` → mapped once to a reason code |
| No new `unwrap()` / `expect()` in a handler | `cargo clippy -D warnings` |

## The six store methods

```rust
#[async_trait]
pub trait TenancyStore: Send + Sync {
    async fn insert_tenant(&self, t: Tenant) -> Result<Tenant>;
    async fn insert_workspace(&self, tenant: Uuid, r: CreateWorkspaceRequest) -> Result<Workspace>;
    async fn insert_grant(&self, g: Grant) -> Result<Grant>;
    async fn tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>>;
    async fn count_workspaces(&self, tenant: Uuid) -> Result<usize>;
    async fn count_members(&self, tenant: Uuid) -> Result<usize>;
}
```

`PgTenancyStore` wraps the existing `WorkspaceServiceImpl` methods inside one `sqlx::Transaction`.
That is the only place `Transaction` appears in this pack.

## The authorization seam

```rust
pub struct Actor {
    pub user_id: Uuid,
    pub platform_admin: bool,
    pub org: Option<MembershipRole>,
    pub workspaces: HashMap<Uuid, MembershipRole>,
}

impl Actor {
    pub fn level_in(&self, scope: &Scope) -> u8 { … }
    pub fn may(&self, cap: Capability, scope: &Scope) -> bool { … }
}
```

`Actor` is built **once** per request from the same rows `me/access` reads, so the handler and
the UI agree by construction. `require_tenant_access` becomes:

```rust
actor.may(TenantAccess::Manage.capability(), &Scope::organization(tenant_id))
    .then_some(())
    .ok_or_else(|| ApiError::forbidden_reason(format!("capability_required:{}", cap.id)))
```

## What I reuse, and what I refuse to duplicate

| Reuse | Source |
|-------|--------|
| `ensure_tenant_choice`, `ensure_workspace_choice` | SPEC-178 `services/model_policy_enforcement.rs` — called, never re-implemented |
| `ensure_embedding_matches_modalities` | SPEC-180 `services/multimodal_policy.rs` |
| `MembershipRole::{can_write, can_manage_users, can_manage_workspaces, can_delete_tenant}` | the catalog's predicates; this pack adds `level_in(axis)` only |
| `delete_workspace`'s nine-step body | moved verbatim into `purge_workspace`; the tenant path calls it per workspace |
| `pg_assign_membership_role` (SSO pinning) | unchanged |
| `guards.rs::{check_assign, check_remove}` | extended with `CrossAxis`, logic preserved |
| `record_compliance_event` | every new mutation uses it |
| `Idempotency-Key` | new, but only on the two create routes |

| Duplicate I will delete | Where |
|------------------------|-------|
| `ON CONFLICT (slug) DO UPDATE SET name = tenants.name` | `pg_create_tenant` |
| the quota `COUNT(*)` then `INSERT` | `pg_create_workspace:51` |
| `generate_slug` × 2 | `helpers.rs`, `setup.rs` |
| the deleted-workspace cascade | now only in `purge_workspace` |
| `check_workspace_limit` / `check_user_limit` (never called) | `edgequake-auth/tenant.rs`, deleted with the module |

## Rust sketch — create organization

```rust
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub plan: Option<String>,
    #[serde(default)] pub owner: OwnerChoice,
    #[serde(flatten)]  pub models: TenantModelDefaults,
    #[serde(default)]  pub primary_color: Option<String>,
    #[serde(default)]  pub workspace: Option<WorkspaceSeed>,
}

pub async fn create_tenant(
    State(state): State<AppState>,
    auth: ApiAuthenticated,
    headers: HeaderMap,
    Json(req): Json<CreateOrganizationRequest>,
) -> Result<(StatusCode, Json<CreateOrganizationResponse>), ApiError> {
    let actor = state.tenancy.actor(&auth.context()).await?;
    actor.require(Capability::PlatformOrganizationsCreate, &Scope::Deployment)?;

    let key = headers.get("idempotency-key").and_then(|v| v.to_str().ok());
    if let Some(created) = state.tenancy.replay(key).await? {
        return Ok((StatusCode::CREATED, Json(created)));
    }

    let created = state
        .tenancy
        .create_organization(&actor, req.into_domain()?)
        .await?;

    Ok((StatusCode::CREATED, Json(created)))
}
```

The handler is 18 lines. Everything interesting is testable without Axum.

## Frontend seam

```ts
// one query
const { data: access, isPending } = useQuery({
  queryKey: ['me-access', tenantId, workspaceId],
  queryFn: () => fetchMeAccess(tenantId, workspaceId),
  staleTime: 30_000,
  retry: false,
});

// one mutation per intent
const createOrg = useCreateOrganization();
createOrg.mutate({ name, owner, plan, ... });
```

No component builds a URL, sets a header, invalidates a second cache key, or calls
`setTenantContext`. That is `hooks/use-create-scope.ts` and nowhere else.

## Testing strategy, in the order I write it

1. **Pure policy** with `InMemoryTenancyStore` — quota resolution, owner grant, rollback on
   injected error. Fast, no DB.
2. **Guards** — `check_assign` / `check_remove` matrix, `CrossAxis`.
3. **Contract** with PostgreSQL — the transaction, the trigger, the purge sweep.
4. **Playwright hermetic** — the dialog, the refusal, the draft, the switcher.
5. **Live stack smoke** — one end-to-end create on `make dev-bg`, manual, before merge.

## What will bite

| Trap | Mitigation |
|------|-----------|
| The deferred trigger fires at COMMIT, so the error surfaces from `commit()`, not from the `INSERT` | map `check_violation` on commit to `tenant_requires_owner`; test it explicitly |
| `FOR UPDATE` in a `BEFORE INSERT` trigger on the tenant row deadlocks against a concurrent `UPDATE tenants` | order: trigger locks the tenant, never the reverse; `PUT /tenants` does not lock |
| `sqlx::Transaction` + the existing `WorkspaceServiceImpl` methods use `&self.pool`, not the tx | add `_in_tx` variants for the six operations only |
| Deleting the two wizards breaks `spec169` / `spec180` specs | update both in the W5 PR |
| `me/access` per workspace is N+1 | one SQL statement with a `GROUP BY`, not a loop |
