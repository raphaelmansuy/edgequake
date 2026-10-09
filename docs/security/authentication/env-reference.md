---
title: SSO environment reference
description: Every EDGEQUAKE_OIDC variable with its default, plus the runtime API for adding more identity providers.
---

Every SSO setting starts with `EDGEQUAKE_OIDC_`. Defaults are the safest values. `.env.example` mirrors this table.

| Variable | Default | Purpose |
|----------|---------|---------|
| `EDGEQUAKE_OIDC_ENABLED` | `false` | Master switch |
| `EDGEQUAKE_OIDC_ISSUER_URL` | - | Issuer (must equal discovery `issuer`) |
| `EDGEQUAKE_OIDC_CLIENT_ID` / `_CLIENT_SECRET` | - | Confidential client |
| `EDGEQUAKE_OIDC_REDIRECT_URI` | - | `https://<api>/api/v1/auth/oidc/callback` |
| `EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL` | - | SPA landing, e.g. `https://<app>/auth/callback` |
| `EDGEQUAKE_OIDC_KIND` | `generic` | `generic` / `keycloak` / `google` / `entra` / `cognito` (sets defaults only; `azure` and `microsoft` mean `entra`, `aws` means `cognito`) |
| `EDGEQUAKE_OIDC_SLUG` | kind name (`oidc` for generic) | Provider id, used as `?provider=` |
| `EDGEQUAKE_OIDC_DISPLAY_NAME` | `Single sign-on` | Login button label |
| `EDGEQUAKE_OIDC_TRUST_EMAIL` | `false` | Trust `email_verified` |
| `EDGEQUAKE_OIDC_LINK_POLICY` | `never` | `never` / `verified_email` (any other value, including `always`, means `never`) |
| `EDGEQUAKE_OIDC_JIT` | `true` | Auto-provision users |
| `EDGEQUAKE_OIDC_REQUIRE_ORG` | `false` (`true` for keycloak) | Deny logins without org |
| `EDGEQUAKE_OIDC_TENANT_SLUG` | - | Pin the provider to one tenant |
| `EDGEQUAKE_OIDC_SCOPES` | kind default (`profile`) | Comma-separated scopes beyond `openid email`; replaces the kind default |
| `EDGEQUAKE_OIDC_ROLE_CLAIM` / `_ROLE_MAP` | Claim: keycloak `realm_access.roles`, entra `roles`, cognito `cognito:groups`. Map: empty | Map IdP roles to a membership role. `ROLE_MAP` is JSON, such as `{"eq-admin":"admin"}`. |
| `EDGEQUAKE_OIDC_DEFAULT_ROLE` / `_MAX_ROLE` | `member` / `admin` | Default and ceiling |
| `EDGEQUAKE_OIDC_ALLOWED_HD` / `_ALLOWED_TID` | - | Comma-separated Google domains / Entra directory IDs |
| `EDGEQUAKE_STRICT_TENANT_BIND` | `false`; forced on with SSO outside dev mode | Reject token and header tenant mismatches |

More providers at runtime (admin only): `GET/PUT/DELETE /api/v1/admin/identity-providers[/{slug}]`.
Public, secret-free list for the UI: `GET /api/v1/auth/sso/providers`.

Compose overlay (`docker-compose.keycloak.yml`): `KC_PUBLIC_URL`, `EQ_API_PUBLIC_URL`,
`EQ_WEB_PUBLIC_URL`, `EQ_KC_CLIENT_SECRET`, `EQ_KC_DEMO_PASSWORD`, `EQ_KC_SSL_REQUIRED`,
`EDGEQUAKE_KEYCLOAK_IMAGE`.
