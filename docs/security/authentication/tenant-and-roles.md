---
title: Tenants, organizations and roles
description: How a login is mapped to a tenant and a role, the denial codes, account linking and lifecycle rules.
---

This page explains how EdgeQuake turns an IdP login into a tenant and a role. The mapping is strict so that an IdP can never give a user more access than you allow.

## Tenant resolution (first match wins)

1. Provider `EDGEQUAKE_OIDC_TENANT_SLUG` (fixed tenant) - every login of that provider lands there.
2. The IdP **organization claim** (Keycloak: `organization`, accepts array / map / string / object
   shapes; aliases are case-insensitive). A requested `?org=` must be asserted by the token.
3. No claim: `EDGEQUAKE_OIDC_REQUIRE_ORG=true` denies (`org_missing`, default for `kind=keycloak`);
   otherwise the default tenant is used.

Several organizations and no hint -> `org_ambiguous:<aliases>` (the UI shows a picker).

## Denial codes

A denied login returns to the web app as `{SPA}/auth/callback?error=<code>`, and the app shows a translated message.

| Code | Meaning |
|------|---------|
| `org_unknown` | No tenant has a slug equal to the organization alias |
| `org_missing` | The token has no organization and one is required |
| `org_ambiguous` | Several organizations and no hint |
| `tenant_suspended` | The tenant is suspended |
| `hd_mismatch` | Google domain not on `EDGEQUAKE_OIDC_ALLOWED_HD` |
| `idp_tenant_not_allowed` | Entra directory not on `EDGEQUAKE_OIDC_ALLOWED_TID` |
| `jit_disabled` | New users are not auto-created (`EDGEQUAKE_OIDC_JIT=false`) |
| `max_users` | The tenant is full |
| `membership_revoked` | The user's membership was removed |
| `tenant_access_denied` | The user may not use this tenant |
| `account_exists_unlinked` | A local account has the same email (HTTP 409) |

## Roles

`EDGEQUAKE_OIDC_ROLE_CLAIM` (default `realm_access.roles` for Keycloak) is mapped through
`EDGEQUAKE_OIDC_ROLE_MAP`, capped by `EDGEQUAKE_OIDC_MAX_ROLE` and defaulting to
`EDGEQUAKE_OIDC_DEFAULT_ROLE`. The IdP can never grant more than `max_role`. Roles are **resynced
at every login and on refresh**, but only for memberships created by SSO (`metadata.source="sso"`);
manually managed memberships are never rewritten. The **last owner is never demoted**.
A local platform admin keeps platform rights independent of tenant roles.

## Account linking

By default a federated identity creates its own user. Linking to an existing local account by
email requires **all** of: `LINK_POLICY=verified_email`, `TRUST_EMAIL=true`, the IdP asserting
`email_verified`, and a non-synthetic email. Otherwise the login is refused with
`account_exists_unlinked`. Keep `TRUST_EMAIL` off for GitHub.

## Capacity and lifecycle

`max_users` on the tenant is enforced at JIT time. Suspending a tenant or removing a membership
takes effect at the next refresh (`tenant_suspended` / `membership_revoked`, HTTP 403), and a
back-channel logout revokes the session family immediately.
