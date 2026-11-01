-- SPEC-181: admin — organization/workspace creation, grants, quota, archive, purge receipts.
--
-- Expand-only. Every statement is idempotent and additive. No column is dropped, no type
-- narrowed, no row rewritten except the backfills below. The previous binary keeps working,
-- with one deliberate exception documented in specs/181-admin/07-data-and-db-contract.md §6:
-- the owner trigger refuses an organization with no owner, which is the bug this pack fixes,
-- so the application-side owner grant must ship before this file is applied.
--
-- WHY the quota columns: `tenants` kept plan/max_workspaces/max_users in `metadata` jsonb,
-- read with `unwrap_or(5)`. A jsonb key cannot be constrained, cannot be inspected with
-- `\d tenants`, and its fallback contradicted the plan it claimed to read (SPEC-181 F-181-06).

SET search_path = public;

-- ===========================================================================
-- 1. Plan catalog
-- ===========================================================================

CREATE TABLE IF NOT EXISTS subscription_plans (
    id              text PRIMARY KEY,
    label           text NOT NULL,
    max_workspaces  integer NOT NULL CHECK (max_workspaces > 0),
    -- 0 = unlimited. Chosen over u32::MAX because it round-trips through JSON and a CHECK.
    max_users       integer NOT NULL CHECK (max_users >= 0),
    sort_order      integer NOT NULL DEFAULT 0,
    is_public       boolean NOT NULL DEFAULT true,
    created_at      timestamptz NOT NULL DEFAULT now()
);

INSERT INTO subscription_plans (id, label, max_workspaces, max_users, sort_order)
VALUES ('free',       'Free',        10,  3, 1),
       ('basic',      'Basic',      100, 10, 2),
       ('pro',        'Pro',        500, 50, 3),
       ('enterprise', 'Enterprise', 500,  0, 4)
ON CONFLICT (id) DO NOTHING;

-- ===========================================================================
-- 2. Real columns on tenants
-- ===========================================================================

ALTER TABLE public.tenants
    ADD COLUMN IF NOT EXISTS plan            text,
    ADD COLUMN IF NOT EXISTS description     text,
    ADD COLUMN IF NOT EXISTS max_workspaces  integer,
    ADD COLUMN IF NOT EXISTS max_users       integer,
    ADD COLUMN IF NOT EXISTS archived_at     timestamptz,
    ADD COLUMN IF NOT EXISTS archived_by     uuid;

-- Backfill order matters: metadata first (the value the tenant already behaved like), then
-- the plan catalog, then a floor. A tenant never silently changes quota during the upgrade.
UPDATE public.tenants t
   SET plan = COALESCE(NULLIF(t.metadata->>'plan', ''), 'free')
 WHERE t.plan IS NULL;

UPDATE public.tenants t
   SET description = t.metadata->>'description'
 WHERE t.description IS NULL AND t.metadata ? 'description';

UPDATE public.tenants t
   SET max_workspaces = COALESCE(
         NULLIF((t.metadata->>'max_workspaces')::numeric::integer, NULL),
         (SELECT p.max_workspaces FROM subscription_plans p WHERE p.id = t.plan),
         5)
 WHERE t.max_workspaces IS NULL;

UPDATE public.tenants t
   SET max_users = COALESCE(
         NULLIF((t.metadata->>'max_users')::numeric::integer, NULL),
         (SELECT p.max_users FROM subscription_plans p WHERE p.id = t.plan),
         3)
 WHERE t.max_users IS NULL;

-- Any plan id that predates the catalog is not a valid FK target.
UPDATE public.tenants SET plan = 'free' WHERE plan NOT IN (SELECT id FROM subscription_plans);

ALTER TABLE public.tenants
    ALTER COLUMN plan           SET DEFAULT 'free',
    ALTER COLUMN plan           SET NOT NULL,
    ALTER COLUMN max_workspaces SET DEFAULT 10,
    ALTER COLUMN max_workspaces SET NOT NULL,
    ALTER COLUMN max_users      SET DEFAULT 3,
    ALTER COLUMN max_users      SET NOT NULL;

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint
                    WHERE conname = 'tenants_plan_fk'
                      AND conrelid = 'public.tenants'::regclass) THEN
        ALTER TABLE public.tenants
            ADD CONSTRAINT tenants_plan_fk FOREIGN KEY (plan)
            REFERENCES public.subscription_plans(id) ON UPDATE CASCADE ON DELETE RESTRICT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint
                    WHERE conname = 'tenants_quota_positive'
                      AND conrelid = 'public.tenants'::regclass) THEN
        ALTER TABLE public.tenants
            ADD CONSTRAINT tenants_quota_positive
            CHECK (max_workspaces > 0 AND max_users >= 0);
    END IF;
END $$;

-- ===========================================================================
-- 3. Archive on workspaces
-- ===========================================================================
-- `is_active` keeps meaning "usable"; `archived_at` records the intent and drives quota.
-- A workspace with is_active = false AND archived_at IS NULL is system-disabled and does
-- NOT free quota — otherwise a disable would silently grant capacity.

ALTER TABLE public.workspaces
    ADD COLUMN IF NOT EXISTS archived_at timestamptz,
    ADD COLUMN IF NOT EXISTS archived_by uuid;

CREATE INDEX IF NOT EXISTS workspaces_live_tenant_idx
    ON public.workspaces (tenant_id, created_at DESC)
    WHERE archived_at IS NULL;

-- ===========================================================================
-- 4. Grant provenance
-- ===========================================================================

ALTER TABLE public.memberships
    ADD COLUMN IF NOT EXISTS granted_by uuid;

CREATE INDEX IF NOT EXISTS memberships_tenant_workspace_role_idx
    ON public.memberships (tenant_id, workspace_id, role)
    WHERE is_active;

-- ===========================================================================
-- 5. Purge receipts (resumable state machine, not a log)
-- ===========================================================================

CREATE TABLE IF NOT EXISTS scope_purge_receipts (
    receipt_id   uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    scope_kind   text NOT NULL CHECK (scope_kind IN ('workspace', 'organization')),
    scope_id     uuid NOT NULL,
    tenant_id    uuid NOT NULL,
    status       text NOT NULL DEFAULT 'running'
                 CHECK (status IN ('running', 'done', 'partial', 'failed')),
    planes       jsonb NOT NULL DEFAULT '{}'::jsonb,
    requested_by uuid,
    started_at   timestamptz NOT NULL DEFAULT now(),
    finished_at  timestamptz
);

CREATE INDEX IF NOT EXISTS scope_purge_receipts_scope_idx
    ON scope_purge_receipts (scope_kind, scope_id, started_at DESC);

-- At most one running receipt per scope, enforced by the database rather than by a check
-- in the handler: two concurrent deletes must not both start purging.
CREATE UNIQUE INDEX IF NOT EXISTS scope_purge_receipts_running_uq
    ON scope_purge_receipts (scope_kind, scope_id)
    WHERE status = 'running';

-- ===========================================================================
-- 6. Owner backfill — runs BEFORE the owner trigger exists
-- ===========================================================================
-- A live organization with no owner predates this pack and cannot be governed by anyone but a
-- platform admin. Giving it the bootstrap admin as Owner is the only non-destructive repair.
-- Logged so an operator can see which organizations were repaired.

DO $$
DECLARE repaired integer := 0; t record; u uuid;
BEGIN
    SELECT user_id INTO u FROM public.users
     WHERE role = 'admin' AND is_active
     ORDER BY created_at LIMIT 1;

    IF u IS NULL THEN
        RAISE NOTICE 'SPEC-181: no active platform admin; ownerless organizations left as-is';
        RETURN;
    END IF;

    FOR t IN
        SELECT tenant_id, slug FROM public.tenants
         WHERE is_active AND archived_at IS NULL
           AND NOT EXISTS (SELECT 1 FROM public.memberships m
                            WHERE m.tenant_id = tenants.tenant_id
                              AND m.workspace_id IS NULL
                              AND m.role = 'owner' AND m.is_active)
    LOOP
        INSERT INTO public.memberships
            (membership_id, tenant_id, workspace_id, user_id, role, is_active, joined_at, metadata)
        VALUES (gen_random_uuid(), t.tenant_id, NULL, u, 'owner', true, now(),
                jsonb_build_object('source', 'spec181_backfill'));
        repaired := repaired + 1;
        RAISE NOTICE 'SPEC-181: repaired ownerless organization %', t.slug;
    END LOOP;

    IF repaired > 0 THEN
        RAISE NOTICE 'SPEC-181: repaired % ownerless organization(s)', repaired;
    END IF;
END $$;

-- ===========================================================================
-- 7. Trigger: an active organization always has an owner
-- ===========================================================================
-- Deferred because creation is tenant -> workspace -> membership, and the intermediate state
-- is ownerless by necessity. Fires on UPDATE/DELETE of memberships only, so it cannot block
-- the create. Returns early while the tenant is being archived or deleted, which is why the
-- purge path sets archived_at before touching memberships.

CREATE OR REPLACE FUNCTION public.eq_org_keeps_an_owner() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE t public.tenants;
BEGIN
    SELECT * INTO t FROM public.tenants
     WHERE tenant_id = COALESCE(OLD.tenant_id, NEW.tenant_id);
    IF NOT FOUND OR NOT t.is_active OR t.archived_at IS NOT NULL THEN
        RETURN COALESCE(NEW, OLD);
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM public.memberships m
         WHERE m.tenant_id = t.tenant_id
           AND m.workspace_id IS NULL
           AND m.role = 'owner'
           AND m.is_active
    ) THEN
        RAISE EXCEPTION 'organization_requires_owner: % would have no owner', t.tenant_id
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN COALESCE(NEW, OLD);
END $$;

DROP TRIGGER IF EXISTS trg_org_keeps_an_owner ON public.memberships;
CREATE CONSTRAINT TRIGGER trg_org_keeps_an_owner
    AFTER UPDATE OR DELETE ON public.memberships
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION public.eq_org_keeps_an_owner();

-- ===========================================================================
-- 8. Trigger: workspace quota is serialized
-- ===========================================================================
-- The application has three workspace-insert paths (create_workspace, setup/initialize, the
-- tenant handler's default workspace). A trigger covers all three, and covers a psql session.
-- FOR UPDATE on the tenant row is what makes two concurrent creates produce one 409 instead
-- of two successes.

CREATE OR REPLACE FUNCTION public.eq_workspace_quota_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE t public.tenants; used integer;
BEGIN
    SELECT * INTO t FROM public.tenants WHERE tenant_id = NEW.tenant_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'tenant_missing: %', NEW.tenant_id
            USING ERRCODE = 'foreign_key_violation';
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

DROP TRIGGER IF EXISTS trg_workspace_quota_guard ON public.workspaces;
CREATE TRIGGER trg_workspace_quota_guard
    BEFORE INSERT ON public.workspaces
    FOR EACH ROW EXECUTE FUNCTION public.eq_workspace_quota_guard();
