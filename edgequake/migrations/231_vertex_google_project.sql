-- A saved Vertex connection can name its Google Cloud project.
--
-- Queries were still reading GOOGLE_CLOUD_PROJECT from the process environment,
-- so a deployment that stored the provider (and even passed its health check via
-- the server identity) failed every question with "GOOGLE_CLOUD_PROJECT is not set".
-- The project id is not a secret. It is stored beside the connection and used
-- when the client is built. Expand-only: one nullable column, no backfill.

SET search_path = public;

ALTER TABLE public.provider_connections
    ADD COLUMN IF NOT EXISTS google_project TEXT;
