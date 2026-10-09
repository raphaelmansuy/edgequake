---
title: Authentication and SSO
description: How people and programs sign in to EdgeQuake, how to choose between password, API key and single sign-on, and how the OIDC login flow works.
---

EdgeQuake signs in people with a password or with single sign-on (SSO) through OpenID Connect (OIDC). Programs use API keys or MCP OAuth. SSO is built in (SPEC-158): Keycloak is the main route, and any OIDC issuer such as Google, Microsoft Entra or AWS Cognito works directly or through Keycloak.

## Pick a path

The chart shows which setup fits your case. Start at the top.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
  A["Who signs in?"] --> B{"Need SSO for people?"}
  B -- "No" --> C["Password login and API keys"]
  C --> C1["Set EDGEQUAKE_AUTH_ENABLED=true"]
  B -- "Yes" --> D{"Use the shipped Keycloak?"}
  D -- "Yes" --> E["make dev-sso or Helm overlay"]
  D -- "No" --> F["Set EDGEQUAKE_OIDC_* for your issuer"]
```

Read it from the top: no SSO means passwords and API keys; SSO with the shipped Keycloak means [Keycloak quickstart](keycloak-quickstart.md); SSO with another issuer means the provider pages below. For local experiments only, `EDGEQUAKE_DEV_MODE=true` turns auth off. An `oauth2-proxy` in front of the API still works as an older alternative.

## How the SSO login works

The browser talks to the EdgeQuake API, which talks to the identity provider (IdP). The IdP's token is checked once, at the callback. After that EdgeQuake issues its own session.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
sequenceDiagram
  participant B as "Browser"
  participant A as "EdgeQuake API"
  participant I as "Identity provider"
  B->>A: "GET /api/v1/auth/oidc/login?org=acme"
  A-->>B: "303 to IdP with state, nonce, PKCE"
  B->>I: "User signs in"
  I-->>B: "Redirect with code and state"
  B->>A: "GET /api/v1/auth/oidc/callback"
  A->>I: "Exchange code, fetch keys"
  A->>A: "Verify token, map tenant and role"
  A-->>B: "303 to app with a one-time code"
  B->>A: "POST /api/v1/auth/handoff"
  A-->>B: "Access token, refresh cookie"
```

Read it top to bottom: the one-time code is valid for 90 seconds and can be used once, so no token ever appears in a URL.

Design rules (see [SPEC-158](../../../specs/158-entreprise-grade-authentication/01-first-principles.md)):

1. IdP tokens are verified once, at the callback. EdgeQuake then issues its own session JWT.
2. A user is identified by the pair (issuer, subject). An email address never merges accounts unless you explicitly trust it.
3. The tenant comes from the IdP's organization claim, never from user input. An unknown organization is denied.
4. The redirect carries only a single-use code.

## Pages

| Page | For |
|------|-----|
| [Keycloak quickstart](keycloak-quickstart.md) | Run Keycloak and EdgeQuake locally, then move to production |
| [Tenants, organizations and roles](tenant-and-roles.md) | Map IdP organizations to tenants and roles |
| [Google](google.md), [Microsoft Entra](microsoft-entra.md), [AWS Cognito](aws-cognito.md), [GitHub](github.md) | Connect one IdP |
| [API keys and MCP](api-keys-and-mcp.md) | Programmatic access |
| [Production hardening](production-hardening.md) | Startup gates and checklist |
| [SSO environment reference](env-reference.md), [Troubleshooting SSO](troubleshooting.md) | Operators |

For non-SSO controls (API keys, rate limits, audit log) see the [security guide](../best-practices.md).
