---
title: Google
description: Connect Google sign-in to EdgeQuake directly over OIDC or through Keycloak.
---

There are two routes. Prefer the Keycloak Google identity provider, because organization mapping then lives in one place.

## Direct OIDC

1. In the Google Cloud console, open APIs and Services, Credentials, and create a Web OAuth client. Set the authorized redirect URI to `https://<api>/api/v1/auth/oidc/callback`.
2. Configure:

```bash
EDGEQUAKE_OIDC_ENABLED=true
EDGEQUAKE_OIDC_KIND=google
EDGEQUAKE_OIDC_SLUG=google
EDGEQUAKE_OIDC_ISSUER_URL=https://accounts.google.com
EDGEQUAKE_OIDC_CLIENT_ID=...apps.googleusercontent.com
EDGEQUAKE_OIDC_CLIENT_SECRET=...
EDGEQUAKE_OIDC_REDIRECT_URI=https://<api>/api/v1/auth/oidc/callback
EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL=https://<app>/auth/callback
EDGEQUAKE_OIDC_ALLOWED_HD=example.com        # Workspace hosted domain(s)
EDGEQUAKE_OIDC_TENANT_SLUG=example           # Google has no org claim: pin one tenant
```

Notes: a user is identified by the `sub` claim, never by email. The `hd` claim must match `EDGEQUAKE_OIDC_ALLOWED_HD` or the login is denied with `hd_mismatch`. Consumer `@gmail.com` accounts have no `hd`, so they are denied when the allow-list is set.

## Via Keycloak

Add Google as an identity provider in the realm and attach users to an Organization (by email domain). EdgeQuake then sees a normal Keycloak login with the `organization` claim. See [Keycloak quickstart](keycloak-quickstart.md).
