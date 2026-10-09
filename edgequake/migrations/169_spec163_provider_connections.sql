-- SPEC-163: tenant-scoped LLM provider connections (expand-only).
CREATE TABLE IF NOT EXISTS public.provider_connections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID,
    slug TEXT NOT NULL,
    display_name TEXT NOT NULL,
    api_shape TEXT NOT NULL,
    locality TEXT NOT NULL DEFAULT 'local',
    base_url TEXT NOT NULL,
    auth_scheme TEXT NOT NULL DEFAULT 'none',
    api_key_ciphertext BYTEA,
    api_key_nonce BYTEA,
    key_id TEXT,
    key_fingerprint TEXT,
    extra_headers_enc BYTEA,
    timeout_secs INT NOT NULL DEFAULT 120,
    allow_private_network BOOLEAN NOT NULL DEFAULT TRUE,
    last_test_at TIMESTAMPTZ,
    last_test_ok BOOLEAN,
    last_test_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, slug)
);

CREATE INDEX IF NOT EXISTS provider_connections_tenant_idx
    ON public.provider_connections (tenant_id);
