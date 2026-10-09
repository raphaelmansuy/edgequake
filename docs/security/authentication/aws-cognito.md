---
title: AWS Cognito
description: Connect an AWS Cognito user pool to EdgeQuake over OIDC, and how to use IAM Identity Center through Keycloak.
---

Create a Cognito user pool and an app client. Use a confidential client, the authorization-code grant, and the scopes `openid email profile`. Then set:

```bash
EDGEQUAKE_OIDC_ENABLED=true
EDGEQUAKE_OIDC_KIND=cognito
EDGEQUAKE_OIDC_SLUG=aws
EDGEQUAKE_OIDC_ISSUER_URL=https://cognito-idp.<region>.amazonaws.com/<pool-id>
EDGEQUAKE_OIDC_CLIENT_ID=...
EDGEQUAKE_OIDC_CLIENT_SECRET=...
EDGEQUAKE_OIDC_REDIRECT_URI=https://<api>/api/v1/auth/oidc/callback
EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL=https://<app>/auth/callback
EDGEQUAKE_OIDC_ROLE_CLAIM=cognito:groups
EDGEQUAKE_OIDC_ROLE_MAP={"eq-admin":"admin"}
EDGEQUAKE_OIDC_TENANT_SLUG=acme
```

Cognito has no organization claim, so `EDGEQUAKE_OIDC_TENANT_SLUG` pins every login to one tenant. Groups arrive in the `cognito:groups` claim and are mapped through `EDGEQUAKE_OIDC_ROLE_MAP` (see [Tenants, organizations and roles](tenant-and-roles.md)).

For IAM Identity Center, connect it to Keycloak as a SAML or OIDC identity provider and point EdgeQuake at Keycloak. This keeps organizations mapped to tenants.

All variables are listed in the [SSO environment reference](env-reference.md).
