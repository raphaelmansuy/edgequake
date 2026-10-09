---
title: Keycloak quickstart
description: Run EdgeQuake with the shipped Keycloak on one machine, create tenants, sign in, then move to a production HTTPS setup.
---

This page runs EdgeQuake and the shipped Keycloak (realm `edgequake`, Keycloak 26.8.0 or later) on one machine, then lists the settings for a production HTTPS setup.

## 1. One-time host setup

The browser and the API container must resolve the **same issuer host** (the issuer string is
compared byte-for-byte):

```bash
echo "127.0.0.1 keycloak" | sudo tee -a /etc/hosts
```

## 2. Start

```bash
make dev-sso            # docker-compose.quickstart.yml + docker-compose.keycloak.yml
make keycloak-smoke     # headless auth-code + PKCE login, org claim, roles, API handoff
```

| Service | URL | Credentials |
|---------|-----|-------------|
| Keycloak admin | http://keycloak:8081 | `admin` / `$KC_ADMIN_PASSWORD` (dev default in the overlay) |
| EdgeQuake API | http://localhost:8080 | break-glass `admin` / `$EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD` |
| Demo users | realm `edgequake` | `alice` (acme, admin), `bob` (globex), `carol` (acme + globex) / `$EQ_KC_DEMO_PASSWORD` |

All defaults in the overlay are **dev-only**; override every `EQ_*` / `KC_*` secret outside a laptop.

`KEYCLOAK_SMOKE_DEEP=1 make keycloak-smoke` also proves **back-channel logout** (Keycloak admin
logout → signed logout token → EdgeQuake access and refresh 401). The Keycloak container must
reach the API at `EQ_API_INTERNAL_URL` (`http://api:8080` on the overlay network, or
`http://host.docker.internal:18080` for the isolated proof stack). Admin REST is called at
`http://127.0.0.1:<port>` so HTTP is allowed without editing `sslRequired` on master.
When `:8080` is another product, use `make spec158-proof-e2e` (Keycloak `:18081`, API `:18080`,
WebUI `:13010`).

## 3. Create the tenants

Organizations map to `tenants.slug` and are **never auto-created** (an unknown org is denied with
`org_unknown`). Create them with the break-glass admin (the smoke target does this for you):

```bash
TOKEN=$(curl -s localhost:8080/api/v1/auth/login -H 'content-type: application/json' \
  -d '{"username":"admin","password":"'"$EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD"'"}' | jq -r .access_token)
for org in acme globex; do
  curl -s localhost:8080/api/v1/tenants -H "authorization: Bearer $TOKEN" \
    -H 'content-type: application/json' -d '{"name":"'$org'","slug":"'$org'"}'
done
```

## 4. Sign in

Open the web UI login page: **Continue with Single sign-on**, optionally typing the organization
alias (`acme`). Users in several organizations without a hint get a picker (`org_ambiguous`).

## What the realm contains

- Organizations enabled; confidential client `edgequake-web` with PKCE `S256`.
- Realm roles `eq-user`, `eq-admin`, `eq-owner` exposed as `realm_access.roles` in the ID token.
- Back-channel logout to `/api/v1/auth/oidc/backchannel-logout` (session required).
- Realm import is **create-only**: an existing realm is skipped, so edit live realms in the Admin
  console or via `kcadm`, not by changing the JSON.

## Same-origin note

The OIDC state cookie is set by the API on the host used to start login, so the login URL host
must equal the host of `EDGEQUAKE_OIDC_REDIRECT_URI`. With the dev web UI proxying `/api` on
`:3000`, set `EQ_API_PUBLIC_URL=http://localhost:3000`; with a direct API URL, use that URL.

## Production configuration (HTTPS hostname)

Use this when Keycloak and EdgeQuake have public names. The laptop overlay
(`EQ_KC_SSL_REQUIRED=none`, demo users `alice`/`bob`/`carol`) must not ship.

1. Run Keycloak >= 26.8.0 with `KC_HOSTNAME` set to the public host. Pin
   `ghcr.io/raphaelmansuy/edgequake-keycloak:<version>` (or your own realm).
   Realm import is **create-only**; edit a live realm in the Admin console or
   with `kcadm`.
2. Set `sslRequired=external` (`EQ_KC_SSL_REQUIRED=external` on the overlay).
3. Create EdgeQuake tenants whose **slugs match Keycloak Organization aliases**
   before anyone signs in. An unknown org is `org_unknown`; tenants are never
   created from a claim.
4. Point the API at the issuer **byte-for-byte** (discovery `issuer` string):

```bash
EDGEQUAKE_AUTH_ENABLED=true
EDGEQUAKE_STRICT_TENANT_BIND=true
EDGEQUAKE_OIDC_ENABLED=true
EDGEQUAKE_OIDC_KIND=keycloak
EDGEQUAKE_OIDC_SLUG=keycloak
EDGEQUAKE_OIDC_DISPLAY_NAME="Sign in with SSO"
EDGEQUAKE_OIDC_ISSUER_URL=https://sso.example.com/realms/edgequake
EDGEQUAKE_OIDC_CLIENT_ID=edgequake-web
EDGEQUAKE_OIDC_CLIENT_SECRET=<from secret manager>
EDGEQUAKE_OIDC_REDIRECT_URI=https://app.example.com/api/v1/auth/oidc/callback
EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL=https://app.example.com/auth/callback
EDGEQUAKE_OIDC_ROLE_MAP='{"eq-owner":"owner","eq-admin":"admin","eq-user":"member"}'
```

`REDIRECT_URI` and `SUCCESS_REDIRECT_URL` must share the **same public host**
as the page that starts `GET /api/v1/auth/oidc/login`. Allow Keycloak to
`POST /api/v1/auth/oidc/backchannel-logout`.

Helm overlay: [values-sso-keycloak.yaml.example](../../../deploy/kubernetes/helm/edgequake/values-sso-keycloak.yaml.example).
Env table: [env-reference.md](env-reference.md). Gates: [production-hardening.md](production-hardening.md).
