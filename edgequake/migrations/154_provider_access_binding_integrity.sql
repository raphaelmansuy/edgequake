-- ============================================================================
-- Migration 154: P0 binding integrity and scoped delivery guards
-- SPEC-149 production-correct hot path
-- ============================================================================
-- Additive only. Does not rewrite migrations 150-153. Enforces that
-- projection deliveries/visibility/cleanup intents stay within the same
-- tenant/workspace as their event and binding via BEFORE INSERT/UPDATE triggers
-- (CHECK constraints cannot reference other tables).

SET search_path = public;

CREATE OR REPLACE FUNCTION public.provider_access_reject_cross_scope_delivery()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    event_tenant UUID;
    event_workspace UUID;
    binding_tenant UUID;
    binding_workspace UUID;
BEGIN
    SELECT tenant_id, workspace_id INTO event_tenant, event_workspace
    FROM public.projection_events WHERE event_id = NEW.event_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'projection delivery references missing event %', NEW.event_id;
    END IF;
    SELECT tenant_id, workspace_id INTO binding_tenant, binding_workspace
    FROM public.data_bindings WHERE binding_id = NEW.binding_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'projection delivery references missing binding %', NEW.binding_id;
    END IF;
    IF event_tenant IS DISTINCT FROM binding_tenant
       OR event_workspace IS DISTINCT FROM binding_workspace THEN
        RAISE EXCEPTION
            'cross-scope projection delivery rejected: event % vs binding %',
            NEW.event_id, NEW.binding_id;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_projection_deliveries_same_scope
    ON public.projection_deliveries;
CREATE TRIGGER trg_projection_deliveries_same_scope
    BEFORE INSERT OR UPDATE OF event_id, binding_id
    ON public.projection_deliveries
    FOR EACH ROW
    EXECUTE FUNCTION public.provider_access_reject_cross_scope_delivery();

CREATE OR REPLACE FUNCTION public.provider_access_reject_cross_scope_visibility()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    binding_tenant UUID;
    binding_workspace UUID;
BEGIN
    SELECT tenant_id, workspace_id INTO binding_tenant, binding_workspace
    FROM public.data_bindings WHERE binding_id = NEW.binding_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'projection visibility references missing binding %', NEW.binding_id;
    END IF;
    IF NEW.tenant_id IS DISTINCT FROM binding_tenant
       OR NEW.workspace_id IS DISTINCT FROM binding_workspace THEN
        RAISE EXCEPTION
            'cross-scope projection visibility rejected for binding %',
            NEW.binding_id;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_projection_visibility_same_scope
    ON public.projection_visibility;
CREATE TRIGGER trg_projection_visibility_same_scope
    BEFORE INSERT OR UPDATE OF tenant_id, workspace_id, binding_id
    ON public.projection_visibility
    FOR EACH ROW
    EXECUTE FUNCTION public.provider_access_reject_cross_scope_visibility();

CREATE OR REPLACE FUNCTION public.provider_access_reject_cross_scope_cleanup()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    binding_tenant UUID;
    binding_workspace UUID;
BEGIN
    SELECT tenant_id, workspace_id INTO binding_tenant, binding_workspace
    FROM public.data_bindings WHERE binding_id = NEW.binding_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'cleanup intent references missing binding %', NEW.binding_id;
    END IF;
    IF NEW.tenant_id IS DISTINCT FROM binding_tenant
       OR NEW.workspace_id IS DISTINCT FROM binding_workspace THEN
        RAISE EXCEPTION
            'cross-scope cleanup intent rejected for binding %',
            NEW.binding_id;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_projection_cleanup_intents_same_scope
    ON public.projection_cleanup_intents;
CREATE TRIGGER trg_projection_cleanup_intents_same_scope
    BEFORE INSERT OR UPDATE OF tenant_id, workspace_id, binding_id
    ON public.projection_cleanup_intents
    FOR EACH ROW
    EXECUTE FUNCTION public.provider_access_reject_cross_scope_cleanup();

CREATE INDEX IF NOT EXISTS idx_data_bindings_scope_role_state
    ON public.data_bindings (tenant_id, workspace_id, role, state);
