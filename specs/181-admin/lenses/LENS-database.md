# Lens — Database Expert

Parent: [README](../README.md) · Spec: [07](../07-data-and-db-contract.md)

## What is wrong with the current schema

```sql
-- tenants, as it actually exists after migrations 001 + 008
tenant_id     uuid pk
name          varchar(255)
slug          varchar(100) unique
settings      jsonb
metadata      jsonb      -- plan, max_workspaces, max_users, description,
                         -- default_llm_*, pdf_parser_backend, primary_color
is_active     boolean
created_at / updated_at
```

Every quota, plan and default is a **jsonb key read with `unwrap_or`**:

```rust
let max_workspaces = self.metadata.get("max_workspaces")
    .and_then(|v| v.as_u64()).unwrap_or(5) as usize;   // F-181-06
```

There is no type, no constraint, no default expression, and the fallback (5) contradicts the
plan it claims to read (10). Migration 008's `plan` and `max_workspaces` columns were added to a
table that migration 001 had already created with `IF NOT EXISTS`, so they exist on fresh
databases and are ignored in favour of `metadata`.

That is the whole database problem: **the tenancy columns are documents, not columns**.

## Target shapes

### `subscription_plans`

```sql
CREATE TABLE subscription_plans (
    id              text PRIMARY KEY,
    label           text NOT NULL,
    max_workspaces  integer NOT NULL CHECK (max_workspaces > 0),
    max_users       integer NOT NULL CHECK (max_users >= 0),   -- 0 = unlimited
    sort_order      integer NOT NULL DEFAULT 0,
    is_public       boolean NOT NULL DEFAULT true,
    created_at      timestamptz NOT NULL DEFAULT now()
);
```

`0 = unlimited` instead of `u32::MAX`: it survives JSON, it survives `CHECK`, and it is what
the UI already assumes when `max_users` is absent.

### `tenants`

```sql
plan            text    NOT NULL DEFAULT 'free'  REFERENCES subscription_plans(id)
description     text
max_workspaces  integer NOT NULL DEFAULT 10     CHECK (max_workspaces > 0)
max_users       integer NOT NULL DEFAULT 3      CHECK (max_users >= 0)
archived_at     timestamptz
archived_by     uuid
```

`metadata` stays. It is still the home of the model defaults (SPEC-032) and of `primary_color`,
and moving them is a second migration with no benefit to this pack. The **quota** keys become
read-only legacy: `into_tenant` reads the column first, and only falls back to `metadata` when
the column is absent (which cannot happen after 226, but keeps a rollback honest).

### `workspaces`

```sql
archived_at  timestamptz
archived_by  uuid
```

`is_active` keeps meaning "usable"; `archived_at` records the intent and drives quota. A
workspace with `is_active = false AND archived_at IS NULL` is system-disabled (pre-existing
rows) and does **not** free quota — otherwise a disable would silently grant capacity.

### `memberships`

```sql
granted_by  uuid  -- nullable: pre-226 and SSO rows have no human author
```

Nothing else. `workspace_id IS NULL` already means the organization grant, and
`memberships_user_tenant_workspace_uidx UNIQUE NULLS NOT DISTINCT` already prevents duplicates
(added by 164). That constraint is what makes "one person, many grants" work: one row per
(user, tenant, workspace), including the NULL workspace.

### `scope_purge_receipts`

One row per destructive operation, with a `planes` jsonb ledger. Not a log: a **resumable
state machine**. `status` moves `running → done | partial | failed`; a retry reads the ledger
and runs only planes that are not `done`.

## The two triggers

### 1. Workspace quota, serialized

```sql
CREATE OR REPLACE FUNCTION public.eq_workspace_quota_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE t public.tenants; used integer;
BEGIN
    SELECT * INTO t FROM public.tenants WHERE tenant_id = NEW.tenant_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'tenant_missing: %', NEW.tenant_id USING ERRCODE = 'foreign_key_violation';
    END IF;
    SELECT count(*) INTO used
      FROM public.workspaces
     WHERE tenant_id = NEW.tenant_id AND archived_at IS NULL;
    IF used >= t.max_workspaces THEN
        RAISE EXCEPTION 'workspace_quota_reached: % of %', used, t.max_workspaces
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END $$;
```

Why a trigger and not application code: the application has **two** workspace-insert paths
already (`create_workspace`, `setup/initialize`, plus the tenant handler's default workspace).
A trigger covers all three, and covers a psql session.

Why `FOR UPDATE`: two concurrent inserts both read `used = 9` and both succeed without it. With
it, the second blocks until the first commits and then reads `used = 10`.

Cost: one row lock per workspace creation. Workspace creation is human-paced (a few per day per
org). The lock is held for the duration of the insert, microseconds.

Deadlock risk: the trigger takes a lock on `tenants` only. `PUT /tenants` updates that row; a
concurrent `PUT` and create serialize, which is correct. No other path locks `tenants` then
`workspaces` in the opposite order.

### 2. Owner, deferred

```sql
CREATE CONSTRAINT TRIGGER trg_org_keeps_an_owner
    AFTER UPDATE OR DELETE ON public.memberships
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION public.eq_org_keeps_an_owner();
```

Deferred because creation is `tenant → workspace → membership` and the intermediate state is
ownerless by necessity. A non-deferred `AFTER INSERT` would be wrong too: an organization is
allowed to exist for the duration of its own transaction.

It fires on `UPDATE OR DELETE` of memberships, not on `INSERT` of a tenant, so it cannot block
the create. It catches:
- the API demoting the last owner (the guard gives a nicer 409 first),
- a script doing `DELETE FROM memberships WHERE ...`,
- a cascade from `DELETE FROM tenants` — **which must not fire**. Hence the early return when
  the tenant row is gone or archived:

```rust
IF NOT FOUND OR NOT t.is_active OR t.archived_at IS NOT NULL THEN
    RETURN COALESCE(NEW, OLD);   -- archive/delete in progress
END IF;
```

Ordering inside a purge matters: delete the `tenants` row **first** (or set `archived_at`), then
the memberships. Otherwise the trigger fires mid-purge and the delete fails. The purge service
does `archived_at = now()` before touching memberships for exactly this reason.

## Indexes

| Index | Definition | Serves | Not needed |
|-------|-----------|--------|-----------|
| `workspaces_live_tenant_idx` | `(tenant_id, created_at DESC) WHERE archived_at IS NULL` | switcher list, quota count, purge enumeration | full index would include archived rows that no list shows |
| `memberships_tenant_workspace_role_idx` | `(tenant_id, workspace_id, role) WHERE is_active` | directory filter by workspace, effective level | |
| `scope_purge_receipts_scope_idx` | `(scope_kind, scope_id, started_at DESC)` | receipt polling | |

Reused, not duplicated: `idx_tenants_slug`, `memberships_user_tenant_workspace_uidx`,
`idx_workspaces_tenant`, `idx_memberships_user`.

## The `me/access` query, as the database sees it

```sql
-- 1. platform + org grant
SELECT u.role AS platform_role, m.role AS org_role
  FROM users u
  LEFT JOIN memberships m
    ON m.user_id = u.user_id AND m.tenant_id = $1
   AND m.workspace_id IS NULL AND m.is_active
 WHERE u.user_id = $2;

-- 2. every workspace the caller can read, with effective level
SELECT w.workspace_id, w.name, w.slug,
       max(CASE m.role WHEN 'owner' THEN 4 WHEN 'admin' THEN 3
                       WHEN 'member' THEN 2 ELSE 1 END) AS level
  FROM workspaces w
  JOIN memberships m
    ON m.tenant_id = w.tenant_id AND m.user_id = $2 AND m.is_active
   AND (m.workspace_id IS NULL OR m.workspace_id = w.workspace_id)
 WHERE w.tenant_id = $1 AND w.is_active AND w.archived_at IS NULL
 GROUP BY w.workspace_id, w.name, w.slug;

-- 3. quota
SELECT t.max_workspaces, t.max_users,
       (SELECT count(*) FROM workspaces w
         WHERE w.tenant_id = t.tenant_id AND w.archived_at IS NULL) AS ws_used,
       (SELECT count(DISTINCT m.user_id) FROM memberships m
         WHERE m.tenant_id = t.tenant_id AND m.is_active) AS mem_used
  FROM tenants t WHERE t.tenant_id = $1;
```

Three statements, all index-backed, `N+1` avoided by the `GROUP BY`. `EXPLAIN` for 500
workspaces × 3 grants per person: index scan on `workspaces_live_tenant_idx` + nested loop on
`memberships_user_tenant_workspace_uidx`, ~4 ms. Budget in
[10 §9](../10-frontend-architecture.md#9--performance-budget) is 25 ms p95; there is headroom.

## Migration safety checklist

```text
[x] expand-only: adds columns, one table, one index set, two triggers
[x] no column dropped, no type narrowed, no row rewritten except the backfills
[x] backfill order: metadata → plan → floor (never a value the tenant did not behave like)
[x] NOT NULL set after backfill, with a DEFAULT so concurrent inserts cannot fail
[x] FK to subscription_plans added after the rows exist
[x] the owner backfill runs BEFORE the owner trigger is created
[x] triggers created with DROP IF EXISTS + CREATE (idempotent re-run)
[x] lock_class = ddl_access_exclusive, phase = expand
[x] rollback: old binary works, except org create, which is why W1 ships first
```

## Anti-patterns I am removing

| Anti-pattern | Where | Why it hurts |
|--------------|-------|--------------|
| Config in jsonb read with `unwrap_or` | `TenantRow::into_tenant` | no constraint, wrong default, invisible to `\d tenants` |
| `ON CONFLICT DO UPDATE SET x = x` as "get or create" | `pg_create_tenant` | a create that silently returns someone else's row |
| Count-then-insert | `pg_create_workspace:44-56` | a race with no lock |
| Two delete implementations | `tenant_ops.rs` vs `workspace_crud.rs` | one is incomplete, and it is the dangerous one |
| A `UNIQUE(user, tenant, workspace)` that treats NULLs as distinct | fixed by 164 | duplicate organization grants |
| A test-only limit function that production never calls | `edgequake-auth/tenant.rs` | tested rules that are not enforced rules |

## What I would do with more time

- Move the model defaults out of `metadata` into columns (SPEC-032's own debt), so `\d tenants`
  is the complete truth.
- Make the purge a queue job instead of an inline 202, so a 50-workspace organization does not
  hold a request thread.
- Partition `audit_logs` by month (it already declares partitioning intent in 012).

None of these blocks the pack; all three are noted for the next schema cycle.
