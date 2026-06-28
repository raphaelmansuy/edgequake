-- SPEC-027 phase 33: PostgreSQL users lockout columns (identity SSOT alignment).
-- Safe when migration 001 created `users` without SEC-011 fields.

SET search_path = public;

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS failed_login_attempts INT NOT NULL DEFAULT 0;

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS locked_until TIMESTAMPTZ;

COMMENT ON COLUMN users.failed_login_attempts IS
    'Failed login count — synced from KV auth when postgres mode (SPEC-027 SEC-011)';

COMMENT ON COLUMN users.locked_until IS
    'Account lockout expiry — synced from KV auth when postgres mode (SPEC-027 SEC-011)';
