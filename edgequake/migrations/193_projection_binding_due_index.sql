-- SPEC-164: a provider-scoped projection claim starts at the binding.
--
-- `($4 IS NULL OR EXISTS (binding.provider = $4))` walks
-- `idx_projection_deliveries_due` and throws away every other provider's due
-- rows. The scoped claim looks up the binding, then this index, and stops at
-- the page. Expand only. Does not raise the serving floor. Apache AGE stays
-- dropped at migration 171.

CREATE INDEX IF NOT EXISTS idx_projection_deliveries_binding_due
    ON public.projection_deliveries (binding_id, next_attempt_at, event_id)
    WHERE state IN ('pending', 'retry');
