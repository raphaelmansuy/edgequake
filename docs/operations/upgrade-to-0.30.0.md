---
title: "Upgrade to EdgeQuake v0.30.0"
description: "Per-release upgrade notes for EdgeQuake v0.30.0: what changed and what to run."
---

# Upgrade to EdgeQuake v0.30.0

> **From:** v0.29.0 · **To:** v0.30.0 · **CD:** GHCR (`edgequake`,
> `edgequake-frontend`, `edgequake-postgres`, `edgequake-keycloak`)

Minor cut: **SPEC-158** enterprise SSO. Schema train moves **163 → 165**.
Run migrate before `/ready` is 200. From ≤0.28.5, migrate to 163 first
([upgrade-to-0.29.0.md](upgrade-to-0.29.0.md)), then to 165 with this cut.

SSO stays off until `EDGEQUAKE_OIDC_ENABLED=true` and the provider env vars
are set. `make dev` remains auth-disabled. Tenants are never created from an
IdP organization claim — create `acme` / `globex` (or your slugs) before login.

**demo.edgequake.com:** this cut installs the 0.30.0 API and Web UI with
**password auth only**. It does not run Keycloak and does not set OIDC env.
Operators who want SSO follow
[Keycloak production configuration](../security/authentication/keycloak-quickstart.md#production-configuration-https-hostname).

## Highlights

| Area | What changed |
|------|----------------|
| Schema | **164** federation tables; **165** `federated_access_jti` |
| Auth | Keycloak Organizations → tenant slug; BFF session; opaque handoff `?code=` |
| Image | `ghcr.io/raphaelmansuy/edgequake-keycloak` on the same tag |
| Documents | List serves KV rows when the 2.5s interactive budget is nearly spent |
| Acc | Same 2026-08-15 medical-mid attestation as 0.29.0; retrieval not re-scored |

## Sequence

```text
# Already on 0.29.0 (schema 163):
EDGEQUAKE_VERSION=0.30.0 docker compose pull
# migrate Job / edgequake migrate (applies 164 then 165), then API
EDGEQUAKE_VERSION=0.30.0 docker compose up -d
```

Keycloak overlay (after `127.0.0.1 keycloak` is in `/etc/hosts`):

```text
make dev-sso
make keycloak-smoke
```

When `:8080` is another product, use `make spec158-proof-e2e` (API `:18080`,
Keycloak `:18081`, WebUI `:13010`).

## Verify

```bash
curl -sf localhost:8080/health | jq '{version, schema}'
# expect version "0.30.0", schema.latest_version 165, pending_count 0
curl -sf localhost:8080/api/v1/auth/sso/providers
# [] until OIDC env is set; non-empty once a provider is configured
```

## Residuals (not release blockers)

- `scripts/keycloak_smoke.py --broker` stops at Keycloak first-broker login
  when the client requests Organizations scopes. Email-link policy is covered
  by the Rust federation tests.
- Live Playwright SSO (`E2E_SSO=1`) is opt-in.
- Unmapped access tokens remain valid until their TTL (LAW-158-10). Logout
  revokes tokens that were recorded in `federated_access_jti`.
