---
title: "Upgrade to EdgeQuake v0.28.4"
description: "Per-release upgrade notes for EdgeQuake v0.28.4: what changed and what to run."
---

# Upgrade to EdgeQuake v0.28.4

> **From:** v0.28.3 · **To:** v0.28.4 · **CD:** GHCR (`edgequake`,
> `edgequake-frontend`, `edgequake-postgres`)

Patch cut: **SPEC-154** auth hardening (refresh family, durable jti, WS
protocol JWT, scoped env API keys, HttpOnly refresh cookie). Schema train
moves **160 → 162**. Run migrate before relying on revoke / family rotation.

## Highlights

| Area | What changed |
|------|----------------|
| Schema | **162** — `jwt_jti_denylist`; refresh `family_id` / status |
| Auth | Access TTL default **900s**; SPA cookie refresh; no WS `?token=` |
| API keys | `EDGEQUAKE_API_KEYS` = read+query; `master_api_key` = break-glass |
| Startup | Auth-off + non-local DB is **Fatal** |

## Sequence

```text
# Compose / Helm: migrate Job then API
docker compose pull
EDGEQUAKE_VERSION=0.28.4 docker compose up -d
# or: edgequake migrate && restart API

# WebUI clients: clear any pre-154 localStorage access/refresh keys once
```

## Verify

```bash
curl -sf localhost:8080/health | jq '{version, schema}'
# expect version "0.28.4", schema.latest_version 162, pending_count 0
```

## Notes

- Residual: Next may set non-HttpOnly `edgequake_access_token` for middleware
  (Secure on HTTPS). Refresh remains HttpOnly `eq_refresh`.
- See [docs/security/best-practices.md](../security/best-practices.md) SPEC-154.
