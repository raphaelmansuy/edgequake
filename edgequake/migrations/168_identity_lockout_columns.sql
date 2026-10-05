-- Plain SQLx replay must include the lockout fields used by identity reads.
-- Migration 001 creates users before 007's CREATE TABLE IF NOT EXISTS, and
-- SQLx does not execute the migration engine's 048 support/apply.sql shim.
-- Already-reconciled installations retain their existing values and schema.
ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS failed_login_attempts INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS locked_until TIMESTAMPTZ;
