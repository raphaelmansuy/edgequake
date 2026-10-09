---
title: Microsoft Entra ID
description: Connect a Microsoft Entra ID directory to EdgeQuake over OIDC, with the directory allow-list.
---

1. In Entra, open App registrations and create a Web app. Set the redirect URI to `https://<api>/api/v1/auth/oidc/callback`, choose the account types your policy allows, and create a client secret.
2. Configure the v2.0 issuer of your directory:

```bash
EDGEQUAKE_OIDC_ENABLED=true
EDGEQUAKE_OIDC_KIND=entra
EDGEQUAKE_OIDC_SLUG=microsoft
EDGEQUAKE_OIDC_ISSUER_URL=https://login.microsoftonline.com/<tenant-guid>/v2.0
EDGEQUAKE_OIDC_CLIENT_ID=<application-id>
EDGEQUAKE_OIDC_CLIENT_SECRET=...
EDGEQUAKE_OIDC_REDIRECT_URI=https://<api>/api/v1/auth/oidc/callback
EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL=https://<app>/auth/callback
EDGEQUAKE_OIDC_ALLOWED_TID=<tenant-guid>      # allow-list of directories (tid claim)
EDGEQUAKE_OIDC_TENANT_SLUG=contoso
```

Notes: Entra has no organization claim, so `EDGEQUAKE_OIDC_TENANT_SLUG` pins every login to one EdgeQuake tenant. Do not use the multi-tenant `common` authority without an `EDGEQUAKE_OIDC_ALLOWED_TID` allow-list; a directory that is not on the list is denied with `idp_tenant_not_allowed`. Guest users keep their home `tid`, so they are denied unless that directory is listed.
