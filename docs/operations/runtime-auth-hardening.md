---
title: "Runtime Config and Authentication Hardening"
---

# Runtime Config and Authentication Hardening

> **Product: v0.26.5** · See also: [Docker Quickstart](/docs/operations/docker-quickstart/) (`EDGEQUAKE_DEV_MODE=true` for frictionless demos).

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

## Local development (open API)

`make dev` sets `EDGEQUAKE_DEV_MODE=true` when `DEV_AUTH_ENABLED=false`, disabling auth for frictionless local testing. The [Docker Quickstart](/docs/operations/docker-quickstart/) compose file does the same for container demos — **do not use in production**.

```bash
# Explicit local open API (alternative to make dev defaults)
export EDGEQUAKE_DEV_MODE=true
```

## What changed

- The WebUI now receives runtime config from the server layout rather than depending only on build-time public variables.
- Protected dashboard routes redirect to the login screen when authentication is required.
- The backend now enforces runtime auth flags and master API keys consistently.
- Bootstrap admin creation can be done securely with the configured master API key.

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

### When auth is disabled

- Demo/dev flows remain available.
- Main application screens load without login.

### When auth is enabled

- Direct access to dashboard routes redirects to login.
- Demo login is hidden.
- Authenticated sessions can access the full dashboard.
- Sensitive endpoints require a valid JWT or configured API key.
