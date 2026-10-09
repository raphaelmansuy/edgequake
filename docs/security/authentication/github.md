---
title: GitHub
description: Why GitHub cannot connect directly and how to sign in with GitHub through a Keycloak identity provider.
---

GitHub speaks OAuth2 only and does not issue an OIDC ID token. EdgeQuake therefore does not accept it directly. Connect GitHub to Keycloak and let EdgeQuake use Keycloak (decision recorded in SPEC-158).

1. In GitHub, open Settings, Developer settings, OAuth Apps, New. Set the callback URL to `https://<keycloak>/realms/edgequake/broker/github/endpoint`.
2. In the Keycloak realm, add a GitHub identity provider with the client ID and secret.
3. Keep **Trust Email** off on the Keycloak broker, and keep `EDGEQUAKE_OIDC_TRUST_EMAIL=false`. GitHub emails are user-controlled and must not be used to link accounts.
4. Put users in a Keycloak Organization so the tenant claim is present, or invite them explicitly. See [Tenants, organizations and roles](tenant-and-roles.md).
