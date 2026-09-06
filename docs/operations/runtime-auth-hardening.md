---
title: "Runtime Config and Authentication Hardening"
---

# Runtime Config and Authentication Hardening

> **Product: v0.26.5** · See also: [Docker Quickstart](/docs/operations/docker-quickstart/) (`EDGEQUAKE_DEV_MODE=true` for frictionless **container** demos).

EdgeQuake supports both demo-friendly local development and fail-closed authenticated deployments.

## Recommended production environment

```bash
export EDGEQUAKE_AUTH_ENABLED=true
# Do NOT set EDGEQUAKE_DEV_MODE in production
export EDGEQUAKE_MASTER_API_KEY="replace-with-a-strong-secret"
export NEXT_PUBLIC_AUTH_ENABLED=true
export NEXT_PUBLIC_DISABLE_DEMO_LOGIN=true
export NEXT_PUBLIC_API_URL="https://your-api-host"
```

## Local development (`make dev`)

`make dev` / `make dev-bg` start with **authentication enabled** and a pinned local account:

| Field | Value |
|-------|--------|
| Username | `admin` |
| Password | `EdgeQuake1` |

Credentials are printed in the Makefile startup banner and shown on the WebUI `/login` screen (`NEXT_PUBLIC_SHOW_DEV_LOGIN_HINT=true`). The backend pins the password on every boot via `EDGEQUAKE_DEV_PIN_LOGIN=1` (local `DATABASE_URL` only).

```bash
make dev          # auth on — admin / EdgeQuake1
make dev-open     # escape hatch: open API (EDGEQUAKE_DEV_MODE=true)
```

The [Docker Quickstart](/docs/operations/docker-quickstart/) compose file still defaults to open API for container demos — **do not use `EDGEQUAKE_DEV_MODE=true` in production**.

## What changed

- The WebUI now receives runtime config from the server layout rather than depending only on build-time public variables.
- Protected dashboard routes redirect to the login screen when authentication is required.
- The backend now enforces runtime auth flags and master API keys consistently.
- Bootstrap admin creation can be done securely with the configured master API key.
- Local `make dev` always requires login with fixed credentials (closes the anonymous/auth gap for SPEC-146).

## Bootstrap an admin user

When authentication is enabled (`EDGEQUAKE_AUTH_ENABLED=true`, the v0.15 default) and no
login-capable users exist in PostgreSQL, set bootstrap credentials **before first API start**:

```bash
export EDGEQUAKE_BOOTSTRAP_ADMIN_USERNAME=admin
export EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD='ChangeMe123!'
export EDGEQUAKE_BOOTSTRAP_ADMIN_EMAIL=admin@example.com
export NEXT_PUBLIC_AUTH_ENABLED=true
export NEXT_PUBLIC_DISABLE_DEMO_LOGIN=true
```

The API creates the admin automatically on startup (GitHub #288). Upgrades from pre-v0.15 KV
identity also import legacy `auth:user:*` rows into PostgreSQL when present.

Alternatively, bootstrap manually with the master API key:

```bash
curl -X POST http://localhost:8080/api/v1/users \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $EDGEQUAKE_MASTER_API_KEY" \
  -d '{
    "username": "admin",
    "email": "admin@example.com",
    "password": "ChangeMe123!",
    "role": "admin"
  }'
```

## Expected behavior

### When auth is disabled (`make dev-open` / Docker quickstart)

- Demo/dev flows remain available.
- Main application screens load without login.
- Login page may show “Continue without login (Demo)”.

### When auth is enabled (`make dev` / production)

- Direct access to dashboard routes redirects to login.
- Demo skip-login is hidden.
- Local `make dev` shows the fixed credentials on `/login`.
- Authenticated sessions can access the full dashboard.
- Sensitive endpoints require a valid JWT or configured API key.

## Document ABAC — worker DB role (SPEC-146 G6 / G-146-53)

When `EDGEQUAKE_DOC_ABAC=1`, the **ingestion / pipeline worker** must not be able to read broad document or chunk content via SQL, even if a process bug tries to query.

**Ops requirement:**

1. Claim the worker as `PrincipalId::Worker` (allow-set is always empty for query paths — LAW-146-13).
2. Provision a dedicated PostgreSQL role for workers (e.g. `edgequake_worker`) that:
   - **May** insert/update pipeline tables, task/outbox rows, and write-path columns needed for ingestion.
   - **Must not** have broad `SELECT` on `documents`, `chunks`, chunk embedding tables, or graph text that would let a compromised worker exfiltrate the corpus.
3. Prefer column/table grants scoped to write + status columns; deny `SELECT` on body / markdown / embedding payload where feasible.
4. Application PEPs alone are not enough if the worker DB credential can `SELECT * FROM documents`.

Break-glass sessions (master elevated access) use a **default TTL of 15 minutes**, hard-capped at **60 minutes** (LAW-146-25). Create/revoke are audited via `edgequake-audit` `Authorization` events.
